//! Native Image Builder for NanX-SU Container Metamodule
//!
//! Generates `.img` overlay containers in EROFS or EXT4 format with pure Rust code,
//! fully supporting Whiteout 0:0 character devices and Opaque directory extended attributes.

pub mod erofs_writer;
pub mod ext4_writer;
pub mod vfs_tree;

use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use log::info;

pub use erofs_writer::ErofsWriter;
pub use ext4_writer::Ext4Writer;
pub use vfs_tree::VfsTree;

/// Supported container image formats
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    Erofs,
    Ext4,
}

impl ImageFormat {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "ext4" => Self::Ext4,
            _ => Self::Erofs,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Erofs => "erofs",
            Self::Ext4 => "ext4",
        }
    }
}

/// Image build statistics summary
#[derive(Debug, Clone)]
pub struct BuildSummary {
    pub image_path: PathBuf,
    pub format: ImageFormat,
    pub total_nodes: usize,
    pub total_bytes: u64,
}

/// Main entry point to build or rebuild the `/data/adb/modules.img` container
pub fn build_modules_image(
    modules_dir: &Path,
    output_img: &Path,
    format: ImageFormat,
) -> Result<BuildSummary> {
    info!(
        "Building modules container image: modules_dir={}, output={}, format={}",
        modules_dir.display(),
        output_img.display(),
        format.as_str()
    );

    // 1. Scan active modules into in-memory VFS tree
    let tree = VfsTree::scan_modules(modules_dir)?;
    let total_nodes = tree.nodes.len();

    // 2. Remove old image if present
    if output_img.exists() {
        let _ = std::fs::remove_file(output_img);
    }

    // 3. Write image using selected format writer
    let total_bytes = match format {
        ImageFormat::Erofs => ErofsWriter::write_image(&tree, output_img)?,
        ImageFormat::Ext4 => Ext4Writer::write_image(&tree, output_img)?,
    };

    info!(
        "Successfully built {} image ({} nodes, {} bytes) at {}",
        format.as_str(),
        total_nodes,
        total_bytes,
        output_img.display()
    );

    Ok(BuildSummary {
        image_path: output_img.to_path_buf(),
        format,
        total_nodes,
        total_bytes,
    })
}
