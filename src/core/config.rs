use super::connection::Connection;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 应用设置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    /// 资源刷新间隔（毫秒）
    pub refresh_interval_ms: u64,
    /// 默认编辑器命令
    pub editor: String,
    /// 主题名称
    pub theme: String,
    /// 连接超时（秒）
    pub connection_timeout_secs: u64,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            refresh_interval_ms: 3000,
            editor: String::from("vim"),
            theme: String::from("default"),
            connection_timeout_secs: 10,
        }
    }
}

/// 应用配置文件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub connections: Vec<Connection>,
    pub settings: AppSettings,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            connections: Vec::new(),
            settings: AppSettings::default(),
        }
    }
}

/// 获取配置文件路径: ~/.oh-my-sftp/config.json
pub fn config_path() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join(".oh-my-sftp").join("config.json")
}

/// 加载配置
pub fn load_config() -> Result<AppConfig> {
    let path = config_path();
    if path.exists() {
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config: {:?}", path))?;
        let config: AppConfig = serde_json::from_str(&content)
            .with_context(|| "Failed to parse config JSON")?;
        Ok(config)
    } else {
        Ok(AppConfig::default())
    }
}

/// 保存配置
pub fn save_config(config: &AppConfig) -> Result<()> {
    let path = config_path();

    // 确保目录存在
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create config dir: {:?}", parent))?;
    }

    let content = serde_json::to_string_pretty(config)
        .context("Failed to serialize config")?;

    std::fs::write(&path, content)
        .with_context(|| format!("Failed to write config: {:?}", path))?;

    Ok(())
}

/// 合并 SSH config 连接和应用配置
pub fn merge_ssh_connections(config: &mut AppConfig, ssh_connections: Vec<Connection>) {
    for conn in ssh_connections {
        // 按 host+username 去重
        let is_duplicate = config
            .connections
            .iter()
            .any(|c| c.host == conn.host && c.username == conn.username);
        if !is_duplicate {
            config.connections.push(conn);
        }
    }
}
