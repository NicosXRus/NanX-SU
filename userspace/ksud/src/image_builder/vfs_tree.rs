//! VFS Tree Builder for Image-Overlay Container
//!
//! Scans system files in active modules and builds an in-memory VFS hierarchy
//! with support for Whiteout 0:0 (mknod 0,0) and Opaque directory attributes (`trusted.overlay.opaque`).

use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};

/// Node type in the virtual filesystem
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VfsNodeType {
    Directory {
        opaque: bool,
    },
    RegularFile {
        content: Vec<u8>,
    },
    Symlink {
        target: String,
    },
    WhiteoutDevice, // Character device 0:0 for overlayfs whiteouts
}

/// Metadata and node info
#[derive(Debug, Clone)]
pub struct VfsNode {
    pub path: PathBuf,      // Relative path within container e.g. "system/bin/app_process"
    pub node_type: VfsNodeType,
    pub mode: u32,          // File permissions
    pub uid: u32,
    pub gid: u32,
    pub mtime: u64,
    pub xattrs: BTreeMap<String, Vec<u8>>,
}

/// VFS Tree container
#[derive(Debug, Default)]
pub struct VfsTree {
    pub nodes: BTreeMap<PathBuf, VfsNode>,
}

impl VfsTree {
    pub fn new() -> Self {
        let mut tree = Self {
            nodes: BTreeMap::new(),
        };

        // Ensure root directory exists
        tree.add_node(VfsNode {
            path: PathBuf::from(""),
            node_type: VfsNodeType::Directory { opaque: false },
            mode: 0o755,
            uid: 0,
            gid: 0,
            mtime: 0,
            xattrs: BTreeMap::new(),
        });

        tree
    }

    pub fn add_node(&mut self, node: VfsNode) {
        self.nodes.insert(node.path.clone(), node);
    }

    /// Scan active modules under `/data/adb/modules` and merge system files into VFS tree
    pub fn scan_modules(modules_dir: &Path) -> Result<Self> {
        let mut tree = Self::new();

        if !modules_dir.exists() {
            return Ok(tree);
        }

        let entries = match fs::read_dir(modules_dir) {
            Ok(iter) => iter,
            Err(_) => return Ok(tree),
        };

        for entry in entries.flatten() {
            let mod_path = entry.path();
            if !mod_path.is_dir() {
                continue;
            }

            // Skip disabled modules or pending removals
            if mod_path.join("disable").exists() || mod_path.join("remove").exists() {
                continue;
            }

            // Scan system directory inside the module if present
            let sys_dir = mod_path.join("system");
            if sys_dir.is_dir() {
                tree.scan_directory_recursive(&sys_dir, Path::new("system"))?;
            }

            // Check for explicit .replace / whiteout markers in module
            let replace_dir = mod_path.join(".replace");
            if replace_dir.is_dir() {
                tree.scan_replace_markers(&replace_dir)?;
            }
        }

        Ok(tree)
    }

    fn scan_directory_recursive(&mut self, disk_path: &Path, rel_path: &Path) -> Result<()> {
        let metadata = fs::symlink_metadata(disk_path)
            .with_context(|| format!("Failed to stat {}", disk_path.display()))?;

        let mode = metadata.permissions().mode();
        let (uid, gid) = get_owner(&metadata);

        if metadata.is_dir() {
            // Check for opaque attribute file marker or xattr
            let is_opaque = disk_path.join(".replace").exists() || check_opaque_xattr(disk_path);

            let mut xattrs = BTreeMap::new();
            if is_opaque {
                xattrs.insert("trusted.overlay.opaque".to_string(), vec![b'y']);
            }

            self.add_node(VfsNode {
                path: rel_path.to_path_buf(),
                node_type: VfsNodeType::Directory { opaque: is_opaque },
                mode: (mode & 0o777) | 0o040000, // S_IFDIR
                uid,
                gid,
                mtime: get_mtime(&metadata),
                xattrs,
            });

            for entry in fs::read_dir(disk_path)?.flatten() {
                let name = entry.file_name();
                let child_disk = entry.path();
                let child_rel = rel_path.join(&name);

                // Handle whiteout marker filenames (e.g. .wh.filename)
                if let Some(name_str) = name.to_str() {
                    if name_str.starts_with(".wh.") {
                        let target_name = &name_str[4..];
                        let wh_rel = rel_path.join(target_name);
                        self.add_node(VfsNode {
                            path: wh_rel,
                            node_type: VfsNodeType::WhiteoutDevice,
                            mode: 0o020666, // S_IFCHR
                            uid: 0,
                            gid: 0,
                            mtime: get_mtime(&metadata),
                            xattrs: BTreeMap::new(),
                        });
                        continue;
                    }
                }

                self.scan_directory_recursive(&child_disk, &child_rel)?;
            }
        } else if metadata.is_symlink() {
            let target = fs::read_link(disk_path)?
                .to_string_lossy()
                .into_owned();

            self.add_node(VfsNode {
                path: rel_path.to_path_buf(),
                node_type: VfsNodeType::Symlink { target },
                mode: (mode & 0o777) | 0o120000, // S_IFLNK
                uid,
                gid,
                mtime: get_mtime(&metadata),
                xattrs: BTreeMap::new(),
            });
        } else if metadata.is_file() {
            let content = fs::read(disk_path)?;

            self.add_node(VfsNode {
                path: rel_path.to_path_buf(),
                node_type: VfsNodeType::RegularFile { content },
                mode: (mode & 0o777) | 0o100000, // S_IFREG
                uid,
                gid,
                mtime: get_mtime(&metadata),
                xattrs: BTreeMap::new(),
            });
        }

        Ok(())
    }

    fn scan_replace_markers(&mut self, replace_dir: &Path) -> Result<()> {
        // Read lines from replace_dir file if it's a file or walk directory
        if replace_dir.is_dir() {
            for entry in fs::read_dir(replace_dir)?.flatten() {
                let rel = entry.path().strip_prefix(replace_dir).unwrap().to_path_buf();
                self.add_node(VfsNode {
                    path: rel,
                    node_type: VfsNodeType::WhiteoutDevice,
                    mode: 0o020666,
                    uid: 0,
                    gid: 0,
                    mtime: 0,
                    xattrs: BTreeMap::new(),
                });
            }
        }
        Ok(())
    }
}

fn get_owner(_meta: &fs::Metadata) -> (u32, u32) {
    #[cfg(target_os = "android")]
    {
        use std::os::unix::fs::MetadataExt;
        (_meta.uid(), _meta.gid())
    }
    #[cfg(not(target_os = "android"))]
    {
        (0, 0)
    }
}

fn get_mtime(_meta: &fs::Metadata) -> u64 {
    #[cfg(target_os = "android")]
    {
        use std::os::unix::fs::MetadataExt;
        _meta.mtime() as u64
    }
    #[cfg(not(target_os = "android"))]
    {
        0
    }
}

fn check_opaque_xattr(_path: &Path) -> bool {
    #[cfg(target_os = "android")]
    {
        use std::os::unix::ffi::OsStrExt;
        let c_path = std::ffi::CString::new(_path.as_os_str().as_bytes()).ok();
        if let Some(cp) = c_path {
            let mut buf = [0u8; 16];
            let res = unsafe {
                libc::getxattr(
                    cp.as_ptr(),
                    b"trusted.overlay.opaque\0".as_ptr() as *const libc::c_char,
                    buf.as_mut_ptr() as *mut libc::c_void,
                    buf.len(),
                )
            };
            return res > 0 && buf[0] == b'y';
        }
    }
    false
}
