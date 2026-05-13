# 模块详细设计

## 1. TUI 布局

```
┌─────────────────────────────────────────────────────┐
│  oh-my-sftp | v0.1.0            [连接状态] [时间]   │  ← 标题栏
├──────────┬──────────────────────────────────────────┤
│          │                                          │
│ 连接列表 │          主内容区域                        │
│          │    (终端 / 文件管理 / 资源看板)            │
│  ┌────┐  │                                          │
│  │ srv1│  │  [user@host:~]$ ls -la                   │
│  │ srv2│  │  total 48                                │
│  │ srv3│  │  drwxr-xr-x  5 user user  4096 ...      │
│  └────┘  │                                          │
│          │                                          │
│  [连接]  │                                          │
│  [断开]  │                                          │
│          │                                          │
├──────────┴──────────────────────────────────────────┤
│  > command input here...                            │  ← 命令栏
├─────────────────────────────────────────────────────┤
│  Ready | host (user) | 1 file transferring...       │  ← 状态栏
└─────────────────────────────────────────────────────┘
```

### 1.1 面板布局

采用 ratatui 的 Constraint layout：

```
Horizontal Split: 20% | 80%
  Left: ConnectionList (连接列表)
  Right: Vertical Split
    Top: MainContent (终端/文件管理/资源看板) ~85%
    Bottom: StatusBar ~1行
Bottom: CommandBar ~1-3行
```

## 2. 状态机设计

```
                    ┌──────────┐
       启动 ──────→ │ LocalTerm│  (本地终端模式)
                    └────┬─────┘
                         │ Ctrl+O
                    ┌────▼─────┐
                    │ ConnList  │  (连接列表模式)
                    └────┬─────┘
                         │ 选择服务器 + 连接
                    ┌────▼─────┐
          ┌─────────│Connected │──────────┐
          │         └────┬─────┘          │
     Ctrl+T│      Ctrl+F │        Ctrl+D  │
    ┌─────▼──┐  ┌──────▼──┐  ┌─────────▼─┐
    │Terminal│  │FileMgr  │  │ Dashboard │
    └────────┘  └─────────┘  └───────────┘
```

## 3. 事件处理

```
          ┌─────────────┐
          │  crossterm   │
          │  EventStream │
          └──────┬──────┘
                 │ KeyEvent / MouseEvent / Resize
          ┌──────▼──────┐
          │  EventLoop   │
          │  (tokio)     │
          └──────┬──────┘
                 │ dispatch
    ┌────────────┼────────────┐
    │            │            │
┌───▼──┐  ┌─────▼────┐ ┌────▼────┐
│Key   │  │Mouse     │ │Resize   │
│Handler│ │Handler   │ │Handler  │
└───┬──┘  └─────┬────┘ └────┬────┘
    │            │            │
    └────────────┼────────────┘
                 │
          ┌──────▼──────┐
          │  App.update │
          └──────┬──────┘
                 │
          ┌──────▼──────┐
          │  Render      │
          └─────────────┘
```

## 4. 连接生命周期

```
[Created] → [Connecting] → [Connected] → [Disconnecting] → [Disconnected]
                                                               │
                                                               ▼
                                                          [Created]
```

## 5. 文件传输流程

```
用户选择文件
    │
    ▼
TransferQueue.push(task)
    │
    ▼
SFTP Service 取出任务
    │
    ├─ Upload: 读本地文件 → sftp.write()
    │   └─ 更新进度 → UI 刷新
    │
    └─ Download: sftp.read() → 写本地文件
        └─ 更新进度 → UI 刷新
```

## 6. 终端模拟设计

### 6.1 本地终端

使用 `portable-pty` 创建伪终端：
- 检测用户默认 Shell（Windows: PowerShell/cmd, Unix: $SHELL）
- 将 PTY 输出映射到 ratatui Paragraph/Text
- 将键盘输入转发到 PTY

### 6.2 远程终端

使用 `ssh2::Channel` 创建 SSH 会话：
- 分配 PTY (`channel.request_pty()`)
- 启动 Shell (`channel.shell()`)
- 读写 channel stream
- 不支持 ANSI 转义序列渲染（v1 简化，后续版本可增强）

## 7. 资源采集命令

### 7.1 CPU 使用率

```bash
# Linux
top -bn1 | grep "Cpu(s)" | awk '{print $2+$4}'

# macOS
top -l 1 | grep "CPU usage" | awk '{print $3}'
```

### 7.2 内存使用

```bash
# Linux
free -m | grep Mem | awk '{print $3"/"$2}'

# macOS
vm_stat | perl -ne '/page size of (\d+)/ and $size=$1; /Pages free:\s+(\d+)/ and $free=$1; /Pages active:\s+(\d+)/ and $active=$1; END { printf "%.0f/%.0f MB\n", ($active*$size)/1048576, (($free+$active)*$size)/1048576 }'
```

### 7.3 磁盘使用

```bash
# Linux / macOS
df -h / | tail -1 | awk '{print $3"/"$2" ("$5")"}'
```

### 7.4 系统信息

```bash
uname -a
cat /etc/os-release 2>/dev/null || cat /etc/redhat-release 2>/dev/null
```

## 8. 错误处理策略

- SSH 连接失败：显示错误信息，保留在连接列表
- 传输中断：标记任务状态，支持重试
- 终端挂断：检测 channel EOF，提示用户重连
- 资源采集失败：显示 "N/A" 占位符

## 9. 安全考虑

- 密码加密存储（使用 OS keyring 或 AES 加密）
- 不在日志中输出密码
- 验证主机密钥（known_hosts）
- 私钥文件权限检查
