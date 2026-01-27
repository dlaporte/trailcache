//! Ranks tab - shows ranks with scouts working on them.
//!
//! This tab pivots the data to show ranks on the left panel
//! and scouts who are working on or completed that rank on the right panel.

use std::collections::HashMap;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
    Frame,
};

use crate::app::{App, Focus};
use crate::models::{RankProgress, Youth};
use crate::ui::styles;

/// A scout working on a rank with their progress info
#[derive(Clone)]
pub struct ScoutRankProgress<'a> {
    pub youth: &'a Youth,
    pub rank: Option<&'a RankProgress>,
}

/// Standard rank order for sorting
fn rank_order(rank_name: &str) -> usize {
    let lower = rank_name.to_lowercase();
    if lower == "crossover" {
        0
    } else if lower == "scout" {
        1
    } else if lower == "tenderfoot" {
        2
    } else if lower.contains("second class") {
        3
    } else if lower.contains("first class") {
        4
    } else if lower.contains("star") {
        5
    } else if lower.contains("life") {
        6
    } else if lower.contains("eagle") {
        7
    } else {
        8
    }
}

/// Group youth by their current (highest completed) rank.
/// Returns: Vec<(rank_name, Vec<ScoutRankProgress>)>
pub fn get_ranks_with_scouts<'a>(
    youth: &'a [Youth],
    all_ranks: &'a HashMap<i64, Vec<RankProgress>>,
) -> Vec<(String, Vec<ScoutRankProgress<'a>>)> {
    let mut by_rank: HashMap<String, Vec<ScoutRankProgress<'a>>> = HashMap::new();
    let mut crossover_youth: Vec<&'a Youth> = Vec::new();

    for y in youth {
        let user_id = match y.user_id {
            Some(id) => id,
            None => {
                // No user_id - treat as crossover
                crossover_youth.push(y);
                continue;
            }
        };

        let ranks = match all_ranks.get(&user_id) {
            Some(r) => r,
            None => {
                // No rank data - treat as crossover
                crossover_youth.push(y);
                continue;
            }
        };

        // Find the highest completed or awarded rank (current rank)
        let current_rank = ranks.iter()
            .filter(|r| r.is_completed() || r.is_awarded())
            .max_by_key(|r| r.level);

        if let Some(rank) = current_rank {
            let entry = by_rank
                .entry(rank.rank_name.clone())
                .or_insert_with(Vec::new);
            entry.push(ScoutRankProgress { youth: y, rank: Some(rank) });
        } else {
            // No completed ranks - treat as crossover
            crossover_youth.push(y);
        }
    }

    // Add crossover scouts if any
    if !crossover_youth.is_empty() {
        let crossover_list: Vec<ScoutRankProgress<'a>> = crossover_youth
            .into_iter()
            .map(|y| ScoutRankProgress { youth: y, rank: None })
            .collect();
        by_rank.insert("Crossover".to_string(), crossover_list);
    }

    // Sort each rank's scouts: awarded first (by date), then completed (by date), then by name
    for scouts in by_rank.values_mut() {
        scouts.sort_by(|a, b| {
            // Crossover scouts (no rank) sort by name
            match (&a.rank, &b.rank) {
                (None, None) => a.youth.display_name().cmp(&b.youth.display_name()),
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (Some(_), None) => std::cmp::Ordering::Less,
                (Some(a_rank), Some(b_rank)) => {
                    let a_awarded = a_rank.is_awarded();
                    let b_awarded = b_rank.is_awarded();
                    if a_awarded != b_awarded {
                        return b_awarded.cmp(&a_awarded); // awarded comes first
                    }
                    // Same status: sort by date desc
                    let date_a = if a_awarded {
                        a_rank.date_awarded.as_deref().unwrap_or("")
                    } else {
                        a_rank.date_completed.as_deref().unwrap_or("")
                    };
                    let date_b = if b_awarded {
                        b_rank.date_awarded.as_deref().unwrap_or("")
                    } else {
                        b_rank.date_completed.as_deref().unwrap_or("")
                    };
                    date_b.cmp(date_a)
                }
            }
        });
    }

    // Convert to sorted vec by rank order (Scout -> Eagle)
    let mut result: Vec<(String, Vec<ScoutRankProgress>)> = by_rank.into_iter().collect();
    result.sort_by(|a, b| rank_order(&a.0).cmp(&rank_order(&b.0)));

    result
}

/// Get list of unique ranks with counts, sorted by name or count
pub fn get_rank_list(
    youth: &[Youth],
    all_ranks: &HashMap<i64, Vec<RankProgress>>,
    sort_by_count: bool,
    sort_ascending: bool,
) -> Vec<(String, usize)> {
    let grouped = get_ranks_with_scouts(youth, all_ranks);
    let mut result: Vec<(String, usize)> = grouped
        .into_iter()
        .map(|(name, scouts)| (name, scouts.len()))
        .collect();

    if sort_by_count {
        // Sort by count
        if sort_ascending {
            result.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| rank_order(&a.0).cmp(&rank_order(&b.0))));
        } else {
            result.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| rank_order(&a.0).cmp(&rank_order(&b.0))));
        }
    } else {
        // Sort by name (rank order)
        if sort_ascending {
            result.sort_by(|a, b| rank_order(&a.0).cmp(&rank_order(&b.0)));
        } else {
            result.sort_by(|a, b| rank_order(&b.0).cmp(&rank_order(&a.0)));
        }
    }

    result
}

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    render_rank_list(frame, app, chunks[0]);
    render_scout_list(frame, app, chunks[1]);
}

fn render_rank_list(frame: &mut Frame, app: &App, area: Rect) {
    let rank_list = get_rank_list(&app.youth, &app.all_youth_ranks, app.ranks_tab_sort_by_count, app.ranks_tab_sort_ascending);
    let focused = matches!(app.focus, Focus::List);

    // Sort indicators for column headers
    let arrow = if app.ranks_tab_sort_ascending { " ▲" } else { " ▼" };
    let name_indicator = if !app.ranks_tab_sort_by_count { arrow } else { "" };
    let count_indicator = if app.ranks_tab_sort_by_count { arrow } else { "" };

    let header_cells = [
        Cell::from(format!("Name{}", name_indicator)),
        Cell::from(format!("Count{}", count_indicator)),
    ];

    let header = Row::new(header_cells)
        .style(styles::title_style())
        .height(1);

    let rows: Vec<Row> = if rank_list.is_empty() {
        vec![Row::new(vec![
            Cell::from(Span::styled("No rank data loaded", styles::muted_style())),
            Cell::from(""),
        ])]
    } else {
        rank_list.iter().enumerate().map(|(i, (rank, count))| {
            let style = if i == app.ranks_tab_selection {
                styles::selected_style()
            } else {
                styles::list_item_style()
            };

            Row::new(vec![
                Cell::from(rank.clone()),
                Cell::from(format!("{:>6}", count)),
            ]).style(style)
        }).collect()
    };

    let widths = [
        Constraint::Fill(1),
        Constraint::Length(8),
    ];

    let sort_help = "[n]ame [c]ount";
    let title = format!(" Ranks ({}) - {} ", rank_list.len(), sort_help);

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
    state.select(Some(app.ranks_tab_selection));

    frame.render_stateful_widget(table, area, &mut state);
}

fn render_scout_list(frame: &mut Frame, app: &App, area: Rect) {
    let grouped = get_ranks_with_scouts(&app.youth, &app.all_youth_ranks);
    let rank_list = get_rank_list(&app.youth, &app.all_youth_ranks, app.ranks_tab_sort_by_count, app.ranks_tab_sort_ascending);
    let focused = matches!(app.focus, Focus::Detail);

    // Get selected rank name from sorted list, then find scouts
    let selected_rank_name = rank_list.get(app.ranks_tab_selection)
        .map(|(name, _)| name.as_str())
        .unwrap_or("");

    let scouts: Vec<&ScoutRankProgress> = grouped.iter()
        .find(|(name, _)| name == selected_rank_name)
        .map(|(_, scouts)| scouts.iter().collect())
        .unwrap_or_default();

    // If viewing requirements, show that instead
    if app.ranks_tab_viewing_requirements {
        render_requirements_view(frame, app, area, focused);
        return;
    }

    let header_cells = [
        Cell::from("Scout"),
        Cell::from("Date"),
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
        scouts.iter().enumerate().map(|(i, srp)| {
            let style = if i == app.ranks_tab_scout_selection && focused {
                styles::selected_style()
            } else {
                styles::list_item_style()
            };

            let progress = match &srp.rank {
                Some(rank) if rank.is_awarded() => {
                    let date = rank.date_awarded.as_ref()
                        .or(rank.date_completed.as_ref())
                        .map(|d| d.chars().take(10).collect::<String>())
                        .unwrap_or_else(|| "Awarded".to_string());
                    (date, styles::success_style())
                }
                Some(rank) if rank.is_completed() => {
                    let date = rank.date_completed.as_ref()
                        .map(|d| d.chars().take(10).collect::<String>())
                        .unwrap_or_else(|| "Done".to_string());
                    (date, styles::highlight_style())
                }
                Some(rank) => {
                    if let Some(pct) = rank.progress_percent() {
                        (format!("{:>3}%", pct), styles::muted_style())
                    } else {
                        ("  -".to_string(), styles::muted_style())
                    }
                }
                None => ("".to_string(), styles::muted_style()), // Crossover - no rank data
            };

            Row::new(vec![
                Cell::from(srp.youth.display_name()),
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
                .title(format!(" {} ({}) ", selected_rank_name, scouts.len()))
                .title_style(styles::title_style())
                .borders(Borders::ALL)
                .border_style(styles::border_style(focused))
        )
        .row_highlight_style(styles::selected_style());

    let mut state = TableState::default();
    if focused {
        state.select(Some(app.ranks_tab_scout_selection));
    }

    frame.render_stateful_widget(table, area, &mut state);
}

fn render_requirements_view(frame: &mut Frame, app: &App, area: Rect, focused: bool) {
    let grouped = get_ranks_with_scouts(&app.youth, &app.all_youth_ranks);
    let rank_list = get_rank_list(&app.youth, &app.all_youth_ranks, app.ranks_tab_sort_by_count, app.ranks_tab_sort_ascending);

    // Get selected rank info from sorted list
    let selected_rank_name = rank_list.get(app.ranks_tab_selection)
        .map(|(name, _)| name.as_str())
        .unwrap_or("");

    let scouts: Vec<&ScoutRankProgress> = grouped.iter()
        .find(|(name, _)| name == selected_rank_name)
        .map(|(_, scouts)| scouts.iter().collect())
        .unwrap_or_default();

    let selected_scout = scouts.get(app.ranks_tab_scout_selection);

    let mut lines = vec![];

    // Header
    if let Some(srp) = selected_scout {
        lines.push(Line::from(vec![
            Span::styled(srp.youth.display_name(), styles::title_style()),
            Span::styled(" - ", styles::muted_style()),
            Span::styled(selected_rank_name, styles::highlight_style()),
        ]));
    }
    lines.push(Line::from(Span::styled("Press Esc to go back", styles::muted_style())));
    lines.push(Line::from(""));

    if app.selected_rank_requirements.is_empty() {
        lines.push(Line::from(Span::styled("Loading requirements...", styles::muted_style())));
    } else {
        // Count completed
        let completed = app.selected_rank_requirements.iter().filter(|r| r.is_completed()).count();
        let total = app.selected_rank_requirements.len();
        lines.push(Line::from(vec![
            Span::styled("Progress: ", styles::muted_style()),
            Span::styled(format!("{}/{}", completed, total), styles::highlight_style()),
        ]));
        lines.push(Line::from(""));

        // Show requirements
        for (i, req) in app.selected_rank_requirements.iter().enumerate() {
            let is_selected = i == app.ranks_tab_requirement_selection;
            let check = if req.is_completed() { "✓" } else { "○" };
            let check_style = if req.is_completed() { styles::success_style() } else { styles::muted_style() };

            let req_num = req.number();
            let req_text = truncate(&req.text(), 50);

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

    let title = format!(" {} Requirements ", selected_rank_name);

    let block = Block::default()
        .title(title)
        .title_style(styles::title_style())
        .borders(Borders::ALL)
        .border_style(styles::border_style(focused));

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

fn truncate(s: &str, max_len: usize) -> String {
    // Replace tabs with spaces and trim to avoid display width issues
    let cleaned: String = s.replace('\t', " ").trim().to_string();
    if cleaned.len() <= max_len {
        cleaned
    } else {
        format!("{}…", &cleaned[..max_len.saturating_sub(1)])
    }
}
