# oh-my-sftp 架构设计文档

## 1. 项目概述

oh-my-sftp 是一个基于 Rust + TUI 的 SSH/SFTP 全能终端工具，提供连接管理、文件传输、远程命令执行、文件编辑和服务器资源监控等功能。

## 2. 技术选型

| 组件       | 选型                | 说明                                    |
| ---------- | ------------------- | --------------------------------------- |
| TUI 框架   | ratatui + crossterm | Rust 生态最成熟的 TUI 框架              |
| SSH/SFTP   | ssh2                | libssh2 的 Rust 绑定，支持 SSH/SFTP/SCP |
| 异步运行时 | tokio               | 用于并发处理 SSH 会话和 UI 事件         |
| 序列化     | serde + serde_json  | 连接配置持久化                          |
| 日志       | log + env_logger    | 调试日志                                |
| 文本编辑   | tui-textarea        | TUI 文本编辑器组件                      |

## 3. 系统架构

```
┌────────────────────────────────────────────────────┐
│                    TUI Layer                        │
│  ┌──────────┐ ┌──────────┐ ┌───────────────────┐  │
│  │ App State│ │ Event Loop│ │  Component Tree   │  │
│  └──────────┘ └──────────┘ └───────────────────┘  │
├────────────────────────────────────────────────────┤
│                  Service Layer                      │
│  ┌──────────┐ ┌──────────┐ ┌───────────────────┐  │
│  │SSH Service│ │SFTP Service│ │Resource Monitor  │  │
│  └──────────┘ └──────────┘ └───────────────────┘  │
├────────────────────────────────────────────────────┤
│                  Core Layer                         │
│  ┌──────────┐ ┌──────────┐ ┌───────────────────┐  │
│  │Connection │ │ Config   │ │   SSH Config      │  │
│  │ Manager   │ │ Manager  │ │   Parser          │  │
│  └──────────┘ └──────────┘ └───────────────────┘  │
└────────────────────────────────────────────────────┘
```

## 4. 模块划分

```
src/
├── main.rs              # 程序入口
├── app.rs               # 应用状态管理
├── event.rs             # 事件处理循环
├── tui.rs               # TUI 渲染入口
├── components/          # UI 组件
│   ├── mod.rs
│   ├── connection_list.rs   # 连接列表面板
│   ├── terminal.rs          # 终端面板（本地/远程）
│   ├── file_manager.rs      # 文件管理面板（上传/下载）
│   ├── resource_dashboard.rs # 资源看板（CPU/内存/磁盘）
│   ├── editor.rs            # 文件编辑器
│   ├── command_bar.rs       # 命令输入栏
│   └── status_bar.rs        # 状态栏
├── services/            # 业务服务
│   ├── mod.rs
│   ├── ssh_service.rs       # SSH 连接管理
│   ├── sftp_service.rs      # SFTP 文件传输
│   └── resource_service.rs  # 远程资源采集
├── core/                # 核心模块
│   ├── mod.rs
│   ├── connection.rs        # 连接数据结构
│   ├── config.rs            # 应用配置
│   └── ssh_config.rs        # ~/.ssh/config 解析器
└── utils/               # 工具函数
    ├── mod.rs
    └── path.rs               # 路径处理
```

## 5. 核心数据结构

### 5.1 连接配置 (Connection)

```rust
struct Connection {
    id: String,           // 唯一标识
    name: String,         // 显示名称
    host: String,         // 主机地址
    port: u16,            // SSH 端口
    username: String,     // 用户名
    auth_method: AuthMethod, // 认证方式
    group: String,        // 分组
}

enum AuthMethod {
    Password(String),     // 密码（加密存储）
    KeyFile(PathBuf),     // 私钥文件路径
    Agent,                // SSH Agent
}
```

### 5.2 应用状态 (AppState)

```rust
struct App {
    // 连接管理
    connections: Vec<Connection>,
    active_connection: Option<ActiveSession>,
    
    // 面板状态
    active_panel: Panel,
    panels: PanelState,
    
    // 本地终端
    local_terminal: LocalTerminal,
    
    // 文件管理
    local_cwd: PathBuf,
    remote_cwd: PathBuf,
    transfer_queue: Vec<TransferTask>,
    
    // UI 状态
    should_quit: bool,
    command_input: String,
    status_message: String,
}

enum Panel {
    Terminal,           // 终端面板
    FileManager,        // 文件管理
    ResourceDashboard,  // 资源看板
    ConnectionList,     // 连接列表
}
```

## 6. 用户交互流程

### 6.1 启动流程

```
启动 → 加载 ~/.ssh/config → 加载本地配置 → 打开本地终端
```

### 6.2 连接流程

```
连接列表 → 选择服务器 → 认证 → 成功后标记为已连接
                                      ↓
                            Terminal / FileManager / Dashboard
```

### 6.3 快捷键设计

| 快捷键   | 功能           | 作用域   |
| -------- | -------------- | -------- |
| `Ctrl+C` | 退出程序       | 全局     |
| `Ctrl+O` | 打开连接列表   | 全局     |
| `Ctrl+T` | 切换到终端     | 全局     |
| `Ctrl+F` | 切换到文件管理 | 全局     |
| `Ctrl+D` | 切换到资源看板 | 全局     |
| `Ctrl+E` | 编辑远程文件   | 文件管理 |
| `Ctrl+U` | 上传文件       | 文件管理 |
| `Ctrl+G` | 下载文件       | 文件管理 |
| `Ctrl+R` | 重新连接       | 终端     |
| `Tab`    | 切换面板       | 全局     |
| `Esc`    | 返回上级/取消  | 全局     |

## 7. SSH 配置解析

### 7.1 支持的 SSH Config 指令

- `Host` / `HostName` / `Port` / `User`
- `IdentityFile` / `Password`
- `ProxyJump` / `ProxyCommand`
- `ForwardAgent`
- Include 指令

### 7.2 解析流程

```
~/.ssh/config → 词法分析 → 语法分析 → Connection 列表
                                              ↓
                                    合并到本地配置
```

## 8. 本地终端实现

使用伪终端 (PTY) 实现本地终端：
- Windows: 使用 `winpty` 或 ConPTY API
- Unix: 使用 `forkpty`

启动时自动打开本地终端（如 PowerShell / bash），支持：
- 完整的终端交互
- 复制粘贴
- 窗口大小自适应

## 9. 文件管理设计

### 9.1 文件传输

使用 SFTP 协议进行文件传输，支持：
- 断点续传（记录 offset）
- 多文件批量传输
- 传输进度显示
- 传输队列管理

### 9.2 远程文件编辑

```
远程文件 → SFTP 下载到临时目录 → 本地编辑器打开
     ← 保存后 SFTP 上传回远程 ← 
```

## 10. 资源看板设计

通过 SSH 远程执行命令采集系统资源：
- **CPU**: `top -bn1` / `/proc/stat` / `wmic cpu`
- **内存**: `free -m` / `/proc/meminfo`
- **磁盘**: `df -h`
- **网络**: `cat /proc/net/dev`
- **系统信息**: `uname -a`, `cat /etc/os-release`

使用定时刷新（默认 3 秒），通过 ratatui 的 gauge/sparkline 组件展示。

## 11. 配置持久化

应用配置保存在 `~/.oh-my-sftp/config.json`：
```json
{
  "connections": [...],
  "settings": {
    "refresh_interval": 3000,
    "editor": "vim",
    "theme": "default"
  }
}
```
