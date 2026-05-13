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
    event::{EnableMouseCapture, Event as CrosstermEvent},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use std::io;
use std::sync::mpsc;

/// PTY 初始化结果（从后台线程传回主线程）
struct PtyReady {
    writer: Box<dyn std::io::Write + Send>,
    rx: mpsc::Receiver<String>,
    _pair: portable_pty::PtyPair,
    _child: Box<dyn portable_pty::Child + Send>,
}

fn main() -> Result<()> {
    // 初始化日志 - 使用更详细的级别以便调试
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp(None)
        .init();

    log::info!("=== oh-my-sftp starting ===");

    // 加载配置
    let mut app_config = match config::load_config() {
        Ok(cfg) => cfg,
        Err(e) => {
            log::warn!("Failed to load config: {}", e);
            config::AppConfig::default()
        }
    };

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

    // 在后台线程启动 PTY 初始化，不阻塞主线程
    let pty_ready_rx = spawn_pty_init();

    // 设置 panic hook 以便在崩溃时恢复终端
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // 先恢复终端
        let _ = restore_terminal();
        // 输出 panic 信息到文件以便调试
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("panic.log")
        {
            use std::io::Write;
            let _ = writeln!(file, "PANIC: {}", info);
        }
        // 然后调用默认的 panic 处理
        default_hook(info);
    }));

    // 启动 TUI
    log::info!("Starting TUI...");
    let result = run_tui(&mut app, pty_ready_rx);

    // 恢复终端
    let restore_result = restore_terminal();
    if let Err(e) = restore_result {
        log::error!("Failed to restore terminal: {}", e);
    }

    match result {
        Ok(()) => log::info!("=== oh-my-sftp exited normally ==="),
        Err(ref e) => log::error!("=== oh-my-sftp exited with error: {} ===", e),
    }

    result
}

/// 在后台线程中初始化 PTY，通过 channel 将结果传回主线程
fn spawn_pty_init() -> mpsc::Receiver<Result<PtyReady>> {
    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        let result = try_init_pty();
        // 如果发送失败说明主线程已退出，忽略错误
        let _ = tx.send(result);
    });

    rx
}

/// 后台线程中执行 PTY 初始化
fn try_init_pty() -> Result<PtyReady> {
    use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};

    let pty_system = NativePtySystem::default();

    let shell = if cfg!(windows) {
        "cmd.exe".to_string()
    } else {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string())
    };

    let cmd = CommandBuilder::new(&shell);
    let pty_size = PtySize {
        rows: 30,
        cols: 100,
        pixel_width: 0,
        pixel_height: 0,
    };

    let pair = pty_system
        .openpty(pty_size)
        .map_err(|e| anyhow::anyhow!("Failed to open PTY: {}", e))?;

    let child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|e| anyhow::anyhow!("Failed to spawn shell: {}", e))?;

    let mut reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| anyhow::anyhow!("Failed to clone PTY reader: {}", e))?;

    let writer: Box<dyn std::io::Write + Send> = Box::new(
        pair.master
            .take_writer()
            .map_err(|e| anyhow::anyhow!("Failed to take PTY writer: {}", e))?,
    );

    let (tx, rx) = mpsc::channel::<String>();

    std::thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let output = String::from_utf8_lossy(&buf[..n]).to_string();
                    if tx.send(output).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    Ok(PtyReady {
        writer,
        rx,
        _pair: pair,
        _child: child,
    })
}

/// 运行 TUI 主循环
fn run_tui(app: &mut App, pty_ready_rx: mpsc::Receiver<Result<PtyReady>>) -> Result<()> {
    log::info!("Setting up terminal...");

    // 设置终端
    let mut stdout = io::stdout();
    // 在 Windows 上可能需要避免使用 EnterAlternateScreen，但某些功能需要它
    let _ = execute!(
        stdout,
        EnterAlternateScreen,
        EnableMouseCapture,
        cursor::Hide
    );
    terminal::enable_raw_mode()?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;

    log::info!("Terminal setup complete, starting event loop...");

    // 事件循环
    let tick_rate = std::time::Duration::from_millis(100);
    let mut last_tick = std::time::Instant::now();
    let mut frame_count = 0u64;

    loop {
        frame_count += 1;

        // 每 100 帧输出一次日志
        if frame_count % 100 == 0 {
            log::debug!("Frame #{}", frame_count);
        }

        // 绘制
        if let Err(e) = terminal.draw(|f| {
            tui::render(f, app);
        }) {
            log::error!("Draw error: {}", e);
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
                        CrosstermEvent::Key(key) => match EventHandler::handle_key(app, key) {
                            EventResult::Quit => {
                                log::info!("Quit via key event");
                                break;
                            }
                            EventResult::Continue => {}
                        },
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
            }
        }

        // Tick 事件
        if last_tick.elapsed() >= tick_rate {
            on_tick(app, &pty_ready_rx);
            last_tick = std::time::Instant::now();
        }
    }

    log::info!("Event loop exited after {} frames", frame_count);
    Ok(())
}

/// 定时任务
fn on_tick(app: &mut App, pty_ready_rx: &mpsc::Receiver<Result<PtyReady>>) {
    // 检查后台 PTY 初始化是否完成
    if app.local_terminal.is_none() && !app.pty_init_done {
        match pty_ready_rx.try_recv() {
            Ok(Ok(ready)) => {
                app.local_terminal = Some(app::LocalTerminal {
                    writer: Some(ready.writer),
                    pty_rx: Some(ready.rx),
                    _pty_pair: Some(ready._pair),
                    _child: Some(ready._child),
                    output: String::new(),
                    scrollback: Vec::new(),
                });
                app.status_message = "Local terminal ready".to_string();
                log::info!("PTY initialized via background thread");
                app.pty_init_done = true;
            }
            Ok(Err(e)) => {
                log::warn!("Background PTY init failed: {}", e);
                app.status_message = format!("PTY unavailable: {}", e);
                app.pty_init_done = true;
            }
            Err(mpsc::TryRecvError::Empty) => {
                // PTY 还在初始化中，继续等待
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                log::warn!("PTY init thread disconnected unexpectedly");
                app.status_message = "PTY init thread crashed".to_string();
                app.pty_init_done = true;
            }
        }
    }

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

    // 读取本地终端输出（来自后台线程的 channel）
    if let Some(ref mut local_term) = app.local_terminal {
        while let Some(output) = local_term.pty_rx.as_ref().and_then(|rx| rx.try_recv().ok()) {
            local_term.output.push_str(&output);
        }
        // 限制 scrollback 大小
        let lines: Vec<&str> = local_term.output.lines().collect();
        if lines.len() > 1000 {
            local_term.output = lines[lines.len() - 500..].join("\n");
        }
    }
}

/// 恢复终端设置
fn restore_terminal() -> Result<()> {
    let mut stdout = io::stdout();
    let _ = execute!(stdout, LeaveAlternateScreen, cursor::Show);
    terminal::disable_raw_mode()?;
    Ok(())
}
