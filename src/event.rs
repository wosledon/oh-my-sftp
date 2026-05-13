use crate::app::App;
use crate::core::connection::Panel;
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
                // TODO: 触发连接
                let _idx = list.selected_index;
                app.status_message = "Connecting...".to_string();
            }
            KeyCode::Char('d') => {
                // 断开连接
                app.status_message = "Disconnected".to_string();
            }
            _ => {}
        }
    }

    fn handle_terminal(app: &mut App, key: KeyEvent) {
        match key.code {
            KeyCode::Char(c) => {
                // 收集命令输入
                app.command_input.push(c);
            }
            KeyCode::Backspace => {
                app.command_input.pop();
            }
            KeyCode::Enter => {
                // 执行命令
                let cmd = std::mem::take(&mut app.command_input);
                if !cmd.is_empty() {
                    // TODO: 如果是远程模式，通过 SSH 执行
                    app.status_message = format!("Executed: {}", cmd);
                }
            }
            _ => {}
        }
    }

    fn handle_file_manager(app: &mut App, key: KeyEvent) {
        let fm = &mut app.panels.file_manager;
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if fm.selected_index > 0 {
                    fm.selected_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if fm.selected_index < fm.entries.len().saturating_sub(1) {
                    fm.selected_index += 1;
                }
            }
            KeyCode::Enter => {
                // 进入目录或编辑文件
                if let Some(entry) = fm.entries.get(fm.selected_index) {
                    if entry.is_dir {
                        app.remote_cwd = std::path::PathBuf::from(&entry.path);
                        app.status_message = format!("Changed to {}", entry.path);
                    }
                }
            }
            KeyCode::Backspace => {
                // 返回上级目录
                if let Some(parent) = app.remote_cwd.parent() {
                    app.remote_cwd = parent.to_path_buf();
                }
            }
            _ => {}
        }
    }

    fn handle_resource_dashboard(_app: &mut App, _key: KeyEvent) {
        // 资源看板主要是只读展示，按 R 刷新
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
