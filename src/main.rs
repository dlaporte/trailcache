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
mod summaries;
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

    // Load requirement summaries
    summaries::init();

    // Check for CLI commands
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] == "--dump-requirements" {
        return dump_requirements().await;
    }
    if args.len() > 1 && args[1] == "--test-versions" {
        return test_version_endpoints().await;
    }

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

/// Test different API endpoints to find version support
async fn test_version_endpoints() -> Result<()> {
    use std::path::PathBuf;

    eprintln!("Testing version endpoints for Chess (id: 135)...\n");

    // Load config and session
    let config = config::Config::load()?;
    let cache_dir = config.cache_dir().unwrap_or_else(|_| PathBuf::from("./cache"));
    let mut session = auth::Session::new(cache_dir);
    session.load()?;

    let session_data = session.data
        .ok_or_else(|| anyhow::anyhow!("No saved session. Please run the app and login first."))?;

    let client = reqwest::Client::new();
    let token = &session_data.token;

    // Chess has versions: 459 (2026), 143 (2013), 118 (2011)
    let test_urls = vec![
        // Badge detail with versions
        ("v2 badge detail", "https://api.scouting.org/advancements/v2/meritBadges/135"),

        // Try requirements with versionId parameter
        ("requirements (no version)", "https://api.scouting.org/advancements/meritBadges/135/requirements"),
        ("requirements versionId=459 (2026)", "https://api.scouting.org/advancements/meritBadges/135/requirements?versionId=459"),
        ("requirements versionId=143 (2013)", "https://api.scouting.org/advancements/meritBadges/135/requirements?versionId=143"),
        ("requirements versionId=118 (2011)", "https://api.scouting.org/advancements/meritBadges/135/requirements?versionId=118"),

        // Try v2 requirements
        ("v2 requirements", "https://api.scouting.org/advancements/v2/meritBadges/135/requirements"),
        ("v2 requirements versionId=143", "https://api.scouting.org/advancements/v2/meritBadges/135/requirements?versionId=143"),

        // Try version in path
        ("requirements/459", "https://api.scouting.org/advancements/meritBadges/135/requirements/459"),
        ("versions/143/requirements", "https://api.scouting.org/advancements/meritBadges/135/versions/143/requirements"),
    ];

    for (name, url) in test_urls {
        eprint!("Testing {}: ", name);

        match client.get(url).bearer_auth(token).send().await {
            Ok(resp) => {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();

                if status.is_success() {
                    let preview = &body[..body.len().min(300)];
                    eprintln!("✓ {}", status);
                    eprintln!("   Preview: {}\n", preview);
                } else {
                    eprintln!("✗ {}", status);
                }
            }
            Err(e) => {
                eprintln!("✗ Error: {}", e);
            }
        }
    }

    Ok(())
}

/// Dump all merit badge requirements to stdout as JSON
async fn dump_requirements() -> Result<()> {
    use serde::{Serialize, Deserialize};
    use std::path::PathBuf;

    eprintln!("Fetching merit badge catalog...");

    // Load config and session
    let config = config::Config::load()?;
    let cache_dir = config.cache_dir().unwrap_or_else(|_| PathBuf::from("./cache"));
    let mut session = auth::Session::new(cache_dir);
    session.load()?;

    let session_data = session.data
        .ok_or_else(|| anyhow::anyhow!("No saved session. Please run the app and login first."))?;

    let http_client = reqwest::Client::new();
    let token = session_data.token.clone();

    // Fetch the catalog
    let api_client = api::ApiClient::new()?.with_token(token.clone());
    let catalog = api_client.fetch_merit_badge_catalog().await?;
    eprintln!("Found {} merit badges", catalog.len());

    #[derive(Serialize)]
    struct BadgeOutput {
        id: String,
        name: String,
        is_eagle_required: Option<bool>,
        versions: Vec<VersionOutput>,
    }

    #[derive(Serialize)]
    struct VersionOutput {
        version_id: String,
        version: String,
        effective_date: String,
        expiry_date: String,
        requirements: Vec<RequirementOutput>,
    }

    #[derive(Serialize)]
    struct RequirementOutput {
        number: String,
        text: String,
    }

    #[derive(Deserialize)]
    struct BadgeDetail {
        #[serde(default)]
        versions: Vec<VersionInfo>,
    }

    #[derive(Deserialize)]
    struct VersionInfo {
        #[serde(rename = "versionId")]
        version_id: Option<String>,
        version: Option<String>,
        #[serde(rename = "versionEffectiveDt")]
        effective_date: Option<String>,
        #[serde(rename = "versionExpiryDt")]
        expiry_date: Option<String>,
    }

    let mut all_badges = Vec::new();
    let mut total_versions = 0;

    for (i, badge) in catalog.iter().enumerate() {
        let badge_id = badge.id.clone().unwrap_or_default();
        eprintln!("Fetching {} ({}/{})...", badge.name, i + 1, catalog.len());

        // First, get the badge detail to find all versions
        let detail_url = format!("https://api.scouting.org/advancements/v2/meritBadges/{}", badge_id);
        let detail_resp = http_client.get(&detail_url).bearer_auth(&token).send().await;

        let versions_to_fetch: Vec<VersionInfo> = match detail_resp {
            Ok(resp) if resp.status().is_success() => {
                let text = resp.text().await.unwrap_or_default();
                match serde_json::from_str::<BadgeDetail>(&text) {
                    Ok(detail) => detail.versions,
                    Err(_) => vec![],
                }
            }
            _ => vec![],
        };

        if versions_to_fetch.is_empty() {
            eprintln!("  Warning: No versions found for {}", badge.name);
            continue;
        }

        eprintln!("  Found {} versions", versions_to_fetch.len());

        let mut badge_versions = Vec::new();

        for ver in &versions_to_fetch {
            let version_id = ver.version_id.clone().unwrap_or_default();
            let version_name = ver.version.clone().unwrap_or_default();

            if version_id.is_empty() {
                continue;
            }

            // Fetch requirements for this specific version
            let req_url = format!(
                "https://api.scouting.org/advancements/meritBadges/{}/requirements?versionId={}",
                badge_id, version_id
            );

            match http_client.get(&req_url).bearer_auth(&token).send().await {
                Ok(resp) if resp.status().is_success() => {
                    let text = resp.text().await.unwrap_or_default();

                    // Simple requirement struct that matches the API response
                    #[derive(Deserialize, Debug)]
                    struct ApiRequirement {
                        #[serde(default, alias = "listNumber", alias = "number")]
                        list_number: Option<String>,
                        #[serde(default)]
                        name: Option<String>,
                        #[serde(default)]
                        short: Option<String>,
                    }

                    #[derive(Deserialize)]
                    struct ReqWrapper {
                        #[serde(default)]
                        requirements: Vec<ApiRequirement>,
                    }

                    let reqs = match serde_json::from_str::<ReqWrapper>(&text) {
                        Ok(wrapper) => wrapper.requirements,
                        Err(_) => vec![],
                    };

                    let requirements: Vec<RequirementOutput> = reqs.iter().map(|r| {
                        let number = r.list_number.clone().unwrap_or_default();
                        let text = r.short.clone()
                            .filter(|s| !s.is_empty())
                            .or_else(|| r.name.clone())
                            .unwrap_or_default();
                        RequirementOutput { number, text }
                    }).collect();

                    badge_versions.push(VersionOutput {
                        version_id: version_id.clone(),
                        version: version_name.clone(),
                        effective_date: ver.effective_date.clone().unwrap_or_default(),
                        expiry_date: ver.expiry_date.clone().unwrap_or_default(),
                        requirements,
                    });

                    total_versions += 1;
                }
                Ok(resp) => {
                    eprintln!("    Warning: Failed to fetch version {}: {}", version_name, resp.status());
                }
                Err(e) => {
                    eprintln!("    Warning: Failed to fetch version {}: {}", version_name, e);
                }
            }

            // Small delay to avoid rate limiting
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        if !badge_versions.is_empty() {
            all_badges.push(BadgeOutput {
                id: badge_id,
                name: badge.name.clone(),
                is_eagle_required: badge.is_eagle_required,
                versions: badge_versions,
            });
        }

        // Slightly longer delay between badges
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Output as JSON
    let json = serde_json::to_string_pretty(&all_badges)?;
    println!("{}", json);

    eprintln!("Done! {} badges with {} total versions exported.", all_badges.len(), total_versions);
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
