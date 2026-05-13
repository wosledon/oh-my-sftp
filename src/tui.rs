use crate::app::App;
use crate::core::connection::Panel;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Wrap},
    Frame,
};

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // 如果终端太小，不渲染
    if area.height < 3 || area.width < 10 {
        return;
    }

    // 分割布局：标题栏(1) + 主内容区(Min 1) + 状态栏(1)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(area);

    render_title_bar(frame, chunks[0], app);
    render_main_content(frame, chunks[1], app);
    render_status_bar(frame, chunks[2], app);
}

// ==================== 标题栏 ====================

fn render_title_bar(frame: &mut Frame, area: Rect, app: &App) {
    let title = format!(" oh-my-sftp v0.1.0 | [{}] ", panel_name(&app.active_panel));
    
    let block = Block::default()
        .style(Style::default().bg(Color::Blue))
        .borders(Borders::NONE);
    
    frame.render_widget(block, area);
    frame.render_widget(
        Paragraph::new(title).style(Style::default().fg(Color::White).bg(Color::Blue)),
        area,
    );
}

// ==================== 主内容区 ====================

fn render_main_content(frame: &mut Frame, area: Rect, app: &App) {
    if app.show_connection_list {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
            .split(area);
        
        render_connection_list(frame, cols[0], app);
        render_active_panel(frame, cols[1], app);
    } else {
        render_active_panel(frame, area, app);
    }
}

fn render_active_panel(frame: &mut Frame, area: Rect, app: &App) {
    match app.active_panel {
        Panel::Terminal => render_terminal_panel(frame, area, app),
        Panel::FileManager => render_file_manager_panel(frame, area, app),
        Panel::ResourceDashboard => render_dashboard_panel(frame, area, app),
        Panel::ConnectionList => render_connection_list(frame, area, app),
    }
}

// ==================== 终端面板 ====================

fn render_terminal_panel(frame: &mut Frame, area: Rect, app: &App) {
    // 简单分割：输出区(70%) + 输入区(30%)
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(area);

    // 输出区
    let output_block = Block::default()
        .title(" Terminal ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green))
        .style(Style::default().bg(Color::Black));
    
    let output_inner = output_block.inner(rows[0]);
    frame.render_widget(output_block, rows[0]);
    
    let output_text = get_terminal_output(app);
    frame.render_widget(
        Paragraph::new(output_text)
            .style(Style::default().fg(Color::White))
            .wrap(Wrap { trim: true }),
        output_inner,
    );

    // 输入区
    let input_block = Block::default()
        .title(" Command ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .style(Style::default().bg(Color::Black));
    
    let input_inner = input_block.inner(rows[1]);
    frame.render_widget(input_block, rows[1]);
    
    let input_text = if app.command_input.is_empty() {
        "> _".to_string()
    } else {
        format!("> {}", app.command_input)
    };
    
    frame.render_widget(
        Paragraph::new(input_text).style(Style::default().fg(Color::White)),
        input_inner,
    );

    // 设置光标位置
    let cx = input_inner.x.saturating_add(2).saturating_add(app.command_input.len() as u16);
    if cx < input_inner.right() && input_inner.y < area.bottom() {
        frame.set_cursor_position((cx, input_inner.y));
    }
}

fn get_terminal_output(app: &App) -> String {
    if app.is_connected() {
        if app.remote_terminal.output.is_empty() {
            format!(
                "Connected to {}.\nType 'help' for commands.\n",
                app.current_host().unwrap_or("")
            )
        } else {
            app.remote_terminal.output.clone()
        }
    } else if let Some(ref local) = app.local_terminal {
        if local.output.is_empty() {
            "Local terminal ready.\nCtrl+O: connections | Ctrl+T: terminal | Ctrl+C: quit\n".to_string()
        } else {
            local.output.clone()
        }
    } else {
        "Initializing terminal...\n".to_string()
    }
}

// ==================== 文件管理面板 ====================

fn render_file_manager_panel(frame: &mut Frame, area: Rect, app: &App) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // 本地文件区
    let local_block = Block::default()
        .title(format!(" Local: {} ", app.local_cwd.display()))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue));
    
    let local_inner = local_block.inner(cols[0]);
    frame.render_widget(local_block, cols[0]);
    frame.render_widget(
        Paragraph::new("(local files - coming soon)")
            .style(Style::default().fg(Color::White)),
        local_inner,
    );

    // 远程文件区
    let remote_block = Block::default()
        .title(format!(" Remote: {} ", app.remote_cwd.display()))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta));
    
    let remote_inner = remote_block.inner(cols[1]);
    frame.render_widget(remote_block, cols[1]);

    let entries: Vec<ListItem> = app
        .panels
        .file_manager
        .entries
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let style = if i == app.panels.file_manager.selected_index {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let icon = if e.is_dir { "[D]" } else { "[F]" };
            ListItem::new(Line::from(Span::styled(
                format!(" {} {}", icon, e.name),
                style,
            )))
        })
        .collect();

    frame.render_widget(
        List::new(entries).style(Style::default().fg(Color::White)),
        remote_inner,
    );
}

// ==================== 资源看板面板 ====================

fn render_dashboard_panel(frame: &mut Frame, area: Rect, app: &App) {
    let r = &app.resources;
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(2),
        ])
        .split(area);

    // CPU
    let cpu_gauge = Gauge::default()
        .block(Block::bordered().title(format!(" CPU: {:.1}% ", r.cpu_usage)))
        .gauge_style(Style::default().fg(cpu_color(r.cpu_usage)))
        .percent(r.cpu_usage as u16);
    frame.render_widget(cpu_gauge, rows[0]);

    // 内存
    let mem_percent = if r.memory_total_mb > 0 {
        (r.memory_used_mb as f64 / r.memory_total_mb as f64 * 100.0) as u16
    } else {
        0
    };
    let mem_gauge = Gauge::default()
        .block(Block::bordered().title(format!(
            " Memory: {}M / {}M ({}%) ",
            r.memory_used_mb, r.memory_total_mb, mem_percent
        )))
        .gauge_style(Style::default().fg(mem_color(mem_percent)))
        .percent(mem_percent);
    frame.render_widget(mem_gauge, rows[1]);

    // 磁盘
    let disk_percent = if r.disk_total_gb > 0.0 {
        ((r.disk_used_gb / r.disk_total_gb) * 100.0) as u16
    } else {
        0
    };
    let disk_gauge = Gauge::default()
        .block(Block::bordered().title(format!(
            " Disk: {:.1}G / {:.1}G ({}%) ",
            r.disk_used_gb, r.disk_total_gb, disk_percent
        )))
        .gauge_style(Style::default().fg(disk_color(disk_percent)))
        .percent(disk_percent);
    frame.render_widget(disk_gauge, rows[2]);

    // 系统信息
    let info = format!(
        " Load: {:.1} {:.1} {:.1}  |  Uptime: {} ",
        r.load_average[0], r.load_average[1], r.load_average[2], r.uptime
    );
    frame.render_widget(
        Paragraph::new(info).style(Style::default().fg(Color::Gray)),
        rows[3],
    );
}

// ==================== 连接列表面板 ====================

fn render_connection_list(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" Connections ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Black));
    
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let connections = &app.panels.connection_list.connections;
    let items: Vec<ListItem> = if connections.is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            " (none)",
            Style::default().fg(Color::Gray),
        )))]
    } else {
        connections
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let style = if i == app.panels.connection_list.selected_index {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                ListItem::new(Line::from(Span::styled(
                    format!(" {} ({}@{})", c.name, c.username, c.host),
                    style,
                )))
            })
            .collect()
    };

    let list = List::new(items).highlight_style(Style::default().bg(Color::DarkGray));
    frame.render_widget(list, inner);
}

// ==================== 状态栏 ====================

fn render_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    let bg = Block::default()
        .style(Style::default().bg(Color::Blue))
        .borders(Borders::NONE);
    frame.render_widget(bg, area);

    let left = format!(
        " {} | {} conns | {} tx ",
        app.status_message,
        app.connection_count(),
        app.transfer_queue.len()
    );
    let time = chrono::Local::now().format("%H:%M:%S").to_string();
    let w = area.width as usize;
    let pad = w.saturating_sub(left.len() + time.len());

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(left, Style::default().fg(Color::White).bg(Color::Blue)),
            Span::styled(" ".repeat(pad), Style::default().bg(Color::Blue)),
            Span::styled(time, Style::default().fg(Color::Gray).bg(Color::Blue)),
        ])),
        area,
    );
}

// ==================== 辅助函数 ====================

fn panel_name(p: &Panel) -> &str {
    match p {
        Panel::Terminal => "Terminal",
        Panel::FileManager => "Files",
        Panel::ResourceDashboard => "Dashboard",
        Panel::ConnectionList => "Connections",
    }
}

fn cpu_color(v: f64) -> Color {
    if v > 90.0 {
        Color::Red
    } else if v > 70.0 {
        Color::Yellow
    } else {
        Color::Green
    }
}

fn mem_color(v: u16) -> Color {
    if v > 90 {
        Color::Red
    } else if v > 70 {
        Color::Yellow
    } else {
        Color::Green
    }
}

fn disk_color(v: u16) -> Color {
    if v > 90 {
        Color::Red
    } else if v > 70 {
        Color::Yellow
    } else {
        Color::Green
    }
}