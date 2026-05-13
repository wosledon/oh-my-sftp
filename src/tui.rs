use crate::app::App;
use crate::core::connection::Panel;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Wrap},
    Frame,
};

/// 渲染主界面
pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // 主布局：垂直分割
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // 标题栏
            Constraint::Min(1),    // 主内容区
            Constraint::Length(1), // 状态栏
        ])
        .split(area);

    render_title_bar(frame, main_chunks[0], app);
    render_main_content(frame, main_chunks[1], app);
    render_status_bar(frame, main_chunks[2], app);
}

fn render_title_bar(frame: &mut Frame, area: Rect, app: &App) {
    let host_info = app
        .current_host()
        .map(|h| format!(" | {}", h))
        .unwrap_or_default();

    let title = format!(
        "oh-my-sftp v0.1.0{} | [{}]",
        host_info,
        panel_name(&app.active_panel)
    );

    let title_widget = Paragraph::new(title)
        .style(Style::default().fg(Color::White).bg(Color::DarkGray))
        .alignment(ratatui::layout::Alignment::Left);

    frame.render_widget(title_widget, area);
}

fn render_main_content(frame: &mut Frame, area: Rect, app: &App) {
    if app.show_connection_list {
        // 显示连接列表面板在主内容区
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
            .split(area);

        render_connection_list(frame, chunks[0], app);
        render_panel_content(frame, chunks[1], app);
    } else {
        render_panel_content(frame, area, app);
    }
}

fn render_panel_content(frame: &mut Frame, area: Rect, app: &App) {
    match app.active_panel {
        Panel::Terminal => render_terminal_panel(frame, area, app),
        Panel::FileManager => render_file_manager_panel(frame, area, app),
        Panel::ResourceDashboard => render_resource_dashboard(frame, area, app),
        Panel::ConnectionList => render_connection_list(frame, area, app),
    }
}

// ─── 连接列表 ───────────────────────────────────────────

fn render_connection_list(frame: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = app
        .panels
        .connection_list
        .connections
        .iter()
        .enumerate()
        .map(|(i, conn)| {
            let is_selected = i == app.panels.connection_list.selected_index;
            let style = if is_selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let line = format!(" {} ({}@{})", conn.name, conn.username, conn.host);
            ListItem::new(Line::from(Span::styled(line, style)))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Connections ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .highlight_style(Style::default().add_modifier(Modifier::BOLD));

    frame.render_widget(list, area);
}

// ─── 终端面板 ───────────────────────────────────────────

fn render_terminal_panel(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(3)])
        .split(area);

    // 终端输出区域
    let output_text = if app.is_connected() {
        if app.remote_terminal.output.is_empty() {
            format!(
                "Connected to {} — Type commands below\n",
                app.current_host().unwrap_or("unknown")
            )
        } else {
            app.remote_terminal.output.clone()
        }
    } else if let Some(ref local) = app.local_terminal {
        if local.output.is_empty() {
            String::from(
                "Local Terminal — Shell started, waiting for output...\n\
                 Press Ctrl+O to open connection list\n",
            )
        } else {
            local.output.clone()
        }
    } else {
        String::from(
            "╔══════════════════════════════════════════════╗\n\
             ║         oh-my-sftp — Terminal Mode           ║\n\
             ╠══════════════════════════════════════════════╣\n\
             ║  Ctrl+O  Open connection list               ║\n\
             ║  Ctrl+T  Terminal panel                     ║\n\
             ║  Ctrl+F  File manager                       ║\n\
             ║  Ctrl+D  Resource dashboard                 ║\n\
             ║  Tab     Switch panels                      ║\n\
             ║  Ctrl+C  Quit                               ║\n\
             ╠══════════════════════════════════════════════╣\n\
             ║  Type commands in the input box below       ║\n\
             ╚══════════════════════════════════════════════╝\n\
             \n\
             Local PTY: not available\n",
        )
    };

    let terminal_block = Paragraph::new(output_text)
        .block(
            Block::default()
                .title(if app.is_connected() {
                    format!(
                        " {}@{} ",
                        app.connections
                            .iter()
                            .find(|c| app
                                .active_session
                                .as_ref()
                                .map_or(false, |s| s.connection_id == c.id))
                            .map(|c| c.username.as_str())
                            .unwrap_or("user"),
                        app.current_host().unwrap_or("")
                    )
                } else {
                    " Local Terminal ".to_string()
                })
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green)),
        )
        .wrap(Wrap { trim: false })
        .scroll((app.remote_terminal.scrollback.len() as u16, 0));

    frame.render_widget(terminal_block, chunks[0]);

    // 命令输入区域
    let input_widget = Paragraph::new(app.command_input.as_str())
        .block(
            Block::default()
                .title(" Command ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        )
        .style(Style::default().fg(Color::White));

    frame.render_widget(input_widget, chunks[1]);

    // 光标位置（简化：固定在输入区末尾）
    if !app.command_input.is_empty() {
        frame.set_cursor_position((
            chunks[1].x + app.command_input.len() as u16 + 1,
            chunks[1].y + 1,
        ));
    }
}

// ─── 文件管理面板 ───────────────────────────────────────

fn render_file_manager_panel(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // 左侧：本地文件浏览占位
    let local_block = Block::default()
        .title(format!(" Local: {} ", app.local_cwd.display()))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue));

    let local_text = Paragraph::new("Local file listing (coming soon)")
        .block(local_block)
        .wrap(Wrap { trim: false });

    frame.render_widget(local_text, chunks[0]);

    // 右侧：远程文件列表
    let entries: Vec<ListItem> = app
        .panels
        .file_manager
        .entries
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let is_selected = i == app.panels.file_manager.selected_index;
            let icon = if entry.is_dir { "📁" } else { "📄" };
            let style = if is_selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else if entry.is_dir {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::White)
            };

            let line = format!(" {} {:<30} {:>10}", icon, entry.name, entry.format_size());
            ListItem::new(Line::from(Span::styled(line, style)))
        })
        .collect();

    let remote_list = List::new(entries)
        .block(
            Block::default()
                .title(format!(" Remote: {} ", app.remote_cwd.display()))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Magenta)),
        )
        .highlight_style(Style::default().add_modifier(Modifier::BOLD));

    frame.render_widget(remote_list, chunks[1]);
}

// ─── 资源看板 ───────────────────────────────────────────

fn render_resource_dashboard(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // CPU
            Constraint::Length(3), // 内存
            Constraint::Length(3), // 磁盘
            Constraint::Length(1), // 负载
            Constraint::Length(1), // 运行时间
        ])
        .split(area);

    let r = &app.resources;

    // CPU 使用率
    let cpu_label = format!(" CPU: {:.1}% ", r.cpu_usage);
    let cpu_gauge = Gauge::default()
        .block(Block::default().title(cpu_label).borders(Borders::ALL))
        .gauge_style(
            Style::default()
                .fg(cpu_color(r.cpu_usage))
                .add_modifier(Modifier::BOLD),
        )
        .percent(r.cpu_usage as u16);
    frame.render_widget(cpu_gauge, chunks[0]);

    // 内存使用率
    let mem_percent = if r.memory_total_mb > 0 {
        (r.memory_used_mb as f64 / r.memory_total_mb as f64 * 100.0) as u16
    } else {
        0
    };
    let mem_label = format!(
        " Memory: {}MB / {}MB ({:.1}%) ",
        r.memory_used_mb, r.memory_total_mb, mem_percent
    );
    let mem_gauge = Gauge::default()
        .block(Block::default().title(mem_label).borders(Borders::ALL))
        .gauge_style(
            Style::default()
                .fg(mem_color(mem_percent))
                .add_modifier(Modifier::BOLD),
        )
        .percent(mem_percent);
    frame.render_widget(mem_gauge, chunks[1]);

    // 磁盘使用率
    let disk_percent = if r.disk_total_gb > 0.0 {
        ((r.disk_used_gb / r.disk_total_gb) * 100.0) as u16
    } else {
        0
    };
    let disk_label = format!(
        " Disk: {:.1}GB / {:.1}GB ({:.1}%) ",
        r.disk_used_gb, r.disk_total_gb, disk_percent
    );
    let disk_gauge = Gauge::default()
        .block(Block::default().title(disk_label).borders(Borders::ALL))
        .gauge_style(
            Style::default()
                .fg(disk_color(disk_percent))
                .add_modifier(Modifier::BOLD),
        )
        .percent(disk_percent);
    frame.render_widget(disk_gauge, chunks[2]);

    // 系统负载
    let load_text = format!(
        " Load Average: {:.2} {:.2} {:.2} | Uptime: {} ",
        r.load_average[0], r.load_average[1], r.load_average[2], r.uptime
    );
    let load_widget = Paragraph::new(load_text)
        .style(Style::default().fg(Color::Gray))
        .block(Block::default().borders(Borders::NONE));
    frame.render_widget(load_widget, chunks[3]);

    // 快捷键提示
    let help_text = " [R] Refresh  |  [Ctrl+T] Terminal  |  [Ctrl+F] Files  |  [Ctrl+D] Dashboard ";
    let help_widget = Paragraph::new(help_text).style(Style::default().fg(Color::DarkGray));
    frame.render_widget(help_widget, chunks[4]);
}

// ─── 状态栏 ─────────────────────────────────────────────

fn render_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    let (left, right) = {
        let status = format!(
            " {} | {} connections | {} transfers ",
            app.status_message,
            app.connection_count(),
            app.transfer_queue.len()
        );
        let time = chrono::Local::now().format("%H:%M:%S").to_string();
        (status, time)
    };

    let left_len = left.len();
    let status_line = Line::from(vec![
        Span::styled(left, Style::default().fg(Color::White).bg(Color::DarkGray)),
        Span::styled(
            format!("{:>width$}", right, width = area.width as usize - left_len),
            Style::default().fg(Color::Gray).bg(Color::DarkGray),
        ),
    ]);

    frame.render_widget(Paragraph::new(status_line), area);
}

// ─── 辅助函数 ───────────────────────────────────────────

fn panel_name(panel: &Panel) -> &str {
    match panel {
        Panel::Terminal => "Terminal",
        Panel::FileManager => "Files",
        Panel::ResourceDashboard => "Dashboard",
        Panel::ConnectionList => "Connections",
    }
}

fn cpu_color(usage: f64) -> Color {
    if usage > 90.0 {
        Color::Red
    } else if usage > 70.0 {
        Color::Yellow
    } else {
        Color::Green
    }
}

fn mem_color(percent: u16) -> Color {
    if percent > 90 {
        Color::Red
    } else if percent > 70 {
        Color::Yellow
    } else {
        Color::Green
    }
}

fn disk_color(percent: u16) -> Color {
    if percent > 90 {
        Color::Red
    } else if percent > 70 {
        Color::Yellow
    } else {
        Color::Green
    }
}
