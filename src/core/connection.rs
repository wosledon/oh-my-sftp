use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// SSH 认证方式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthMethod {
    /// 密码认证（加密存储的 base64 编码值）
    Password(String),
    /// 私钥文件路径
    KeyFile(PathBuf),
    /// SSH Agent
    Agent,
}

/// 连接状态
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}

/// SSH 连接配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    /// 唯一标识
    pub id: String,
    /// 显示名称
    pub name: String,
    /// 主机地址
    pub host: String,
    /// SSH 端口
    pub port: u16,
    /// 用户名
    pub username: String,
    /// 认证方式
    pub auth_method: AuthMethod,
    /// 分组标签
    pub group: String,
    /// 备注
    pub note: String,
}

/// 传输方向
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransferDirection {
    Upload,
    Download,
}

/// 传输任务状态
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransferStatus {
    Pending,
    InProgress,
    Completed,
    Failed(String),
}

/// 文件传输任务
#[derive(Debug, Clone)]
pub struct TransferTask {
    pub id: String,
    pub connection_id: String,
    pub direction: TransferDirection,
    pub local_path: PathBuf,
    pub remote_path: PathBuf,
    pub total_bytes: u64,
    pub transferred_bytes: u64,
    pub status: TransferStatus,
}

/// 远程系统资源信息
#[derive(Debug, Clone, Default)]
pub struct SystemResources {
    pub cpu_usage: f64,
    pub memory_used_mb: u64,
    pub memory_total_mb: u64,
    pub disk_used_gb: f64,
    pub disk_total_gb: f64,
    pub load_average: [f64; 3],
    pub uptime: String,
}

/// 活跃的 SSH 会话
pub struct ActiveSession {
    pub connection_id: String,
    pub session: ssh2::Session,
    pub status: ConnectionStatus,
}

/// 应用面板
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Panel {
    Terminal,
    FileManager,
    ResourceDashboard,
    ConnectionList,
}

impl Connection {
    pub fn new(name: &str, host: &str, port: u16, username: &str, auth: AuthMethod) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            host: host.to_string(),
            port,
            username: username.to_string(),
            auth_method: auth,
            group: String::new(),
            note: String::new(),
        }
    }

    /// 从 ~/.ssh/config 的 Host 条目创建
    pub fn from_ssh_config_entry(entry: SshConfigEntry) -> Self {
        let auth = if let Some(key_path) = entry.identity_file {
            AuthMethod::KeyFile(key_path)
        } else {
            AuthMethod::Agent
        };

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: entry.host_alias.clone(),
            host: entry.hostname.unwrap_or_else(|| entry.host_alias.clone()),
            port: entry.port.unwrap_or(22),
            username: entry.user.unwrap_or_else(|| whoami::username()),
            auth_method: auth,
            group: String::new(),
            note: format!("Imported from ~/.ssh/config: {}", entry.host_alias),
        }
    }
}

impl TransferTask {
    pub fn progress_percent(&self) -> f64 {
        if self.total_bytes == 0 {
            return 0.0;
        }
        (self.transferred_bytes as f64 / self.total_bytes as f64) * 100.0
    }
}

/// ~/.ssh/config 解析后的 Host 条目
#[derive(Debug, Clone, Default)]
pub struct SshConfigEntry {
    pub host_alias: String,
    pub hostname: Option<String>,
    pub port: Option<u16>,
    pub user: Option<String>,
    pub identity_file: Option<PathBuf>,
    pub proxy_jump: Option<String>,
    pub forward_agent: bool,
}
