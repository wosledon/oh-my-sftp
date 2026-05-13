# oh-my-sftp

一个基于 Rust + TUI 的 SSH/SFTP 全能终端工具 — 连接管理、文件传输、远程命令、资源监控，一个工具全部搞定。

## 功能

- 🔌 **SSH 连接管理** — 自动加载 `~/.ssh/config`，支持密码/私钥/SSH Agent 三种认证
- 🖥️ **本地 + 远程终端** — 启动即打开本地终端，一键连接远程服务器
- 📁 **SFTP 文件管理** — 双栏浏览、上传下载、远程文件编辑
- 📊 **资源看板** — 实时查看远程 CPU/内存/磁盘/负载
- ⌨️ **全键盘操作** — 所有操作都有快捷键，无需鼠标

## 安装

```bash
# 从源码编译
git clone https://github.com/yourname/oh-my-sftp.git
cd oh-my-sftp
cargo build --release

# 二进制文件在 target/release/oh-my-sftp.exe (Windows) 或 oh-my-sftp (Unix)
```

## 快速开始

```bash
cargo run
```

启动后进入 TUI 界面，默认显示本地终端面板。如果本地 PTY 不可用，会显示帮助快捷键面板。

## 快捷键

| 快捷键 | 功能 |
|--------|------|
| `Ctrl+O` | 打开/关闭连接列表面板 |
| `Ctrl+T` | 切换到终端面板 |
| `Ctrl+F` | 切换到文件管理面板（需先连接） |
| `Ctrl+D` | 切换到资源看板（需先连接） |
| `Tab` | 轮转切换面板 |
| `Esc` | 返回终端主面板 |
| `Ctrl+C` / `Ctrl+Q` | 退出程序 |

### 连接列表

| 按键 | 功能 |
|------|------|
| `↑` `↓` / `j` `k` | 上下移动选择 |
| `Enter` | 连接选中服务器 |
| `d` | 断开当前连接 |

### 文件管理

| 按键 | 功能 |
|------|------|
| `↑` `↓` / `j` `k` | 上下移动选择 |
| `Enter` | 进入目录 / 编辑文件 |
| `Backspace` | 返回上级目录 |

## 配置

### SSH 配置

程序启动时自动加载 `~/.ssh/config`，无需额外配置。支持的指令：

- `Host` / `HostName` / `Port` / `User`
- `IdentityFile`（私钥路径）
- `ForwardAgent`
- `ProxyJump`
- `Include`（支持 glob 模式）

### 应用配置

应用配置保存在 `~/.oh-my-sftp/config.json`：

```json
{
  "connections": [],
  "settings": {
    "refresh_interval_ms": 3000,
    "connection_timeout_secs": 10,
    "editor": "vim",
    "theme": "default"
  }
}
```

| 设置项 | 默认值 | 说明 |
|--------|--------|------|
| `refresh_interval_ms` | 3000 | 资源看板刷新间隔（毫秒） |
| `connection_timeout_secs` | 10 | SSH 连接超时（秒） |
| `editor` | vim | 远程文件编辑器命令 |
| `theme` | default | 主题名称（保留） |

## 界面布局

```
┌─────────────────────────────────────────────────────┐
│  oh-my-sftp v0.1.0                    [Terminal]    │  ← 标题栏
├──────────┬──────────────────────────────────────────┤
│          │  ╔═══════════════════════════════════╗    │
│ 连接列表 │  ║    oh-my-sftp — Terminal Mode    ║    │
│          │  ╠═══════════════════════════════════╣    │
│  ┌────┐  │  ║  Ctrl+O  连接列表                ║    │
│  │srv1│  │  ║  Ctrl+T  终端                    ║    │
│  │srv2│  │  ║  Ctrl+F  文件管理                ║    │
│  └────┘  │  ║  Ctrl+D  资源看板                ║    │
│          │  ╚═══════════════════════════════════╝    │
├──────────┴──────────────────────────────────────────┤
│  > _                                                │  ← 命令输入
├─────────────────────────────────────────────────────┤
│  Ready | 2 connections | 0 transfers         21:30  │  ← 状态栏
└─────────────────────────────────────────────────────┘
```

## 认证方式

程序支持三种 SSH 认证，优先级与 `ssh` 命令一致：

1. **SSH Agent** — 如果 `~/.ssh/config` 未指定 `IdentityFile`，默认尝试 Agent
2. **私钥文件** — 通过 `IdentityFile` 指定路径，支持 `~` 展开
3. **密码** — 通过应用配置手动添加（base64 编码存储）

## 依赖

- Rust 1.70+
- Windows: 需要 Windows 10 1809+（ConPTY 支持）
- Linux/macOS: 无额外依赖

## 技术栈

| 组件 | 选型 |
|------|------|
| TUI | ratatui + crossterm |
| SSH/SFTP | ssh2 (libssh2) |
| PTY | portable-pty |
| 异步 | tokio |
| 序列化 | serde + serde_json |

## 开发

```bash
# 编译
cargo build

# 运行
cargo run

# 测试
cargo test

# 代码检查
cargo clippy
```

## 设计文档

- [架构设计](docs/ARCHITECTURE.md)
- [模块详细设计](docs/MODULE_DESIGN.md)

## License

MIT
