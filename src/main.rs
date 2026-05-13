mod app;
mod components;
mod core;
mod event;
mod services;
mod tui;
mod utils;

use crate::app::App;
use crate::core::config;
use crate::core::connection::Connection;
use crate::event::{EventHandler, EventResult};
use anyhow::Result;
use crossterm::{
    cursor,
    event::{DisableMouseCapture, EnableMouseCapture, Event as CrosstermEvent},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use std::io;

fn main() -> Result<()> {
    // 初始化日志
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp(None)
        .init();

    log::info!("=== oh-my-sftp starting ===");

    // 加载配置
    let mut app_config = config::load_config().unwrap_or_default();

    // 加载 ~/.ssh/config
    let ssh_entries = core::ssh_config::parse_ssh_config().unwrap_or_default();
    log::info!("Loaded {} entries from ~/.ssh/config", ssh_entries.len());

    let ssh_connections: Vec<Connection> = ssh_entries
        .into_iter()
        .map(Connection::from_ssh_config_entry)
        .collect();

    config::merge_ssh_connections(&mut app_config, ssh_connections);
    let _ = config::save_config(&app_config);

    // 初始化应用状态
    let mut app = App::new();
    app.connections = app_config.connections.clone();
    app.settings = app_config.settings.clone();
    app.panels.connection_list.connections = app_config.connections;

    log::info!("App initialized with {} connections", app.connections.len());

    // 启动 TUI
    let result = run_tui(&mut app);

    // 恢复终端
    restore_terminal()?;

    if result.is_err() {
        log::error!("=== oh-my-sftp exited with error ===");
    } else {
        log::info!("=== oh-my-sftp exited normally ===");
    }

    result
}

/// 运行 TUI 主循环
fn run_tui(app: &mut App) -> Result<()> {
    // 设置终端
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        EnableMouseCapture,
        cursor::Hide
    )?;
    terminal::enable_raw_mode()?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;

    log::info!("Terminal setup complete, starting event loop...");

    // 事件循环
    let tick_rate = std::time::Duration::from_millis(100);
    let mut last_tick = std::time::Instant::now();

    loop {
        // 绘制界面
        if let Err(e) = terminal.draw(|f| {
            tui::render(f, app);
        }) {
            log::error!("Draw error: {}", e);
            // 短暂休眠后继续
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        // 检查退出
        if app.should_quit {
            log::info!("Quit requested");
            break;
        }

        // 事件轮询
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| std::time::Duration::from_millis(0));

        match crossterm::event::poll(timeout) {
            Ok(true) => {
                if let Ok(event) = crossterm::event::read() {
                    match event {
                        CrosstermEvent::Key(key) => {
                            match EventHandler::handle_key(app, key) {
                                EventResult::Quit => {
                                    log::info!("Quit via key event");
                                    break;
                                }
                                EventResult::Continue => {}
                            }
                        }
                        CrosstermEvent::Resize(w, h) => {
                            log::debug!("Terminal resized: {}x{}", w, h);
                        }
                        CrosstermEvent::Mouse(_) => {}
                        _ => {}
                    }
                }
            }
            Ok(false) => {
                // 超时，继续循环
            }
            Err(e) => {
                log::error!("Event poll error: {}", e);
                // 继续循环，不退出
            }
        }

        // Tick 事件
        if last_tick.elapsed() >= tick_rate {
            on_tick(app);
            last_tick = std::time::Instant::now();
        }
    }

    log::info!("Event loop exited");
    Ok(())
}

/// 定时任务
fn on_tick(app: &mut App) {
    // 资源看板定时刷新（每 3 秒）
    if app.active_panel == crate::core::connection::Panel::ResourceDashboard
        && app.is_connected()
        && app.last_resource_refresh.elapsed()
            >= std::time::Duration::from_millis(app.settings.refresh_interval_ms)
    {
        if let Some(ref session) = app.session {
            match crate::services::resource_service::ResourceService::collect(session) {
                Ok(resources) => {
                    app.resources = resources;
                }
                Err(e) => {
                    log::warn!("Failed to collect resources: {}", e);
                }
            }
        }
        app.last_resource_refresh = std::time::Instant::now();
    }
}

/// 恢复终端设置
fn restore_terminal() -> Result<()> {
    let mut stdout = io::stdout();
    let _ = execute!(stdout, LeaveAlternateScreen, DisableMouseCapture, cursor::Show);
    terminal::disable_raw_mode()?;
    Ok(())
}