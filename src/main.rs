//! Scoutbook TUI - A terminal user interface for Scoutbook data.
//!
//! This application provides a fast, keyboard-driven interface for viewing
//! and managing Boy Scouts of America troop data from Scoutbook.

mod app;
mod auth;
mod api;
mod cache;
mod config;
mod models;
mod ui;
mod utils;

use std::io;
use std::time::Duration;

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use tracing::info;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use app::{App, AppState};
use ui::input::handle_input;
use ui::render::render;

// ============================================================================
// Constants
// ============================================================================

/// Timeout for polling terminal events (in milliseconds)
const EVENT_POLL_TIMEOUT_MS: u64 = 100;

/// Initialize the tracing subscriber for logging
fn init_tracing() {
    // Set up logging with environment-based filter
    // Use RUST_LOG env var to control log level (e.g., RUST_LOG=debug)
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("warn"));

    tracing_subscriber::registry()
        .with(fmt::layer().with_writer(io::stderr))
        .with(filter)
        .init();
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file if present (silently ignore if not found)
    let _ = dotenvy::dotenv();

    // Initialize logging
    init_tracing();
    info!("Scoutbook TUI starting");

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new().await?;

    // Load cached data first (for display behind login)
    let _ = app.load_from_cache().await;

    // Check if we need to login
    if !app.is_authenticated().await {
        app.start_login();
    } else {
        // Start background refresh if cache is stale
        if app.is_cache_stale() {
            app.refresh_all_background().await;
        }
    }

    // Main loop
    let result = run_app(&mut terminal, &mut app).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        eprintln!("Error: {}", e);
    }

    info!("Scoutbook TUI shutting down");
    Ok(())
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    loop {
        // Draw UI
        terminal.draw(|f| render(f, app))?;

        // Poll for events with timeout to allow background updates
        if event::poll(Duration::from_millis(EVENT_POLL_TIMEOUT_MS))? {
            if let Event::Key(key) = event::read()? {
                // Ctrl+C to quit
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    return Ok(());
                }

                // Handle input
                if handle_input(app, key).await? {
                    return Ok(());
                }
            }
        }

        // Check for completed background tasks
        app.check_background_tasks().await;

        // Check if we should quit
        if matches!(app.state, AppState::Quitting) {
            return Ok(());
        }
    }
}
