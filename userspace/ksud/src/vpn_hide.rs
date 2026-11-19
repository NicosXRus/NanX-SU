//! VPNHide Management Interface for ksud
//!
//! Controls kernel-level VPN interface hiding (tun0, wg0, ppp0) per app UID.

use anyhow::Result;
use log::info;
use std::fs::OpenOptions;
use std::io::Write;

pub fn add_vpn_hide_uid(uid: u32) -> Result<()> {
    if let Ok(mut f) = OpenOptions::new()
        .write(true)
        .open("/sys/module/kernelsu/parameters/vpn_hide_add_uid")
    {
        let str_val = format!("{uid}\n");
        f.write_all(str_val.as_bytes())?;
        info!("Added UID {} to VPNHide filter", uid);
    }
    Ok(())
}

pub fn remove_vpn_hide_uid(uid: u32) -> Result<()> {
    if let Ok(mut f) = OpenOptions::new()
        .write(true)
        .open("/sys/module/kernelsu/parameters/vpn_hide_remove_uid")
    {
        let str_val = format!("{uid}\n");
        f.write_all(str_val.as_bytes())?;
        info!("Removed UID {} from VPNHide filter", uid);
    }
    Ok(())
}
