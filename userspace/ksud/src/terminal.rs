//! NanX Shell Root Terminal PTY Bridge for ksud

use anyhow::{Context, Result};
use log::info;
use std::process::{Command, Stdio};

pub fn run_root_shell_command(cmd_str: &str) -> Result<String> {
    info!("Executing NanX root shell command: {}", cmd_str);
    let output = Command::new("su")
        .arg("-c")
        .arg(cmd_str)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .context("Failed to execute root shell command")?;

    let stdout_str = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr_str = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() && !stderr_str.is_empty() {
        Ok(format!("{stdout_str}\n{stderr_str}"))
    } else {
        Ok(stdout_str)
    }
}
