use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::app::{App, AppState, EventDetailView, LoginFocus, ScoutDetailView, Tab};

use super::styles;
use super::tabs::{badges, events, ranks, roster, unit};

// ============================================================================
// Overlay Constants and Helpers
// ============================================================================

/// Standard overlay width (52 total = 50 interior with borders)
const OVERLAY_WIDTH: u16 = 52;

/// ASCII art logo lines (centered for 50-char interior)
const LOGO_LINE_1: &str = "      ╔╦╗ ╦═╗ ╔═╗ ╦ ╦   ╔═╗ ╔═╗ ╔═╗ ╦ ╦ ╔═╗";
const LOGO_LINE_2: &str = "       ║  ╠╦╝ ╠═╣ ║ ║   ║   ╠═╣ ║   ╠═╣ ║╣ ";
const LOGO_LINE_3: &str = "       ╩  ╩╚═ ╩ ╩ ╩ ╩═╝ ╚═╝ ╩ ╩ ╚═╝ ╩ ╩ ╚═╝";

/// Returns the ASCII logo as styled Lines
fn logo_lines() -> Vec<Line<'static>> {
    vec![
        Line::from(Span::styled(LOGO_LINE_1, styles::title_style())),
        Line::from(Span::styled(LOGO_LINE_2, styles::title_style())),
        Line::from(Span::styled(LOGO_LINE_3, styles::title_style())),
    ]
}

/// Creates a standard overlay block with borders
fn overlay_block() -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(styles::border_style(true))
        .style(Style::default())
}

pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title bar
            Constraint::Length(3), // Tabs
            Constraint::Min(10),   // Main content
            Constraint::Length(2), // Status bar
        ])
        .split(frame.area());

    render_title_bar(frame, app, chunks[0]);
    render_tabs(frame, app, chunks[1]);
    render_main_content(frame, app, chunks[2]);
    render_status_bar(frame, app, chunks[3]);

    // Render overlays
    if matches!(app.state, AppState::ShowingHelp) {
        render_help_overlay(frame, app);
    }

    if matches!(app.state, AppState::LoggingIn) {
        render_login_overlay(frame, app);
    }

    if matches!(app.state, AppState::ConfirmingQuit) {
        render_quit_overlay(frame);
    }

    if matches!(app.state, AppState::ConfirmingOffline) {
        render_offline_overlay(frame);
    }

    if matches!(app.state, AppState::ConfirmingOnline) {
        render_online_overlay(frame);
    }
}

fn render_title_bar(frame: &mut Frame, _app: &App, area: Rect) {
    let title = "  Trailcache";
    let help_hint = "[?] Help";
    let title_len = title.len();

    let title_line = Line::from(vec![
        Span::styled(title, styles::title_style()),
        Span::raw(" ".repeat(
            area.width
                .saturating_sub(title_len as u16 + help_hint.len() as u16 + 4)
                as usize,
        )),
        Span::styled(help_hint, styles::muted_style()),
    ]);

    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(styles::muted_style());

    let paragraph = Paragraph::new(title_line).block(block);
    frame.render_widget(paragraph, area);
}

fn render_tabs(frame: &mut Frame, app: &App, area: Rect) {
    // Build main tabs text
    let main_tabs = [("[1] Scouts", app.current_tab == Tab::Scouts),
        ("[2] Ranks", app.current_tab == Tab::Ranks),
        ("[3] Badges", app.current_tab == Tab::Badges),
        ("[4] Events", app.current_tab == Tab::Events),
        ("[5] Adults", app.current_tab == Tab::Adults),
        ("[6] Unit", app.current_tab == Tab::Unit)];

    let mut spans = vec![Span::raw(" ")];
    for (i, (label, selected)) in main_tabs.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" | ", styles::muted_style()));
        }
        if *selected {
            spans.push(Span::styled(*label, styles::tab_style(true)));
        } else {
            spans.push(Span::styled(*label, styles::muted_style()));
        }
    }

    // Add detail view toggle on the right when on Scouts or Events tab
    let detail_tabs: Option<Vec<(&str, bool)>> = match app.current_tab {
        Tab::Scouts => Some(vec![
            ("[d]etails", app.scout_detail_view == ScoutDetailView::Details),
            ("[r]anks", app.scout_detail_view == ScoutDetailView::Ranks),
            ("[m]erit badges", app.scout_detail_view == ScoutDetailView::MeritBadges),
            ("[l]eadership", app.scout_detail_view == ScoutDetailView::Leadership),
        ]),
        Tab::Events => Some(vec![
            ("[d]etails", app.event_detail_view == EventDetailView::Details),
            ("[r]svp", app.event_detail_view == EventDetailView::Rsvp),
        ]),
        _ => None,
    };

    if let Some(detail_tabs) = detail_tabs {
        // Calculate padding to push detail tabs to the right
        let main_width: usize = spans.iter().map(|s| s.content.len()).sum();
        let detail_width: usize = detail_tabs.iter().map(|(l, _)| l.len()).sum::<usize>()
            + (detail_tabs.len() - 1) * 3; // " | " separators
        let padding = (area.width as usize).saturating_sub(main_width + detail_width + 2);

        spans.push(Span::raw(" ".repeat(padding)));

        for (i, (label, selected)) in detail_tabs.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled(" | ", styles::muted_style()));
            }
            if *selected {
                spans.push(Span::styled(*label, styles::tab_style(true)));
            } else {
                spans.push(Span::styled(*label, styles::muted_style()));
            }
        }
    }

    let line = Line::from(spans);

    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(styles::muted_style());

    let paragraph = Paragraph::new(line).block(block);
    frame.render_widget(paragraph, area);
}

fn render_main_content(frame: &mut Frame, app: &App, area: Rect) {
    match app.current_tab {
        Tab::Scouts => roster::render_scouts(frame, app, area),
        Tab::Adults => roster::render_adults(frame, app, area),
        Tab::Events => events::render(frame, app, area),
        Tab::Unit => unit::render(frame, app, area),
        Tab::Ranks => ranks::render(frame, app, area),
        Tab::Badges => badges::render(frame, app, area),
    }
}

fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let last_updated = app.cache_ages.last_updated();
    let shortcuts = if app.offline_mode {
        "[o]nline | [q]uit"
    } else {
        "[u]pdate | [o]ffline | [q]uit"
    };

    let (left_text, left_style) = if let Some(ref msg) = app.status_message {
        (format!(" {} ", msg), styles::muted_style())
    } else if app.offline_mode {
        (" OFFLINE MODE ".to_string(), styles::error_style())
    } else {
        (format!(" Updated {} ", last_updated), styles::muted_style())
    };

    let right_text = format!(" {} ", shortcuts);

    // Center text for Events tab - show calendar subscribe URL
    let center_text = if app.current_tab == Tab::Events {
        // Get unit_id from first event
        if let Some(unit_id) = app.events.first().and_then(|e| e.unit_id()) {
            format!("Subscribe: https://api.scouting.org/advancements/events/calendar/{}", unit_id)
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let width = area.width as usize;

    if center_text.is_empty() {
        // No center text - just left and right
        let padding_len = width.saturating_sub(left_text.len()).saturating_sub(right_text.len());
        let status_line = Line::from(vec![
            Span::styled(left_text, left_style),
            Span::raw(" ".repeat(padding_len)),
            Span::styled(right_text, styles::muted_style()),
        ]);
        let paragraph = Paragraph::new(status_line).style(styles::status_bar_style());
        frame.render_widget(paragraph, area);
    } else {
        // With center text - center it absolutely, regardless of left/right content
        let center_start = (width.saturating_sub(center_text.len())) / 2;
        let left_pad = center_start.saturating_sub(left_text.len());
        let right_start = center_start + center_text.len();
        let right_pad = width.saturating_sub(right_start).saturating_sub(right_text.len());

        let status_line = Line::from(vec![
            Span::styled(left_text, left_style),
            Span::raw(" ".repeat(left_pad)),
            Span::styled(center_text, styles::muted_style()),
            Span::raw(" ".repeat(right_pad)),
            Span::styled(right_text, styles::muted_style()),
        ]);
        let paragraph = Paragraph::new(status_line).style(styles::status_bar_style());
        frame.render_widget(paragraph, area);
    }
}

fn render_help_overlay(frame: &mut Frame, _app: &App) {
    let area = centered_rect_fixed(OVERLAY_WIDTH, 27, frame.area());
    frame.render_widget(Clear, area);

    let version = env!("CARGO_PKG_VERSION");

    let mut lines = logo_lines();
    lines.extend(vec![
        Line::from(Span::styled(
            format!("                  version {}", version),
            styles::muted_style(),
        )),
        Line::from(""),
        Line::from(Span::styled(" Navigation", styles::highlight_style())),
        Line::from(vec![
            Span::styled("  1-6       ", styles::help_key_style()),
            Span::styled("Switch tabs", styles::help_desc_style()),
        ]),
        Line::from(vec![
            Span::styled("  ←/→       ", styles::help_key_style()),
            Span::styled("Prev/next tab or detail view", styles::help_desc_style()),
        ]),
        Line::from(vec![
            Span::styled("  Tab       ", styles::help_key_style()),
            Span::styled("Switch focus (list ↔ detail)", styles::help_desc_style()),
        ]),
        Line::from(vec![
            Span::styled("  ↑/↓ j/k   ", styles::help_key_style()),
            Span::styled("Navigate list", styles::help_desc_style()),
        ]),
        Line::from(vec![
            Span::styled("  Enter     ", styles::help_key_style()),
            Span::styled("Select / drill down", styles::help_desc_style()),
        ]),
        Line::from(vec![
            Span::styled("  Esc       ", styles::help_key_style()),
            Span::styled("Go back", styles::help_desc_style()),
        ]),
        Line::from(""),
        Line::from(Span::styled(" Actions", styles::highlight_style())),
        Line::from(vec![
            Span::styled("  /         ", styles::help_key_style()),
            Span::styled("Search", styles::help_desc_style()),
        ]),
        Line::from(vec![
            Span::styled("  u         ", styles::help_key_style()),
            Span::styled("Update data from API", styles::help_desc_style()),
        ]),
        Line::from(vec![
            Span::styled("  o         ", styles::help_key_style()),
            Span::styled("Toggle offline mode", styles::help_desc_style()),
        ]),
        Line::from(vec![
            Span::styled("  q         ", styles::help_key_style()),
            Span::styled("Quit", styles::help_desc_style()),
        ]),
        Line::from(""),
        Line::from(Span::styled(" Scouts Tab", styles::highlight_style())),
        Line::from(vec![
            Span::styled("  n/r/p/g/a ", styles::help_key_style()),
            Span::styled("Sort by name/rank/patrol/grade/age", styles::help_desc_style()),
        ]),
        Line::from(vec![
            Span::styled("  d/r/m/l   ", styles::help_key_style()),
            Span::styled("View details/ranks/merit badges/leadership", styles::help_desc_style()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("          Press ", styles::muted_style()),
            Span::styled("?", styles::help_key_style()),
            Span::styled(" or ", styles::muted_style()),
            Span::styled("Esc", styles::help_key_style()),
            Span::styled(" to close", styles::muted_style()),
        ]),
    ]);

    let paragraph = Paragraph::new(lines).block(overlay_block());
    frame.render_widget(paragraph, area);
}

fn render_login_overlay(frame: &mut Frame, app: &App) {
    let height = if app.login_error.is_some() { 14 } else { 12 };
    let area = centered_rect_fixed(OVERLAY_WIDTH, height, frame.area());
    frame.render_widget(Clear, area);

    let mut lines = logo_lines();
    lines.push(Line::from(""));

    // Username field
    let username_focused = app.login_focus == LoginFocus::Username;
    let username_style = if username_focused {
        styles::selected_style()
    } else {
        styles::list_item_style()
    };
    let username_display = format!("{:<16}", app.login_username);
    let cursor = if username_focused { "▌" } else { "" };
    lines.push(Line::from(vec![
        Span::raw("         "),
        Span::styled("Username: [", styles::muted_style()),
        Span::styled(format!("{}{}", username_display, cursor), username_style),
        Span::styled("]", styles::muted_style()),
    ]));

    // Password field
    let password_focused = app.login_focus == LoginFocus::Password;
    let password_style = if password_focused {
        styles::selected_style()
    } else {
        styles::list_item_style()
    };
    let password_masked: String = "*".repeat(app.login_password.len().min(16));
    let password_display = format!("{:<16}", password_masked);
    let cursor = if password_focused { "▌" } else { "" };
    lines.push(Line::from(vec![
        Span::raw("         "),
        Span::styled("Password: [", styles::muted_style()),
        Span::styled(format!("{}{}", password_display, cursor), password_style),
        Span::styled("]", styles::muted_style()),
    ]));

    // Login button
    let button_focused = app.login_focus == LoginFocus::Button;
    let button_style = if button_focused {
        styles::selected_style()
    } else {
        styles::list_item_style()
    };
    lines.push(Line::from(""));
    let button_text = if button_focused { " ▶ Login ◀ " } else { "   Login   " };
    lines.push(Line::from(vec![
        Span::raw("                  ["),
        Span::styled(button_text, button_style),
        Span::raw("]"),
    ]));

    // Error message
    if let Some(ref error) = app.login_error {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!(" {}", error),
            styles::error_style(),
        )));
    }

    let paragraph = Paragraph::new(lines).block(overlay_block());
    frame.render_widget(paragraph, area);
}

/// Create a centered rectangle with fixed dimensions
fn centered_rect_fixed(width: u16, height: u16, r: Rect) -> Rect {
    let x = r.x + (r.width.saturating_sub(width)) / 2;
    let y = r.y + (r.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width.min(r.width), height.min(r.height))
}

fn render_quit_overlay(frame: &mut Frame) {
    let area = centered_rect_fixed(OVERLAY_WIDTH, 10, frame.area());
    frame.render_widget(Clear, area);

    let mut lines = logo_lines();
    lines.extend(vec![
        Line::from(""),
        Line::from(Span::styled(
            "          Are you sure you want to quit?",
            styles::highlight_style(),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("         Press ", styles::muted_style()),
            Span::styled("[Y]", styles::help_key_style()),
            Span::styled(" to quit, ", styles::muted_style()),
            Span::styled("[N]", styles::help_key_style()),
            Span::styled(" to cancel", styles::muted_style()),
        ]),
    ]);

    let paragraph = Paragraph::new(lines).block(overlay_block());
    frame.render_widget(paragraph, area);
}

fn render_offline_overlay(frame: &mut Frame) {
    let area = centered_rect_fixed(OVERLAY_WIDTH, 14, frame.area());
    frame.render_widget(Clear, area);

    let mut lines = logo_lines();
    lines.extend(vec![
        Line::from(""),
        Line::from(Span::styled(
            "               Enter Offline Mode?",
            styles::highlight_style(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "   All data will be cached for offline access.",
            styles::muted_style(),
        )),
        Line::from(Span::styled(
            "    Data will remain static until you go back",
            styles::muted_style(),
        )),
        Line::from(Span::styled(
            "          online by pressing [o] again.",
            styles::muted_style(),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("      Press ", styles::muted_style()),
            Span::styled("[Y]", styles::help_key_style()),
            Span::styled(" to go offline, ", styles::muted_style()),
            Span::styled("[N]", styles::help_key_style()),
            Span::styled(" to cancel", styles::muted_style()),
        ]),
    ]);

    let paragraph = Paragraph::new(lines).block(overlay_block());
    frame.render_widget(paragraph, area);
}

fn render_online_overlay(frame: &mut Frame) {
    let area = centered_rect_fixed(OVERLAY_WIDTH, 12, frame.area());
    frame.render_widget(Clear, area);

    let mut lines = logo_lines();
    lines.extend(vec![
        Line::from(""),
        Line::from(Span::styled(
            "             You are in Offline Mode",
            styles::highlight_style(),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("     Press ", styles::muted_style()),
            Span::styled("[o]", styles::help_key_style()),
            Span::styled(" to go online or any other key", styles::muted_style()),
        ]),
        Line::from(Span::styled(
            "                 to stay offline",
            styles::muted_style(),
        )),
    ]);

    let paragraph = Paragraph::new(lines).block(overlay_block());
    frame.render_widget(paragraph, area);
}

