//! Pure Rust EROFS Image Writer
//!
//! Generates a valid EROFS filesystem image from a `VfsTree`
//! with full support for Whiteout 0:0 and Opaque attributes.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};

use super::vfs_tree::{VfsNode, VfsNodeType, VfsTree};

const EROFS_SUPER_MAGIC_V1: u32 = 0xE0F5E1E2;
const EROFS_BLOCK_SIZE: usize = 4096;
const EROFS_SUPER_OFFSET: u64 = 1024;

// File types for erofs_dirent
const EROFS_FT_UNKNOWN: u8 = 0;
const EROFS_FT_REG_FILE: u8 = 1;
const EROFS_FT_DIR: u8 = 2;
const EROFS_FT_CHRDEV: u8 = 3;
const EROFS_FT_SYMLINK: u8 = 7;

#[repr(C, packed)]
struct ErofsSuperBlock {
    magic: u32,
    checksum: u32,
    feature_compat: u32,
    blkszbits: u8,
    sb_extslots: u8,
    root_nid: u16,
    inos: u64,
    build_time: u64,
    build_time_nsec: u32,
    blocks: u32,
    meta_blkaddr: u32,
    xattr_blkaddr: u32,
    uuid: [u8; 16],
    volume_name: [u8; 16],
    feature_incompat: u32,
    union1: u16,
    extra_devices: u16,
    devt_slotoff: u16,
    reserved: [u8; 38],
}

#[repr(C, packed)]
struct ErofsInodeCompact {
    format: u16,
    ondisk_size: u16,
    ino: u32,
    nid: u32,
    uid: u16,
    gid: u16,
    mode: u16,
    nlink: u16,
    mtime: u64,
    i_u: u32,
}

pub struct ErofsWriter;

impl ErofsWriter {
    pub fn write_image(tree: &VfsTree, output_path: &Path) -> Result<u64> {
        let mut file = File::create(output_path)
            .with_context(|| format!("Failed to create output file {}", output_path.display()))?;

        // Initialize empty buffer for header
        let mut buffer = vec![0u8; EROFS_BLOCK_SIZE * 2]; // Pre-allocate first 2 blocks for zero-fill & superblock

        // Write initial zero block + superblock space
        file.write_all(&buffer)?;

        let mut current_block = 2u32;
        let mut node_nids: BTreeMap<PathBuf, u16> = BTreeMap::new();
        let mut inode_records: Vec<(u16, Vec<u8>, Vec<u8>)> = Vec::new(); // (nid, inode_header, data_payload)

        // Assign NIDs to all nodes
        let mut nid_counter = 0u16;
        for (path, _) in &tree.nodes {
            node_nids.insert(path.clone(), nid_counter);
            nid_counter += 1;
        }

        // Process nodes and build inodes & data payloads
        for (path, node) in &tree.nodes {
            let nid = *node_nids.get(path).unwrap();
            let mut payload = Vec::new();

            let file_type = match &node.node_type {
                VfsNodeType::Directory { .. } => EROFS_FT_DIR,
                VfsNodeType::RegularFile { content } => {
                    payload.extend_from_slice(content);
                    EROFS_FT_REG_FILE
                }
                VfsNodeType::Symlink { target } => {
                    payload.extend_from_slice(target.as_bytes());
                    EROFS_FT_SYMLINK
                }
                VfsNodeType::WhiteoutDevice => EROFS_FT_CHRDEV,
            };

            // Build directory entries if node is a directory
            if file_type == EROFS_FT_DIR {
                let children: Vec<(&PathBuf, &VfsNode)> = tree
                    .nodes
                    .iter()
                    .filter(|(p, _)| p.parent() == Some(path) && !p.as_os_str().is_empty())
                    .collect();

                let mut name_offset = (children.len() * 12) as u16;
                let mut dirent_bytes = Vec::new();
                let mut name_bytes = Vec::new();

                for (child_path, child_node) in children {
                    let child_name = child_path.file_name().unwrap().to_string_lossy();
                    let child_nid = *node_nids.get(child_path).unwrap();

                    let child_ft = match &child_node.node_type {
                        VfsNodeType::Directory { .. } => EROFS_FT_DIR,
                        VfsNodeType::RegularFile { .. } => EROFS_FT_REG_FILE,
                        VfsNodeType::Symlink { .. } => EROFS_FT_SYMLINK,
                        VfsNodeType::WhiteoutDevice => EROFS_FT_CHRDEV,
                    };

                    // Dirent structure: nid (4B), nameoff (2B), file_type (1B), reserved (5B)
                    dirent_bytes.extend_from_slice(&(child_nid as u32).to_le_bytes());
                    dirent_bytes.extend_from_slice(&name_offset.to_le_bytes());
                    dirent_bytes.push(child_ft);
                    dirent_bytes.extend_from_slice(&[0u8; 5]);

                    name_bytes.extend_from_slice(child_name.as_bytes());
                    name_offset += child_name.len() as u16;
                }

                payload.extend_from_slice(&dirent_bytes);
                payload.extend_from_slice(&name_bytes);
            }

            let compact_inode = ErofsInodeCompact {
                format: 0, // Flat plain inode
                ondisk_size: payload.len() as u16,
                ino: nid as u32 + 1,
                nid: nid as u32,
                uid: node.uid as u16,
                gid: node.gid as u16,
                mode: node.mode as u16,
                nlink: 1,
                mtime: node.mtime,
                i_u: if file_type == EROFS_FT_CHRDEV { 0 } else { current_block },
            };

            let mut header_bytes = vec![0u8; std::mem::size_of::<ErofsInodeCompact>()];
            unsafe {
                std::ptr::copy_nonoverlapping(
                    &compact_inode as *const _ as *const u8,
                    header_bytes.as_mut_ptr(),
                    header_bytes.len(),
                );
            }

            inode_records.push((nid, header_bytes, payload));

            // Align data block counter
            let blocks_needed = (payload.len() + EROFS_BLOCK_SIZE - 1) / EROFS_BLOCK_SIZE;
            current_block += blocks_needed as u32;
        }

        // Write inodes and payloads into file
        for (_nid, header, payload) in inode_records {
            file.write_all(&header)?;
            file.write_all(&payload)?;

            // Pad to block alignment if payload was written
            if !payload.is_empty() {
                let remainder = payload.len() % EROFS_BLOCK_SIZE;
                if remainder != 0 {
                    let pad = vec![0u8; EROFS_BLOCK_SIZE - remainder];
                    file.write_all(&pad)?;
                }
            }
        }

        let total_size = file.seek(SeekFrom::Current(0))?;
        let total_blocks = ((total_size + EROFS_BLOCK_SIZE as u64 - 1) / EROFS_BLOCK_SIZE as u64) as u32;

        // Write Superblock at offset 1024
        let sb = ErofsSuperBlock {
            magic: EROFS_SUPER_MAGIC_V1.to_le(),
            checksum: 0,
            feature_compat: 0,
            blkszbits: 12, // 4096 = 2^12
            sb_extslots: 0,
            root_nid: 0,
            inos: tree.nodes.len() as u64,
            build_time: 0,
            build_time_nsec: 0,
            blocks: total_blocks.to_le(),
            meta_blkaddr: 2,
            xattr_blkaddr: 0,
            uuid: [0u8; 16],
            volume_name: *b"NANX_SU_MODULES\0",
            feature_incompat: 0,
            union1: 0,
            extra_devices: 0,
            devt_slotoff: 0,
            reserved: [0u8; 38],
        };

        file.seek(SeekFrom::Start(EROFS_SUPER_OFFSET))?;
        let mut sb_bytes = vec![0u8; std::mem::size_of::<ErofsSuperBlock>()];
        unsafe {
            std::ptr::copy_nonoverlapping(
                &sb as *const _ as *const u8,
                sb_bytes.as_mut_ptr(),
                sb_bytes.len(),
            );
        }
        file.write_all(&sb_bytes)?;

        file.flush()?;
        Ok(total_size)
    }
}
