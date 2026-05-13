#[allow(dead_code)]
use crate::core::config::AppSettings;
use crate::core::connection::{ActiveSession, Connection, Panel, SystemResources, TransferTask};
use crate::services::sftp_service::SftpEntry;
use std::path::PathBuf;
use std::sync::mpsc;

/// 本地终端状态
#[allow(dead_code)]
pub struct LocalTerminal {
    pub writer: Option<Box<dyn std::io::Write + Send>>,
    /// PTY 输出 channel 接收端（后台线程写入，主线程读取）
    pub pty_rx: Option<mpsc::Receiver<String>>,
    /// 保持 PTY pair 存活（同时保持 child process 存活）
    pub _pty_pair: Option<portable_pty::PtyPair>,
    /// 保持子进程存活，防止被提前终止
    pub _child: Option<Box<dyn portable_pty::Child + Send>>,
    pub output: String,
    pub scrollback: Vec<String>,
}

/// 远程终端状态
#[allow(dead_code)]
pub struct RemoteTerminal {
    pub output: String,
    pub scrollback: Vec<String>,
}

/// 文件管理面板状态
#[allow(dead_code)]
pub struct FileManagerState {
    pub entries: Vec<SftpEntry>,
    pub selected_index: usize,
    pub scroll_offset: usize,
}

/// 编辑器状态
#[allow(dead_code)]
pub struct EditorState {
    pub file_path: String,
    pub content: String,
    pub is_remote: bool,
    pub modified: bool,
}

/// 连接列表面板状态
#[allow(dead_code)]
pub struct ConnectionListState {
    pub connections: Vec<Connection>,
    pub selected_index: usize,
    pub scroll_offset: usize,
}

/// 面板状态集合
#[allow(dead_code)]
pub struct PanelsState {
    pub connection_list: ConnectionListState,
    pub file_manager: FileManagerState,
    pub editor: Option<EditorState>,
}

/// 全局应用状态
#[allow(dead_code)]
pub struct App {
    // 连接管理
    pub connections: Vec<Connection>,
    pub active_session: Option<ActiveSession>,
    pub session: Option<ssh2::Session>,

    // 面板状态
    pub active_panel: Panel,
    pub panels: PanelsState,

    // 本地终端
    pub local_terminal: Option<LocalTerminal>,

    // 远程终端
    pub remote_terminal: RemoteTerminal,

    // 文件管理
    pub local_cwd: PathBuf,
    pub remote_cwd: PathBuf,
    pub transfer_queue: Vec<TransferTask>,

    // 资源看板
    pub resources: SystemResources,

    // UI 状态
    pub should_quit: bool,
    pub command_input: String,
    pub status_message: String,
    pub show_connection_list: bool,
    pub editing_connection: bool,

    // 设置
    pub settings: AppSettings,

    // 资源刷新相关
    pub last_resource_refresh: std::time::Instant,

    // PTY 初始化状态
    pub pty_init_done: bool,
}

#[allow(dead_code)]
impl App {
    pub fn new() -> Self {
        Self {
            connections: Vec::new(),
            active_session: None,
            session: None,
            active_panel: Panel::Terminal,
            panels: PanelsState {
                connection_list: ConnectionListState {
                    connections: Vec::new(),
                    selected_index: 0,
                    scroll_offset: 0,
                },
                file_manager: FileManagerState {
                    entries: Vec::new(),
                    selected_index: 0,
                    scroll_offset: 0,
                },
                editor: None,
            },
            local_terminal: None,
            remote_terminal: RemoteTerminal {
                output: String::new(),
                scrollback: Vec::new(),
            },
            local_cwd: std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")),
            remote_cwd: PathBuf::from("/"),
            transfer_queue: Vec::new(),
            resources: SystemResources::default(),
            should_quit: false,
            command_input: String::new(),
            status_message: String::from("Ready"),
            show_connection_list: false,
            editing_connection: false,
            settings: AppSettings::default(),
            last_resource_refresh: std::time::Instant::now(),
            pty_init_done: false,
        }
    }

    /// 切换到指定面板
    pub fn switch_panel(&mut self, panel: Panel) {
        self.active_panel = panel;
    }

    /// 获取当前连接数
    pub fn connection_count(&self) -> usize {
        self.connections.len()
    }

    /// 是否有活跃连接
    pub fn is_connected(&self) -> bool {
        self.active_session.is_some()
    }

    /// 获取当前连接的主机名
    pub fn current_host(&self) -> Option<&str> {
        self.active_session.as_ref().map(|s| {
            self.connections
                .iter()
                .find(|c| c.id == s.connection_id)
                .map(|c| c.host.as_str())
                .unwrap_or("unknown")
        })
    }
}
