// Allow dead code: Style functions defined for consistent UI
#![allow(dead_code)]

use ratatui::style::{Color, Modifier, Style};

// Color palette
pub const PRIMARY: Color = Color::Rgb(64, 128, 192);
pub const SECONDARY: Color = Color::Rgb(96, 160, 96);
pub const ACCENT: Color = Color::Rgb(192, 160, 64);
pub const ERROR: Color = Color::Rgb(192, 64, 64);
pub const MUTED: Color = Color::Rgb(128, 128, 128);
pub const HIGHLIGHT: Color = Color::Rgb(48, 48, 64);

// Styles
pub fn title_style() -> Style {
    Style::default().fg(PRIMARY).add_modifier(Modifier::BOLD)
}

pub fn selected_style() -> Style {
    Style::default()
        .bg(HIGHLIGHT)
        .add_modifier(Modifier::BOLD)
}

pub fn list_item_style() -> Style {
    Style::default().fg(Color::White)
}

pub fn muted_style() -> Style {
    Style::default().fg(MUTED)
}

pub fn highlight_style() -> Style {
    Style::default().fg(ACCENT)
}

pub fn success_style() -> Style {
    Style::default().fg(SECONDARY)
}

pub fn error_style() -> Style {
    Style::default().fg(ERROR)
}

pub fn tab_style(selected: bool) -> Style {
    if selected {
        Style::default()
            .fg(PRIMARY)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
    } else {
        Style::default().fg(Color::White)
    }
}

pub fn border_style(focused: bool) -> Style {
    if focused {
        Style::default().fg(PRIMARY)
    } else {
        Style::default().fg(MUTED)
    }
}

pub fn search_style() -> Style {
    Style::default().fg(ACCENT)
}

pub fn status_bar_style() -> Style {
    Style::default().bg(Color::Rgb(32, 32, 40)).fg(Color::White)
}

pub fn help_key_style() -> Style {
    Style::default()
        .fg(ACCENT)
        .add_modifier(Modifier::BOLD)
}

pub fn help_desc_style() -> Style {
    Style::default().fg(Color::White)
}

// Rank-specific colors for charts (all white for consistency)
pub fn eagle_style() -> Style {
    Style::default().fg(Color::White)
}

pub fn life_style() -> Style {
    Style::default().fg(Color::White)
}

pub fn star_style() -> Style {
    Style::default().fg(Color::White)
}
