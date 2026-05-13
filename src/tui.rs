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

    // Guard: skip rendering if terminal is too small
    if area.height < 3 || area.width < 10 {
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(area);

    let title_area = chunks[0];
    let main_area = chunks[1];
    let status_area = chunks[2];

    // Render title bar (always)
    render_title_bar(frame, title_area, app);

    // Render main content (always, even if small)
    render_main(frame, main_area, app);

    // Render status bar (always)
    render_status_bar(frame, status_area, app);
}

// --- title bar ---

fn render_title_bar(frame: &mut Frame, area: Rect, app: &App) {
    let title = format!(" oh-my-sftp v0.1.0 | [{}] ", panel_name(&app.active_panel));
    let block = Block::default()
        .style(Style::default().bg(Color::Blue).fg(Color::White))
        .borders(Borders::NONE);
    
    frame.render_widget(block, area);
    frame.render_widget(
        Paragraph::new(title).style(Style::default().fg(Color::White).bg(Color::Blue)),
        area,
    );
}

// --- main content ---

fn render_main(frame: &mut Frame, area: Rect, app: &App) {
    if app.show_connection_list {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
            .split(area);
        render_conn_list(frame, cols[0], app);
        render_panel(frame, cols[1], app);
    } else {
        render_panel(frame, area, app);
    }
}

fn render_panel(frame: &mut Frame, area: Rect, app: &App) {
    match app.active_panel {
        Panel::Terminal => render_terminal(frame, area, app),
        Panel::FileManager => render_files(frame, area, app),
        Panel::ResourceDashboard => render_dashboard(frame, area, app),
        Panel::ConnectionList => render_conn_list(frame, area, app),
    }
}

// --- connection list ---

fn render_conn_list(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" Connections ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let cons = &app.panels.connection_list.connections;
    let items: Vec<ListItem> = if cons.is_empty() {
        vec![ListItem::new(" (none)")]
    } else {
        cons.iter()
            .enumerate()
            .map(|(i, c)| {
                let s = if i == app.panels.connection_list.selected_index {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                ListItem::new(Line::from(Span::styled(
                    format!(" {} ({}@{})", c.name, c.username, c.host),
                    s,
                )))
            })
            .collect()
    };

    let list = List::new(items).highlight_style(Style::default().bg(Color::DarkGray));
    frame.render_widget(list, inner);
}

// --- terminal ---

fn render_terminal(frame: &mut Frame, area: Rect, app: &App) {
    // Use simple layout: body takes most space, input at bottom
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(3)])
        .split(area);

    // body with visible border and background
    let body_block = Block::default()
        .title(term_title(app))
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Plain)
        .border_style(Style::default().fg(Color::Green))
        .style(Style::default().bg(Color::DarkGray));
    let body_inner = body_block.inner(rows[0]);
    frame.render_widget(body_block, rows[0]);

    let body = term_body(app);
    frame.render_widget(
        Paragraph::new(body)
            .style(Style::default().fg(Color::White))
            .wrap(Wrap { trim: true }),
        body_inner,
    );

    // input with visible border and background
    let input_block = Block::default()
        .title(" Input ")
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Plain)
        .border_style(Style::default().fg(Color::Yellow))
        .style(Style::default().bg(Color::DarkGray));
    let input_inner = input_block.inner(rows[1]);
    frame.render_widget(input_block, rows[1]);

    let txt = if app.command_input.is_empty() {
        "> _".to_string()
    } else {
        format!("> {}", app.command_input)
    };
    frame.render_widget(
        Paragraph::new(txt).style(Style::default().fg(Color::White)),
        input_inner,
    );

    // Set cursor with proper bounds checking
    let cx = input_inner
        .x
        .saturating_add(2)
        .saturating_add(app.command_input.len() as u16);
    if cx < input_inner.right() && input_inner.y < area.bottom() {
        frame.set_cursor_position((cx, input_inner.y));
    }
}

fn term_title(app: &App) -> String {
    if app.is_connected() {
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
        " Terminal ".to_string()
    }
}

fn term_body(app: &App) -> String {
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
            "Local terminal started.\nCtrl+O: connections | Ctrl+T: terminal | Ctrl+C: quit\n".to_string()
        } else {
            local.output.clone()
        }
    } else {
        let hint = if app.status_message.contains("PTY init") {
            "PTY initializing..."
        } else {
            &app.status_message
        };
        format!(
            "{}\n\n\
            --- Commands ---\n\n\
            help        Show this help\n\
            clear       Clear terminal\n\
            exit        Quit\n\
            list        List connections\n\
            connect N   Connect to server N\n\
            disconnect  Disconnect\n\
            status      Show status\n\n\
            --- Hotkeys ---\n\n\
            Ctrl+O      Connection list\n\
            Ctrl+T      Terminal\n\
            Ctrl+C      Quit\n\n\
            Type in the input box below.\n",
            hint
        )
    }
}

// --- file manager ---

fn render_files(frame: &mut Frame, area: Rect, app: &App) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let lb = Block::default()
        .title(format!(" Local: {} ", app.local_cwd.display()))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue));
    let li = lb.inner(cols[0]);
    frame.render_widget(lb, cols[0]);
    frame.render_widget(Paragraph::new("(local files - coming soon)"), li);

    let rb = Block::default()
        .title(format!(" Remote: {} ", app.remote_cwd.display()))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta));
    let ri = rb.inner(cols[1]);
    frame.render_widget(rb, cols[1]);

    let entries: Vec<ListItem> = app
        .panels
        .file_manager
        .entries
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let s = if i == app.panels.file_manager.selected_index {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            };
            let icon = if e.is_dir { "[D]" } else { "[F]" };
            ListItem::new(Line::from(Span::styled(format!(" {} {}", icon, e.name), s)))
        })
        .collect();

    frame.render_widget(List::new(entries), ri);
}

// --- dashboard ---

fn render_dashboard(frame: &mut Frame, area: Rect, app: &App) {
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

    let cpu_g = Gauge::default()
        .block(Block::bordered().title(format!(" CPU: {:.1}% ", r.cpu_usage)))
        .gauge_style(Style::default().fg(cpu_c(r.cpu_usage)))
        .percent(r.cpu_usage as u16);
    frame.render_widget(cpu_g, rows[0]);

    let mp = if r.memory_total_mb > 0 {
        (r.memory_used_mb as f64 / r.memory_total_mb as f64 * 100.0) as u16
    } else {
        0
    };
    let mem_g = Gauge::default()
        .block(Block::bordered().title(format!(
            " Memory: {}M / {}M ({}%) ",
            r.memory_used_mb, r.memory_total_mb, mp
        )))
        .gauge_style(Style::default().fg(mem_c(mp)))
        .percent(mp);
    frame.render_widget(mem_g, rows[1]);

    let dp = if r.disk_total_gb > 0.0 {
        ((r.disk_used_gb / r.disk_total_gb) * 100.0) as u16
    } else {
        0
    };
    let disk_g = Gauge::default()
        .block(Block::bordered().title(format!(
            " Disk: {:.1}G / {:.1}G ({}%) ",
            r.disk_used_gb, r.disk_total_gb, dp
        )))
        .gauge_style(Style::default().fg(disk_c(dp)))
        .percent(dp);
    frame.render_widget(disk_g, rows[2]);

    let info = format!(
        " Load: {:.1} {:.1} {:.1}  |  Uptime: {} ",
        r.load_average[0], r.load_average[1], r.load_average[2], r.uptime
    );
    frame.render_widget(
        Paragraph::new(info).style(Style::default().fg(Color::Gray)),
        rows[3],
    );
}

// --- status bar ---

fn render_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    let bg = Block::default().style(Style::default().bg(Color::Blue));
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

// --- helpers ---

fn panel_name(p: &Panel) -> &str {
    match p {
        Panel::Terminal => "Terminal",
        Panel::FileManager => "Files",
        Panel::ResourceDashboard => "Dashboard",
        Panel::ConnectionList => "Connections",
    }
}

fn cpu_c(v: f64) -> Color {
    if v > 90.0 {
        Color::Red
    } else if v > 70.0 {
        Color::Yellow
    } else {
        Color::Green
    }
}
fn mem_c(v: u16) -> Color {
    if v > 90 {
        Color::Red
    } else if v > 70 {
        Color::Yellow
    } else {
        Color::Green
    }
}
fn disk_c(v: u16) -> Color {
    if v > 90 {
        Color::Red
    } else if v > 70 {
        Color::Yellow
    } else {
        Color::Green
    }
}
