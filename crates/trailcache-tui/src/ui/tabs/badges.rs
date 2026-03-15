//! Badges tab - shows merit badges with scouts working on them.
//!
//! This tab pivots the data to show merit badges on the left panel
//! and scouts who are working on or completed that badge on the right panel.

use std::collections::HashMap;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};

use crate::app::{App, Focus};
use trailcache_core::models::{MeritBadgeProgress, Youth};
use trailcache_core::models::pivot::{group_youth_by_badge, BadgeGroup, BadgeGroupEntry};
use crate::ui::styles;
use trailcache_core::utils::{strip_html, wrap_text};

/// Aggregate all youth badges into a list of badges with scouts working on them.
/// Delegates to shared core pivot logic.
pub fn get_badges_with_scouts(
    youth: &[Youth],
    all_badges: &HashMap<i64, Vec<MeritBadgeProgress>>,
) -> Vec<BadgeGroup> {
    group_youth_by_badge(youth, all_badges)
}

/// Get list of unique badges with counts, sorted by name or count
pub fn get_badge_list(
    youth: &[Youth],
    all_badges: &HashMap<i64, Vec<MeritBadgeProgress>>,
    sort_by_count: bool,
    sort_ascending: bool,
) -> Vec<(String, bool, usize)> {
    let grouped = get_badges_with_scouts(youth, all_badges);
    let mut result: Vec<(String, bool, usize)> = grouped
        .into_iter()
        .map(|g| (g.badge_name, g.is_eagle_required, g.scouts.len()))
        .collect();

    if sort_by_count {
        // Sort by count
        if sort_ascending {
            result.sort_by(|a, b| a.2.cmp(&b.2).then_with(|| a.0.to_lowercase().cmp(&b.0.to_lowercase())));
        } else {
            result.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.0.to_lowercase().cmp(&b.0.to_lowercase())));
        }
    } else {
        // Sort by name
        if sort_ascending {
            result.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
        } else {
            result.sort_by(|a, b| b.0.to_lowercase().cmp(&a.0.to_lowercase()));
        }
    }

    result
}

pub fn render(frame: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    app.layout_areas.left_panel = chunks[0];
    app.layout_areas.right_panel = chunks[1];

    render_badge_list(frame, app, chunks[0]);
    render_scout_list(frame, app, chunks[1]);
}

fn render_badge_list(frame: &mut Frame, app: &mut App, area: Rect) {
    let badge_list = get_badge_list(&app.youth, &app.all_youth_badges, app.badges_sort_by_count, app.badges_sort_ascending);
    let focused = matches!(app.focus, Focus::List);

    // Sort indicators for column headers
    let arrow = if app.badges_sort_ascending { " ▲" } else { " ▼" };
    let name_indicator = if !app.badges_sort_by_count { arrow } else { "" };
    let count_indicator = if app.badges_sort_by_count { arrow } else { "" };

    let header_cells = [
        Cell::from(format!("Name{}", name_indicator)),
        Cell::from(format!("Count{}", count_indicator)),
    ];

    let header = Row::new(header_cells)
        .style(styles::title_style())
        .height(1);

    let rows: Vec<Row> = if badge_list.is_empty() {
        vec![Row::new(vec![
            Cell::from(Span::styled("No badge data loaded", styles::muted_style())),
            Cell::from(""),
        ])]
    } else {
        badge_list.iter().enumerate().map(|(i, (name, is_eagle, count))| {
            let style = if i == app.badges_selection {
                styles::selected_style()
            } else {
                styles::list_item_style()
            };

            let eagle_marker = if *is_eagle { "* " } else { "  " };
            let marker_style = if *is_eagle { styles::highlight_style() } else { style };

            Row::new(vec![
                Cell::from(Line::from(vec![
                    Span::styled(eagle_marker, marker_style),
                    Span::styled(name.as_str(), style),
                ])),
                Cell::from(format!("{:>6}", count)).style(style),
            ])
        }).collect()
    };

    let widths = [
        Constraint::Fill(1),
        Constraint::Length(8),
    ];

    let sort_help = "[n]ame [c]ount";
    let title = format!(" Badges ({}) - {} ", badge_list.len(), sort_help);

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .title(title)
                .title_style(styles::muted_style())
                .borders(Borders::ALL)
                .border_style(styles::border_style(focused))
        )
        .row_highlight_style(styles::selected_style());

    app.left_table_state.select(Some(app.badges_selection));
    frame.render_stateful_widget(table, area, &mut app.left_table_state);
}

fn render_scout_list(frame: &mut Frame, app: &mut App, area: Rect) {
    let grouped = get_badges_with_scouts(&app.youth, &app.all_youth_badges);
    let badge_list = get_badge_list(&app.youth, &app.all_youth_badges, app.badges_sort_by_count, app.badges_sort_ascending);
    let focused = matches!(app.focus, Focus::Detail);

    // Get selected badge name from sorted list, then find scouts
    let selected_badge_name = badge_list.get(app.badges_selection)
        .map(|(name, _, _)| name.as_str())
        .unwrap_or("");

    let scouts: Vec<&BadgeGroupEntry> = grouped.iter()
        .find(|g| g.badge_name == selected_badge_name)
        .map(|g| g.scouts.iter().collect())
        .unwrap_or_default();

    // If viewing requirements, show that instead
    if app.badges_viewing_requirements {
        render_requirements_view(frame, app, area, focused);
        return;
    }

    let header_cells = [
        Cell::from("Scout"),
        Cell::from("Awarded"),
    ];

    let header = Row::new(header_cells)
        .style(styles::title_style())
        .height(1);

    let rows: Vec<Row> = if scouts.is_empty() {
        vec![Row::new(vec![
            Cell::from(Span::styled("No scouts", styles::muted_style())),
            Cell::from(""),
        ])]
    } else {
        scouts.iter().enumerate().map(|(i, entry)| {
            let style = if i == app.badges_scout_selection && focused {
                styles::selected_style()
            } else {
                styles::list_item_style()
            };

            let format_date = |d: &str| -> String {
                chrono::NaiveDate::parse_from_str(&d[..10.min(d.len())], "%Y-%m-%d")
                    .ok()
                    .map(|parsed| parsed.format("%b %d, %Y").to_string())
                    .unwrap_or_else(|| d.chars().take(10).collect())
            };

            let progress = if entry.badge.is_awarded() {
                let date = entry.badge.awarded_date.as_ref()
                    .or(entry.badge.date_completed.as_ref())
                    .map(|d| format_date(d))
                    .unwrap_or_else(|| "Awarded".to_string());
                (date, styles::success_style())
            } else if entry.badge.is_completed() {
                let date = entry.badge.date_completed.as_ref()
                    .map(|d| format_date(d))
                    .unwrap_or_else(|| "Done".to_string());
                (date, styles::highlight_style())
            } else if let Some(pct) = entry.badge.progress_percent() {
                (format!("{}%", pct), styles::muted_style())
            } else {
                ("-".to_string(), styles::muted_style())
            };

            Row::new(vec![
                Cell::from(entry.display_name.clone()),
                Cell::from(Span::styled(progress.0, progress.1)),
            ]).style(style)
        }).collect()
    };

    let widths = [
        Constraint::Fill(1),
        Constraint::Length(12),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .title(format!(" {} ({}) ", selected_badge_name, scouts.len()))
                .title_style(styles::title_style())
                .borders(Borders::ALL)
                .border_style(styles::border_style(focused))
        )
        .row_highlight_style(styles::selected_style());

    if focused {
        app.right_table_state.select(Some(app.badges_scout_selection));
    }
    frame.render_stateful_widget(table, area, &mut app.right_table_state);
}

fn render_requirements_view(frame: &mut Frame, app: &mut App, area: Rect, focused: bool) {
    let grouped = get_badges_with_scouts(&app.youth, &app.all_youth_badges);
    let badge_list = get_badge_list(&app.youth, &app.all_youth_badges, app.badges_sort_by_count, app.badges_sort_ascending);

    // Get selected badge info from sorted list
    let selected_badge_name = badge_list.get(app.badges_selection)
        .map(|(name, _, _)| name.as_str())
        .unwrap_or("");

    let scouts: Vec<&BadgeGroupEntry> = grouped.iter()
        .find(|g| g.badge_name == selected_badge_name)
        .map(|g| g.scouts.iter().collect())
        .unwrap_or_default();

    let selected_scout = scouts.get(app.badges_scout_selection);

    let mut lines = vec![];

    // Header
    if let Some(entry) = selected_scout {
        lines.push(Line::from(vec![
            Span::styled(entry.display_name.clone(), styles::title_style()),
            Span::styled(" - ", styles::muted_style()),
            Span::styled(selected_badge_name, styles::highlight_style()),
        ]));
    }
    lines.push(Line::from(Span::styled("Press Esc to go back", styles::muted_style())));
    lines.push(Line::from(""));

    if app.selected_badge_requirements.is_empty() {
        lines.push(Line::from(Span::styled("Loading requirements...", styles::muted_style())));
    } else {
        // Show counselor info if available (from the detail fetch)
        if let Some(ref counselor) = app.selected_badge_counselor {
            let name = counselor.full_name();
            if !name.is_empty() {
                lines.push(Line::from(vec![
                    Span::styled("Counselor: ", styles::muted_style()),
                    Span::styled(name, styles::highlight_style()),
                ]));
                lines.push(Line::from(""));
            }
        }

        // Count completed
        let completed = app.selected_badge_requirements.iter().filter(|r| r.is_completed()).count();
        let total = app.selected_badge_requirements.len();
        lines.push(Line::from(vec![
            Span::styled("Progress: ", styles::muted_style()),
            Span::styled(format!("{}/{}", completed, total), styles::highlight_style()),
        ]));
        lines.push(Line::from(""));

        // Show requirements
        for (i, req) in app.selected_badge_requirements.iter().enumerate() {
            let is_selected = i == app.badges_requirement_selection;
            let check = if req.is_completed() { "✓" } else { "○" };
            let check_style = if req.is_completed() { styles::success_style() } else { styles::muted_style() };

            let req_num = req.number();
            // prefix(2) + check(1) + space(1) + num(5) + margin(2) + borders(2) = 13 chars overhead
            let text_width = (area.width as usize).saturating_sub(13);
            let clean_text = strip_html(&req.text());

            let prefix = if is_selected { "▶ " } else { "  " };
            let text_style = if is_selected { styles::selected_style() } else { styles::list_item_style() };

            let wrapped = wrap_text(&clean_text, text_width);
            lines.push(Line::from(vec![
                Span::raw(prefix),
                Span::styled(check, check_style),
                Span::raw(" "),
                Span::styled(format!("{:<5}", req_num), styles::highlight_style()),
                Span::styled(wrapped[0].clone(), text_style),
            ]));
            let indent = " ".repeat(9);
            for wrap_line in &wrapped[1..] {
                lines.push(Line::from(vec![
                    Span::raw(indent.clone()),
                    Span::styled(wrap_line.clone(), text_style),
                ]));
            }

            // Show completion date if completed and selected
            if is_selected && req.is_completed() {
                if let Some(ref date) = req.date_completed {
                    if !date.is_empty() {
                        let formatted_date = chrono::NaiveDate::parse_from_str(&date[..10.min(date.len())], "%Y-%m-%d")
                            .ok()
                            .map(|d| d.format("%b %d, %Y").to_string())
                            .unwrap_or_else(|| date.chars().take(10).collect());
                        lines.push(Line::from(vec![
                            Span::raw("          "),
                            Span::styled("Completed: ", styles::muted_style()),
                            Span::styled(formatted_date, styles::highlight_style()),
                        ]));
                    }
                }
            }
        }
    }

    let title = format!(" {} Requirements ", selected_badge_name);

    let block = Block::default()
        .title(title)
        .title_style(styles::title_style())
        .borders(Borders::ALL)
        .border_style(styles::border_style(focused));

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

