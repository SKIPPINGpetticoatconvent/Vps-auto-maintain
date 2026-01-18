use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::fs;
use std::path::Path;

/// GitHub Release 信息
#[derive(Debug, Deserialize)]
pub struct GitHubRelease {
    pub tag_name: String,
    #[allow(dead_code)]
    pub name: Option<String>,
    pub body: Option<String>,
    pub assets: Vec<GitHubAsset>,
}

/// GitHub Release Asset 信息
#[derive(Debug, Deserialize)]
pub struct GitHubAsset {
    pub name: String,
    pub browser_download_url: String,
    #[allow(dead_code)]
    pub size: u64,
}

/// 更新状态
#[derive(Debug)]
pub enum UpdateStatus {
    /// 已是最新版本
    UpToDate,
    /// 有新版本可用
    UpdateAvailable {
        current: String,
        latest: String,
        release_notes: Option<String>,
    },
    /// 无法确定版本
    #[allow(dead_code)]
    Unknown(String),
}

/// GitHub 仓库配置
const REPOS: &[&str] = &[
    "FTDRTD/Vps-auto-maintain",
    "SKIPPINGpetticoatconvent/Vps-auto-maintain",
];

/// 二进制文件名
const BINARY_NAME: &str = "vps-tg-bot-rust-linux-amd64";

/// 安装路径
const INSTALL_PATH: &str = "/usr/local/bin/vps-tg-bot-rust";

/// 服务名称
const SERVICE_NAME: &str = "vps-tg-bot-rust";

/// 获取当前版本
pub fn get_current_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// 检查最新版本
pub async fn check_latest_version() -> Result<UpdateStatus> {
    let client = reqwest::Client::builder()
        .user_agent("vps-tg-bot-rust")
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    for repo in REPOS {
        let api_url = format!("https://api.github.com/repos/{}/releases/latest", repo);
        
        match client.get(&api_url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    if let Ok(release) = response.json::<GitHubRelease>().await {
                        let current = get_current_version();
                        let latest = release.tag_name.trim_start_matches('v').to_string();
                        
                        if compare_versions(&current, &latest) >= 0 {
                            return Ok(UpdateStatus::UpToDate);
                        } else {
                            return Ok(UpdateStatus::UpdateAvailable {
                                current,
                                latest,
                                release_notes: release.body,
                            });
                        }
                    }
                }
            }
            Err(e) => {
                log::warn!("从 {} 获取版本失败: {}", repo, e);
                continue;
            }
        }
    }

    Err(anyhow!("无法从任何仓库获取最新版本信息"))
}

/// 获取最新版本的下载信息
pub async fn get_latest_release() -> Result<(String, String)> {
    let client = reqwest::Client::builder()
        .user_agent("vps-tg-bot-rust")
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    for repo in REPOS {
        let api_url = format!("https://api.github.com/repos/{}/releases/latest", repo);
        
        match client.get(&api_url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    if let Ok(release) = response.json::<GitHubRelease>().await {
                        // 查找对应的二进制文件
                        for asset in &release.assets {
                            if asset.name.contains("linux-amd64") || asset.name == BINARY_NAME {
                                return Ok((
                                    release.tag_name.clone(),
                                    asset.browser_download_url.clone(),
                                ));
                            }
                        }
                        
                        // 如果没有 assets，构建默认下载 URL
                        let download_url = format!(
                            "https://github.com/{}/releases/download/{}/{}",
                            repo, release.tag_name, BINARY_NAME
                        );
                        return Ok((release.tag_name, download_url));
                    }
                }
            }
            Err(e) => {
                log::warn!("从 {} 获取 release 失败: {}", repo, e);
                continue;
            }
        }
    }

    Err(anyhow!("无法从任何仓库获取最新版本下载信息"))
}

/// 下载更新
pub async fn download_update(download_url: &str) -> Result<Vec<u8>> {
    log::info!("开始下载更新: {}", download_url);
    
    let client = reqwest::Client::builder()
        .user_agent("vps-tg-bot-rust")
        .timeout(std::time::Duration::from_secs(300)) // 5 分钟超时
        .build()?;

    let response = client.get(download_url).send().await?;
    
    if !response.status().is_success() {
        return Err(anyhow!("下载失败: HTTP {}", response.status()));
    }

    let bytes = response.bytes().await?;
    log::info!("下载完成，大小: {} 字节", bytes.len());
    
    Ok(bytes.to_vec())
}

/// 应用更新
pub async fn apply_update(binary_data: &[u8]) -> Result<()> {
    let install_path = Path::new(INSTALL_PATH);
    let backup_path = format!("{}.bak", INSTALL_PATH);
    let temp_path = format!("{}.new", INSTALL_PATH);

    // 1. 写入临时文件
    log::info!("写入临时文件: {}", temp_path);
    fs::write(&temp_path, binary_data)?;
    
    // 2. 设置执行权限
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let permissions = fs::Permissions::from_mode(0o755);
        fs::set_permissions(&temp_path, permissions)?;
    }

    // 3. 备份当前二进制
    if install_path.exists() {
        log::info!("备份当前二进制: {} -> {}", INSTALL_PATH, backup_path);
        fs::copy(install_path, &backup_path)?;
    }

    // 4. 替换二进制
    log::info!("替换二进制文件");
    fs::rename(&temp_path, install_path)?;

    Ok(())
}

/// 触发服务重启
pub async fn restart_service() -> Result<()> {
    log::info!("触发服务重启: {}", SERVICE_NAME);
    
    let output = tokio::process::Command::new("systemctl")
        .args(["restart", SERVICE_NAME])
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("重启服务失败: {}", stderr));
    }

    Ok(())
}

/// 执行完整更新流程
pub async fn perform_update() -> Result<String> {
    // 1. 获取最新版本信息
    let (version, download_url) = get_latest_release().await?;
    log::info!("准备更新到版本: {}", version);

    // 2. 下载二进制
    let binary_data = download_update(&download_url).await?;

    // 3. 应用更新
    apply_update(&binary_data).await?;

    Ok(version)
}

/// 比较版本号
/// 返回: -1 (a < b), 0 (a == b), 1 (a > b)
pub fn compare_versions(a: &str, b: &str) -> i32 {
    let parse_version = |v: &str| -> Vec<u32> {
        v.trim_start_matches('v')
            .split('.')
            .filter_map(|s| s.parse().ok())
            .collect()
    };

    let va = parse_version(a);
    let vb = parse_version(b);

    for i in 0..std::cmp::max(va.len(), vb.len()) {
        let na = va.get(i).copied().unwrap_or(0);
        let nb = vb.get(i).copied().unwrap_or(0);
        
        if na < nb {
            return -1;
        } else if na > nb {
            return 1;
        }
    }
    
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compare_versions() {
        assert_eq!(compare_versions("0.1.0", "0.1.0"), 0);
        assert_eq!(compare_versions("0.1.0", "0.2.0"), -1);
        assert_eq!(compare_versions("0.2.0", "0.1.0"), 1);
        assert_eq!(compare_versions("1.0.0", "0.9.9"), 1);
        assert_eq!(compare_versions("v0.1.0", "0.1.0"), 0);
        assert_eq!(compare_versions("0.1", "0.1.0"), 0);
        assert_eq!(compare_versions("0.1.0", "0.1.1"), -1);
    }

    #[test]
    fn test_get_current_version() {
        let version = get_current_version();
        assert!(!version.is_empty());
        // 当前版本应该是 0.1.0
        assert_eq!(version, "0.1.0");
    }
}
