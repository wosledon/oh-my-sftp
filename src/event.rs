use crate::app::App;
use crate::core::connection::Panel;
use anyhow::Context;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

/// 事件处理器
pub struct EventHandler;

/// 运行事件循环时的结果
pub enum EventResult {
    Continue,
    Quit,
}

impl EventHandler {
    /// 处理键盘事件，返回是否应该继续运行
    pub fn handle_key(app: &mut App, key: KeyEvent) -> EventResult {
        // 只处理按键按下事件
        if key.kind != KeyEventKind::Press {
            return EventResult::Continue;
        }

        // 全局快捷键
        match key.code {
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.should_quit = true;
                return EventResult::Quit;
            }
            KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.should_quit = true;
                return EventResult::Quit;
            }
            _ => {}
        }

        // 如果在编辑模式，只处理编辑相关按键
        if app.editing_connection {
            return Self::handle_editing(app, key);
        }

        // 面板切换快捷键
        if !key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Tab => {
                    Self::cycle_panel(app);
                    return EventResult::Continue;
                }
                KeyCode::Esc => {
                    // 返回主面板
                    if app.show_connection_list {
                        app.show_connection_list = false;
                    }
                    app.active_panel = Panel::Terminal;
                    return EventResult::Continue;
                }
                _ => {}
            }
        }

        // Ctrl+Key 面板切换
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('o') => {
                    app.show_connection_list = !app.show_connection_list;
                    return EventResult::Continue;
                }
                KeyCode::Char('t') => {
                    app.active_panel = Panel::Terminal;
                    app.show_connection_list = false;
                    return EventResult::Continue;
                }
                KeyCode::Char('f') => {
                    if app.is_connected() {
                        app.active_panel = Panel::FileManager;
                        app.show_connection_list = false;
                        // 自动加载文件列表
                        if app.panels.file_manager.entries.is_empty() {
                            if let Some(ref active) = app.active_session {
                                if let Ok(entries) =
                                    refresh_remote_files(&active.session, &app.remote_cwd)
                                {
                                    app.panels.file_manager.entries = entries;
                                    app.panels.file_manager.selected_index = 0;
                                }
                            }
                        }
                    }
                    return EventResult::Continue;
                }
                KeyCode::Char('d') => {
                    if app.is_connected() {
                        app.active_panel = Panel::ResourceDashboard;
                        app.show_connection_list = false;
                    }
                    return EventResult::Continue;
                }
                _ => {}
            }
        }

        // 根据当前面板分发事件
        match app.active_panel {
            Panel::ConnectionList => Self::handle_connection_list(app, key),
            Panel::Terminal => Self::handle_terminal(app, key),
            Panel::FileManager => Self::handle_file_manager(app, key),
            Panel::ResourceDashboard => Self::handle_resource_dashboard(app, key),
        }

        EventResult::Continue
    }

    fn cycle_panel(app: &mut App) {
        let panels = if app.is_connected() {
            vec![
                Panel::Terminal,
                Panel::FileManager,
                Panel::ResourceDashboard,
            ]
        } else {
            vec![Panel::Terminal]
        };

        let current_idx = panels
            .iter()
            .position(|p| *p == app.active_panel)
            .unwrap_or(0);
        let next_idx = (current_idx + 1) % panels.len();
        app.active_panel = panels[next_idx].clone();
    }

    fn handle_connection_list(app: &mut App, key: KeyEvent) {
        let list = &mut app.panels.connection_list;
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if list.selected_index > 0 {
                    list.selected_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if list.selected_index < list.connections.len().saturating_sub(1) {
                    list.selected_index += 1;
                }
            }
            KeyCode::Enter => {
                // 连接选中的服务器
                let idx = list.selected_index;
                if idx < list.connections.len() {
                    let conn = &list.connections[idx];
                    app.status_message = format!("Connecting to {}...", conn.name);

                    // 尝试建立 SSH 连接
                    match crate::services::ssh_service::SshService::connect(conn, 10) {
                        Ok((session, _tcp)) => {
                            app.active_session = Some(crate::core::connection::ActiveSession {
                                connection_id: conn.id.clone(),
                                session,
                                status: crate::core::connection::ConnectionStatus::Connected,
                            });
                            app.status_message = format!("Connected to {}", conn.name);
                            app.remote_terminal.output = format!(
                                "Connected to {}@{}:{}\n\
                                Type 'help' for available commands.\n",
                                conn.username, conn.host, conn.port
                            );
                            // 连接成功后切换到终端面板
                            app.active_panel = crate::core::connection::Panel::Terminal;
                            app.show_connection_list = false;
                        }
                        Err(e) => {
                            app.status_message = format!("Connection failed: {}", e);
                        }
                    }
                }
            }
            KeyCode::Char('d') => {
                // 断开连接
                if app.is_connected() {
                    app.active_session = None;
                    app.session = None;
                    app.status_message = "Disconnected".to_string();
                }
            }
            _ => {}
        }
    }

    fn handle_terminal(app: &mut App, key: KeyEvent) {
        match key.code {
            KeyCode::Char(c) => {
                app.command_input.push(c);
            }
            KeyCode::Backspace => {
                app.command_input.pop();
            }
            KeyCode::Enter => {
                let cmd = std::mem::take(&mut app.command_input);
                if !cmd.is_empty() {
                    execute_builtin(app, &cmd);
                }
            }
            _ => {}
        }
    }

    fn handle_file_manager(app: &mut App, key: KeyEvent) {
        // 如果文件列表为空，尝试加载
        if app.panels.file_manager.entries.is_empty() && app.is_connected() {
            if let Some(ref active) = app.active_session {
                match refresh_remote_files(&active.session, &app.remote_cwd) {
                    Ok(entries) => {
                        app.panels.file_manager.entries = entries;
                        app.panels.file_manager.selected_index = 0;
                    }
                    Err(e) => {
                        app.status_message = format!("Failed to list files: {}", e);
                        return;
                    }
                }
            }
        }

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if app.panels.file_manager.selected_index > 0 {
                    app.panels.file_manager.selected_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if app.panels.file_manager.selected_index
                    < app.panels.file_manager.entries.len().saturating_sub(1)
                {
                    app.panels.file_manager.selected_index += 1;
                }
            }
            KeyCode::Enter => {
                // 进入目录或编辑文件
                if let Some(entry) = app
                    .panels
                    .file_manager
                    .entries
                    .get(app.panels.file_manager.selected_index)
                {
                    let entry_path = entry.path.clone();
                    let is_dir = entry.is_dir;
                    let entry_name = entry.name.clone();

                    if is_dir {
                        app.remote_cwd = std::path::PathBuf::from(&entry_path);
                        app.status_message = format!("Changed to {}", entry_path);
                        // 刷新文件列表
                        if let Some(ref active) = app.active_session {
                            if let Ok(entries) =
                                refresh_remote_files(&active.session, &app.remote_cwd)
                            {
                                app.panels.file_manager.entries = entries;
                                app.panels.file_manager.selected_index = 0;
                            }
                        }
                    } else {
                        app.status_message =
                            format!("Selected file: {} (edit not implemented yet)", entry_name);
                    }
                }
            }
            KeyCode::Backspace => {
                // 返回上级目录
                if let Some(parent) = app.remote_cwd.parent() {
                    app.remote_cwd = parent.to_path_buf();
                    app.status_message = format!("Changed to {}", app.remote_cwd.display());
                    // 刷新文件列表
                    if let Some(ref active) = app.active_session {
                        if let Ok(entries) = refresh_remote_files(&active.session, &app.remote_cwd)
                        {
                            app.panels.file_manager.entries = entries;
                            app.panels.file_manager.selected_index = 0;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_resource_dashboard(_app: &mut App, _key: KeyEvent) {
        // 资源看板主要是只读展示
    }

    fn handle_editing(app: &mut App, key: KeyEvent) -> EventResult {
        match key.code {
            KeyCode::Esc => {
                app.editing_connection = false;
            }
            _ => {}
        }
        EventResult::Continue
    }
}

/// 刷新远程文件列表
fn refresh_remote_files(
    session: &ssh2::Session,
    path: &std::path::Path,
) -> Result<Vec<crate::services::sftp_service::SftpEntry>, anyhow::Error> {
    let sftp = session.sftp().context("Failed to open SFTP channel")?;
    let entries = sftp.readdir(path).context("Failed to read directory")?;

    Ok(entries
        .into_iter()
        .map(|(p, stat)| crate::services::sftp_service::SftpEntry {
            name: p
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            path: p.to_string_lossy().to_string(),
            is_dir: stat.is_dir(),
            size: stat.size.unwrap_or(0),
            mtime: stat.mtime.unwrap_or(0),
        })
        .collect())
}

/// 执行内置命令
fn execute_builtin(app: &mut App, cmd: &str) {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    let name = parts.first().map(|s| s.to_lowercase()).unwrap_or_default();

    // Helper to set output based on connection state
    // We'll use a simple approach: if connected, use remote, else use local
    let is_connected = app.is_connected();

    match name.as_str() {
        "help" => {
            app.status_message = "Showing help".to_string();
            let help_text = "=== oh-my-sftp v0.1.0 Help ===\n\n\
                --- Commands ---\n\
                help              Show this help message\n\
                clear             Clear terminal output\n\
                exit / quit       Exit the application\n\
                list              List all saved connections\n\
                connect <index>   Connect to server by index\n\
                disconnect        Disconnect from current server\n\
                status            Show connection status\n\
                refresh           Refresh resource dashboard\n\n\
                --- Hotkeys ---\n\
                Ctrl+O            Toggle connection list\n\
                Ctrl+T            Switch to terminal panel\n\
                Ctrl+F            Switch to file manager (need connection)\n\
                Ctrl+D            Switch to resource dashboard (need connection)\n\
                Tab               Cycle through panels\n\
                Esc               Return to terminal panel\n\
                Ctrl+C / Ctrl+Q   Quit application\n\n\
                --- Connection List ---\n\
                Up/Down or j/k    Navigate connections\n\
                Enter             Connect to selected server\n\
                d                 Disconnect current server\n\n\
                --- File Manager ---\n\
                Up/Down or j/k    Navigate files\n\
                Enter             Enter directory / edit file\n\
                Backspace         Go to parent directory\n";
            if is_connected {
                app.remote_terminal.output = help_text.to_string();
            } else if let Some(ref mut local) = app.local_terminal {
                local.output = help_text.to_string();
            }
        }
        "clear" => {
            if is_connected {
                app.remote_terminal.output.clear();
            } else if let Some(ref mut local) = app.local_terminal {
                local.output.clear();
            }
            app.status_message = "Cleared".to_string();
        }
        "exit" | "quit" => {
            app.should_quit = true;
        }
        "list" => {
            let count = app.connections.len();
            if count == 0 {
                app.status_message = "No connections saved".to_string();
                let msg = "No connections found.\nAdd connections to ~/.ssh/config or use the app config.\n";
                if is_connected {
                    app.remote_terminal.output = msg.to_string();
                } else if let Some(ref mut local) = app.local_terminal {
                    local.output = msg.to_string();
                }
            } else {
                let line: Vec<String> = app
                    .connections
                    .iter()
                    .enumerate()
                    .map(|(i, c)| {
                        format!("  {}: {} ({}@{}:{})", i, c.name, c.username, c.host, c.port)
                    })
                    .collect();
                let msg = format!(
                    "=== Saved Connections ===\n{}\nTotal: {} connections\n",
                    line.join("\n"),
                    count
                );
                if is_connected {
                    app.remote_terminal.output = msg;
                } else if let Some(ref mut local) = app.local_terminal {
                    local.output = msg;
                }
                app.status_message = format!("{} connections listed", count);
            }
        }
        "connect" => {
            let idx_str = parts.get(1).unwrap_or(&"");
            if let Ok(idx) = idx_str.parse::<usize>() {
                if idx < app.connections.len() {
                    let conn = app.connections[idx].clone(); // Clone to avoid borrow conflict
                    app.status_message = format!("Connecting to {}...", conn.name);

                    match crate::services::ssh_service::SshService::connect(&conn, 10) {
                        Ok((session, _tcp)) => {
                            app.active_session = Some(crate::core::connection::ActiveSession {
                                connection_id: conn.id.clone(),
                                session,
                                status: crate::core::connection::ConnectionStatus::Connected,
                            });
                            app.status_message = format!("Connected to {}", conn.name);
                            app.remote_terminal.output = format!(
                                "Connected to {}@{}:{}\n\
                                Type 'help' for available commands.\n\
                                Use Ctrl+F for file manager, Ctrl+D for dashboard.\n",
                                conn.username, conn.host, conn.port
                            );
                        }
                        Err(e) => {
                            app.status_message = format!("Connection failed: {}", e);
                            let msg = format!("Failed to connect to {}:\n{}\n\nTip: Check if the server is reachable and credentials are correct.\n", conn.name, e);
                            // Since connection failed, we are likely still disconnected, so print to local if possible, or just status
                            if let Some(ref mut local) = app.local_terminal {
                                local.output = msg;
                            } else {
                                app.remote_terminal.output = msg;
                            }
                        }
                    }
                } else {
                    app.status_message = format!("Index {} out of range", idx);
                    let msg = format!("Index {} out of range. Available indices: 0 to {}\nUse 'list' to see all connections.\n", idx, app.connections.len().saturating_sub(1));
                    if is_connected {
                        app.remote_terminal.output = msg;
                    } else if let Some(ref mut local) = app.local_terminal {
                        local.output = msg;
                    }
                }
            } else {
                app.status_message = "Usage: connect <index>".to_string();
                let msg = "Usage: connect <index>\nUse 'list' to see available connections and their indices.\n";
                if is_connected {
                    app.remote_terminal.output = msg.to_string();
                } else if let Some(ref mut local) = app.local_terminal {
                    local.output = msg.to_string();
                }
            }
        }
        "disconnect" => {
            if is_connected {
                app.active_session = None;
                app.session = None;
                app.status_message = "Disconnected".to_string();
                if let Some(ref mut local) = app.local_terminal {
                    local.output = "Disconnected from server.\n".to_string();
                }
            } else {
                app.status_message = "Not connected".to_string();
                if let Some(ref mut local) = app.local_terminal {
                    local.output = "Not connected to any server.\n".to_string();
                }
            }
        }
        "status" => {
            if is_connected {
                if let Some(ref session_info) = app.active_session {
                    if let Some(conn) = app
                        .connections
                        .iter()
                        .find(|c| c.id == session_info.connection_id)
                    {
                        app.status_message = format!("Connected to {}", conn.name);
                        let msg = format!(
                            "=== Connection Status ===\n\
                            Server: {}@{}:{}\n\
                            Name: {}\n\
                            Status: Connected\n",
                            conn.username, conn.host, conn.port, conn.name
                        );
                        app.remote_terminal.output = msg;
                    }
                }
            } else {
                app.status_message = "Not connected".to_string();
                if let Some(ref mut local) = app.local_terminal {
                    local.output =
                        "Not connected to any server.\nUse 'list' to see available connections.\n"
                            .to_string();
                }
            }
        }
        "refresh" => {
            if is_connected {
                if let Some(ref active) = app.active_session {
                    match crate::services::resource_service::ResourceService::collect(
                        &active.session,
                    ) {
                        Ok(resources) => {
                            app.resources = resources;
                            app.status_message = "Resources refreshed".to_string();
                            app.remote_terminal.output = format!(
                                "=== Resource Dashboard ===\n\
                                CPU: {:.1}%\n\
                                Memory: {}M / {}M\n\
                                Disk: {:.1}G / {:.1}G\n\
                                Load: {:.1} {:.1} {:.1}\n\
                                Uptime: {}\n",
                                app.resources.cpu_usage,
                                app.resources.memory_used_mb,
                                app.resources.memory_total_mb,
                                app.resources.disk_used_gb,
                                app.resources.disk_total_gb,
                                app.resources.load_average[0],
                                app.resources.load_average[1],
                                app.resources.load_average[2],
                                app.resources.uptime
                            );
                        }
                        Err(e) => {
                            app.status_message = format!("Refresh failed: {}", e);
                            app.remote_terminal.output =
                                format!("Failed to refresh resources:\n{}\n", e);
                        }
                    }
                }
            } else {
                app.status_message = "Not connected".to_string();
                if let Some(ref mut local) = app.local_terminal {
                    local.output = "Not connected. Cannot refresh resources.\n".to_string();
                }
            }
        }
        _ => {
            app.status_message = format!(
                "Unknown command: {}. Type 'help' for available commands.",
                name
            );
            let msg = format!(
                "Unknown command: '{}'\nType 'help' for available commands.\n",
                name
            );
            if is_connected {
                app.remote_terminal.output = msg;
            } else if let Some(ref mut local) = app.local_terminal {
                local.output = msg;
            }
        }
    }
}
