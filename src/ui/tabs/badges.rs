//! Badges tab - shows merit badges with scouts working on them.
//!
//! This tab pivots the data to show merit badges on the left panel
//! and scouts who are working on or completed that badge on the right panel.

use std::collections::HashMap;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
    Frame,
};

use crate::app::{App, Focus};
use crate::models::{MeritBadgeProgress, Youth};
use crate::ui::styles;

/// A scout working on a badge with their progress info
#[derive(Clone)]
pub struct ScoutBadgeProgress<'a> {
    pub youth: &'a Youth,
    pub badge: &'a MeritBadgeProgress,
}

/// Aggregate all youth badges into a list of badges with scouts working on them.
/// Returns: Vec<(badge_name, is_eagle_required, Vec<ScoutBadgeProgress>)>
pub fn get_badges_with_scouts<'a>(
    youth: &'a [Youth],
    all_badges: &'a HashMap<i64, Vec<MeritBadgeProgress>>,
) -> Vec<(String, bool, Vec<ScoutBadgeProgress<'a>>)> {
    let mut by_badge: HashMap<String, (bool, Vec<ScoutBadgeProgress<'a>>)> = HashMap::new();

    for y in youth {
        let user_id = match y.user_id {
            Some(id) => id,
            None => continue,
        };

        let badges = match all_badges.get(&user_id) {
            Some(b) => b,
            None => continue,
        };

        for badge in badges {
            let entry = by_badge
                .entry(badge.name.clone())
                .or_insert_with(|| (badge.is_eagle_required.unwrap_or(false), Vec::new()));

            entry.1.push(ScoutBadgeProgress { youth: y, badge });
        }
    }

    // Sort each badge's scouts by progress (in-progress first by %, then completed by date)
    for (_, scouts) in by_badge.values_mut() {
        scouts.sort_by(|a, b| {
            let a_done = a.badge.is_completed();
            let b_done = b.badge.is_completed();
            if a_done != b_done {
                return a_done.cmp(&b_done); // false (in-progress) comes first
            }
            if !a_done {
                // Both in-progress: sort by percent desc
                let pct_a = a.badge.percent_completed.unwrap_or(0.0);
                let pct_b = b.badge.percent_completed.unwrap_or(0.0);
                pct_b.partial_cmp(&pct_a).unwrap_or(std::cmp::Ordering::Equal)
            } else {
                // Both completed: sort by date desc
                let date_a = a.badge.date_completed.as_deref().unwrap_or("");
                let date_b = b.badge.date_completed.as_deref().unwrap_or("");
                date_b.cmp(date_a)
            }
        });
    }

    // Convert to sorted vec - alphabetically by badge name
    let mut result: Vec<(String, bool, Vec<ScoutBadgeProgress>)> = by_badge
        .into_iter()
        .map(|(name, (eagle, scouts))| (name, eagle, scouts))
        .collect();

    result.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));

    result
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
        .map(|(name, eagle, scouts)| (name, eagle, scouts.len()))
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

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    render_badge_list(frame, app, chunks[0]);
    render_scout_list(frame, app, chunks[1]);
}

fn render_badge_list(frame: &mut Frame, app: &App, area: Rect) {
    let badge_list = get_badge_list(&app.youth, &app.all_youth_badges, app.badges_tab_sort_by_count, app.badges_tab_sort_ascending);
    let focused = matches!(app.focus, Focus::List);

    // Sort indicators for column headers
    let arrow = if app.badges_tab_sort_ascending { " ▲" } else { " ▼" };
    let name_indicator = if !app.badges_tab_sort_by_count { arrow } else { "" };
    let count_indicator = if app.badges_tab_sort_by_count { arrow } else { "" };

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
            let style = if i == app.badges_tab_selection {
                styles::selected_style()
            } else {
                styles::list_item_style()
            };

            let eagle_marker = if *is_eagle { "*" } else { " " };

            Row::new(vec![
                Cell::from(format!("{}{}", eagle_marker, truncate(name, 28))),
                Cell::from(format!("{:>6}", count)),
            ]).style(style)
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

    let mut state = TableState::default();
    state.select(Some(app.badges_tab_selection));

    frame.render_stateful_widget(table, area, &mut state);
}

fn render_scout_list(frame: &mut Frame, app: &App, area: Rect) {
    let grouped = get_badges_with_scouts(&app.youth, &app.all_youth_badges);
    let badge_list = get_badge_list(&app.youth, &app.all_youth_badges, app.badges_tab_sort_by_count, app.badges_tab_sort_ascending);
    let focused = matches!(app.focus, Focus::Detail);

    // Get selected badge name from sorted list, then find scouts
    let selected_badge_name = badge_list.get(app.badges_tab_selection)
        .map(|(name, _, _)| name.as_str())
        .unwrap_or("");

    let scouts: Vec<&ScoutBadgeProgress> = grouped.iter()
        .find(|(name, _, _)| name == selected_badge_name)
        .map(|(_, _, scouts)| scouts.iter().collect())
        .unwrap_or_default();

    // If viewing requirements, show that instead
    if app.badges_tab_viewing_requirements {
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
        scouts.iter().enumerate().map(|(i, sbp)| {
            let style = if i == app.badges_tab_scout_selection && focused {
                styles::selected_style()
            } else {
                styles::list_item_style()
            };

            let progress = if sbp.badge.is_awarded() {
                let date = sbp.badge.awarded_date.as_ref()
                    .or(sbp.badge.date_completed.as_ref())
                    .map(|d| d.chars().take(10).collect::<String>())
                    .unwrap_or_else(|| "Awarded".to_string());
                (date, styles::success_style())
            } else if sbp.badge.is_completed() {
                let date = sbp.badge.date_completed.as_ref()
                    .map(|d| d.chars().take(10).collect::<String>())
                    .unwrap_or_else(|| "Done".to_string());
                (date, styles::highlight_style())
            } else if let Some(pct) = sbp.badge.progress_percent() {
                (format!("{:>3}%", pct), styles::muted_style())
            } else {
                ("  -".to_string(), styles::muted_style())
            };

            Row::new(vec![
                Cell::from(sbp.youth.display_name()),
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

    let mut state = TableState::default();
    if focused {
        state.select(Some(app.badges_tab_scout_selection));
    }

    frame.render_stateful_widget(table, area, &mut state);
}

fn render_requirements_view(frame: &mut Frame, app: &App, area: Rect, focused: bool) {
    let grouped = get_badges_with_scouts(&app.youth, &app.all_youth_badges);
    let badge_list = get_badge_list(&app.youth, &app.all_youth_badges, app.badges_tab_sort_by_count, app.badges_tab_sort_ascending);

    // Get selected badge info from sorted list
    let selected_badge_name = badge_list.get(app.badges_tab_selection)
        .map(|(name, _, _)| name.as_str())
        .unwrap_or("");

    let scouts: Vec<&ScoutBadgeProgress> = grouped.iter()
        .find(|(name, _, _)| name == selected_badge_name)
        .map(|(_, _, scouts)| scouts.iter().collect())
        .unwrap_or_default();

    let selected_scout = scouts.get(app.badges_tab_scout_selection);

    let mut lines = vec![];

    // Header
    if let Some(sbp) = selected_scout {
        lines.push(Line::from(vec![
            Span::styled(sbp.youth.display_name(), styles::title_style()),
            Span::styled(" - ", styles::muted_style()),
            Span::styled(selected_badge_name, styles::highlight_style()),
        ]));
    }
    lines.push(Line::from(Span::styled("Press Esc to go back", styles::muted_style())));
    lines.push(Line::from(""));

    if app.selected_badge_requirements.is_empty() {
        lines.push(Line::from(Span::styled("Loading requirements...", styles::muted_style())));
    } else {
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
            let is_selected = i == app.badges_tab_requirement_selection;
            let check = if req.is_completed() { "✓" } else { "○" };
            let check_style = if req.is_completed() { styles::success_style() } else { styles::muted_style() };

            let req_num = req.number();
            let req_text = truncate(&strip_html(&req.text()), 45);

            let prefix = if is_selected { "▶ " } else { "  " };
            let text_style = if is_selected { styles::selected_style() } else { styles::list_item_style() };

            lines.push(Line::from(vec![
                Span::raw(prefix),
                Span::styled(check, check_style),
                Span::raw(" "),
                Span::styled(format!("{:<5}", req_num), styles::highlight_style()),
                Span::styled(req_text, text_style),
            ]));

            // Show completion date if completed and selected
            if is_selected && req.is_completed() {
                if let Some(ref date) = req.date_completed {
                    if !date.is_empty() {
                        lines.push(Line::from(vec![
                            Span::raw("          "),
                            Span::styled("Completed: ", styles::muted_style()),
                            Span::styled(date.chars().take(10).collect::<String>(), styles::highlight_style()),
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

/// Strip HTML tags from a string
fn strip_html(s: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(c),
            _ => {}
        }
    }
    result.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}…", &s[..max_len.saturating_sub(1)])
    }
}
