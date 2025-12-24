use anyhow::{Context, Result};
use std::process::Command;

pub async fn perform_maintenance() -> Result<String> {
    let mut log = String::new();

    log.push_str("🔄 Updating System...\n");
    match run_command("apt-get", &["update"]).await {
        Ok(output) => log.push_str(&format!("✅ Apt Update: Success\n{}\n", output)),
        Err(e) => log.push_str(&format!("❌ Apt Update: Failed ({})\n", e)),
    }

    log.push_str("🔄 Upgrading System...\n");
    match run_command("apt-get", &["upgrade", "-y"]).await {
        Ok(output) => log.push_str(&format!("✅ Apt Upgrade: Success\n{}\n", output)),
        Err(e) => log.push_str(&format!("❌ Apt Upgrade: Failed ({})\n", e)),
    }

    Ok(log)
}

pub async fn check_security_updates() -> Result<bool> {
    let output = run_command("apt-get", &["upgrade", "-s"])
        .await
        .context("Failed to check security updates")?;
    Ok(output.contains("security"))
}

pub fn reboot_system() -> Result<()> {
    Command::new("reboot")
        .status()
        .context("Failed to reboot system")?;
    Ok(())
}

pub fn restart_service(service_name: &str) -> Result<()> {
    Command::new("systemctl")
        .args(["restart", service_name])
        .status()
        .context(format!("Failed to restart service: {}", service_name))?;
    Ok(())
}

async fn run_command(command: &str, args: &[&str]) -> Result<String> {
    let output = tokio::process::Command::new(command)
        .args(args)
        .output()
        .await
        .context(format!("Failed to execute command: {}", command))?;

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "Command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}