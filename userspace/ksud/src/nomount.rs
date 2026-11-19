//! NoMount Management and Kernel Interop for ksud
//!
//! Provides control interfaces for LKM VFS path redirection rules
//! and container image triggers.

use anyhow::{Context, Result};
use log::{info, warn};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::image_builder::{self, ImageFormat};

pub const MODULES_IMAGE_PATH: &str = "/data/adb/modules.img";

/// Check if NoMount kernel extension is supported
pub fn is_nomount_supported() -> bool {
    Path::new("/sys/module/kernelsu/parameters/nomount_enabled").exists()
        || Path::new("/proc/nanx_nomount").exists()
}

/// Enable or disable NoMount mode in the kernel
pub fn set_nomount_enabled(enabled: bool) -> Result<()> {
    if let Ok(mut f) = OpenOptions::new()
        .write(true)
        .open("/sys/module/kernelsu/parameters/nomount_enabled")
    {
        let val = if enabled { "1" } else { "0" };
        f.write_all(val.as_bytes())?;
        info!("NoMount mode set to {}", enabled);
        return Ok(());
    }
    warn!("Kernel does not support NoMount parameter toggle");
    Ok(())
}

/// Trigger native Rust `image_builder` to rebuild `/data/adb/modules.img`
pub fn rebuild_container_image(format: ImageFormat) -> Result<PathBuf> {
    let modules_dir = Path::new(crate::defs::MODULE_DIR);
    let output_img = Path::new(MODULES_IMAGE_PATH);

    let summary = image_builder::build_modules_image(modules_dir, output_img, format)
        .context("Failed to build modules image container")?;

    info!(
        "Rebuilt container image: format={}, nodes={}, size={}",
        summary.format.as_str(),
        summary.total_nodes,
        summary.total_bytes
    );

    Ok(summary.image_path)
}
