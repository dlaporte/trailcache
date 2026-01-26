// Allow dead code: Helper methods for future advancement views
#![allow(dead_code)]

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
    Frame,
};

use crate::app::{AdvancementView, App, Focus};
use crate::models::MeritBadgeProgress;
use crate::ui::styles;

/// Get badges sorted: in-progress first (by percent desc), then completed (by date desc)
pub fn get_sorted_badges(badges: &[MeritBadgeProgress]) -> Vec<&MeritBadgeProgress> {
    let mut sorted: Vec<&MeritBadgeProgress> = badges.iter().collect();
    sorted.sort_by(|a, b| {
        let a_done = a.is_completed();
        let b_done = b.is_completed();
        if a_done != b_done {
            return a_done.cmp(&b_done); // false (in-progress) comes first
        }
        if !a_done {
            // Both in-progress: sort by percent desc
            let pct_a = a.percent_completed.unwrap_or(0.0);
            let pct_b = b.percent_completed.unwrap_or(0.0);
            pct_b.partial_cmp(&pct_a).unwrap_or(std::cmp::Ordering::Equal)
        } else {
            // Both completed: sort by date desc
            let date_a = a.date_completed.as_deref().unwrap_or("");
            let date_b = b.date_completed.as_deref().unwrap_or("");
            date_b.cmp(date_a)
        }
    });
    sorted
}

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    render_scout_list(frame, app, chunks[0]);
    render_advancement_detail(frame, app, chunks[1]);
}

fn render_scout_list(frame: &mut Frame, app: &App, area: Rect) {
    let sorted_youth = app.get_youth_by_rank();

    let header_cells = [
        Cell::from("Name"),
        Cell::from("Rank"),
    ];

    let header = Row::new(header_cells)
        .style(styles::title_style())
        .height(1);

    let rows: Vec<Row> = sorted_youth.iter().enumerate().map(|(i, youth)| {
        let style = if i == app.advancement_selection {
            styles::selected_style()
        } else {
            styles::list_item_style()
        };

        let name = youth.short_name();
        let rank = youth.rank();

        Row::new(vec![
            Cell::from(name),
            Cell::from(rank),
        ]).style(style)
    }).collect();

    let widths = [
        Constraint::Fill(2),
        Constraint::Fill(1),
    ];

    let focused = matches!(app.focus, Focus::List);
    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .title(format!(" Scouts ({}) ", app.youth.len()))
                .title_style(styles::title_style())
                .borders(Borders::ALL)
                .border_style(styles::border_style(focused))
        )
        .row_highlight_style(styles::selected_style());

    let mut state = TableState::default();
    state.select(Some(app.advancement_selection));

    frame.render_stateful_widget(table, area, &mut state);
}

fn render_advancement_detail(frame: &mut Frame, app: &App, area: Rect) {
    let sorted_youth = app.get_youth_by_rank();
    let selected = sorted_youth.get(app.advancement_selection);

    let focused = matches!(app.focus, Focus::Detail);

    // Split into view selector and content
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(5)])
        .split(area);

    // Render view toggle
    render_view_toggle(frame, app, chunks[0]);

    // Render content based on view
    match app.advancement_view {
        AdvancementView::Ranks => render_ranks_view(frame, app, selected, chunks[1], focused),
        AdvancementView::MeritBadges => render_badges_view(frame, app, selected, chunks[1], focused),
    }
}

fn render_view_toggle(frame: &mut Frame, app: &App, area: Rect) {
    let ranks_style = if app.advancement_view == AdvancementView::Ranks {
        styles::highlight_style()
    } else {
        styles::muted_style()
    };
    let badges_style = if app.advancement_view == AdvancementView::MeritBadges {
        styles::highlight_style()
    } else {
        styles::muted_style()
    };

    let line = Line::from(vec![
        Span::styled(" ran[k]s ", ranks_style),
        Span::raw(" | "),
        Span::styled(" [m]erit Badges ", badges_style),
    ]);

    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(styles::muted_style());

    let paragraph = Paragraph::new(line).block(block);
    frame.render_widget(paragraph, area);
}

fn render_ranks_view(frame: &mut Frame, app: &App, selected: Option<&&crate::models::Youth>, area: Rect, focused: bool) {
    // If viewing requirements, show requirements detail
    if app.viewing_requirements {
        render_requirements_view(frame, app, selected, area, focused);
        return;
    }

    let placeholder = "-";

    let content = match selected {
        Some(youth) => {
            let mut lines = vec![];

            // Scout name header
            lines.push(Line::from(Span::styled(
                youth.full_name(),
                styles::title_style(),
            )));
            lines.push(Line::from(vec![
                Span::styled("Current Rank: ", styles::muted_style()),
                Span::styled(youth.rank(), styles::highlight_style()),
            ]));
            lines.push(Line::from(""));

            if app.selected_youth_ranks.is_empty() {
                lines.push(Line::from(Span::styled(
                    "Press Enter to load rank progress",
                    styles::muted_style(),
                )));
            } else {
                // Show all ranks with progress
                lines.push(Line::from(Span::styled("Rank Progress (Enter to view requirements)", styles::highlight_style())));
                lines.push(Line::from(""));

                for (i, rank) in app.selected_youth_ranks.iter().enumerate() {
                    let is_selected = i == app.advancement_rank_selection && focused;
                    let prefix = if is_selected { "> " } else { "  " };

                    let (status_text, status_style) = if rank.is_awarded() {
                        ("Awarded", styles::success_style())
                    } else if rank.is_completed() {
                        ("Complete", styles::highlight_style())
                    } else if let Some(pct) = rank.progress_percent() {
                        if pct > 0 {
                            let text = format!("{}%", pct);
                            (text.leak() as &str, styles::muted_style())
                        } else {
                            (placeholder, styles::muted_style())
                        }
                    } else {
                        (placeholder, styles::muted_style())
                    };

                    let rank_style = if is_selected {
                        styles::selected_style()
                    } else {
                        styles::list_item_style()
                    };

                    lines.push(Line::from(vec![
                        Span::raw(prefix),
                        Span::styled(format!("{:<15}", rank.rank_name), rank_style),
                        Span::styled(format!(" {}", status_text), status_style),
                    ]));

                    // Show date if completed/awarded
                    if is_selected {
                        if let Some(ref date) = rank.date_completed {
                            lines.push(Line::from(vec![
                                Span::raw("    "),
                                Span::styled("Completed:  ", styles::muted_style()),
                                Span::raw(date.chars().take(10).collect::<String>()),
                            ]));
                        }
                        if let Some(ref date) = rank.date_awarded {
                            lines.push(Line::from(vec![
                                Span::raw("    "),
                                Span::styled("Awarded:    ", styles::muted_style()),
                                Span::raw(date.chars().take(10).collect::<String>()),
                            ]));
                        }
                    }
                }
            }

            lines
        }
        None => vec![Line::from(Span::styled(
            "Select a scout from the list",
            styles::muted_style(),
        ))],
    };

    let title = selected
        .map(|y| format!(" {} - Ranks ", y.last_name))
        .unwrap_or_else(|| " Ranks ".to_string());

    let block = Block::default()
        .title(title)
        .title_style(styles::title_style())
        .borders(Borders::ALL)
        .border_style(styles::border_style(focused));

    let paragraph = Paragraph::new(content).block(block);
    frame.render_widget(paragraph, area);
}

fn render_requirements_view(frame: &mut Frame, app: &App, selected: Option<&&crate::models::Youth>, area: Rect, focused: bool) {
    let rank_name = app.selected_youth_ranks
        .get(app.advancement_rank_selection)
        .map(|r| r.rank_name.clone())
        .unwrap_or_else(|| "Rank".to_string());

    let mut lines = vec![];

    // Header
    if let Some(youth) = selected {
        lines.push(Line::from(vec![
            Span::styled(youth.full_name(), styles::title_style()),
            Span::styled(" - ", styles::muted_style()),
            Span::styled(&rank_name, styles::highlight_style()),
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
            let is_selected = i == app.requirement_selection;
            let check = if req.is_completed() { "✓" } else { "○" };
            let check_style = if req.is_completed() { styles::success_style() } else { styles::muted_style() };

            let req_num = req.number();
            let req_text = truncate(&req.text(), 50);

            // Highlight selected requirement
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

    let title = format!(" {} Requirements ", rank_name);

    let block = Block::default()
        .title(title)
        .title_style(styles::title_style())
        .borders(Borders::ALL)
        .border_style(styles::border_style(focused));

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

fn render_badges_view(frame: &mut Frame, app: &App, selected: Option<&&crate::models::Youth>, area: Rect, focused: bool) {
    // If viewing requirements, show requirements detail
    if app.viewing_requirements {
        render_badge_requirements_view(frame, app, selected, area, focused);
        return;
    }

    let content = match selected {
        Some(youth) => {
            let mut lines = vec![];

            // Scout name header
            lines.push(Line::from(Span::styled(
                youth.full_name(),
                styles::title_style(),
            )));

            // Count stats
            let in_progress_count = app.selected_youth_badges.iter().filter(|mb| !mb.is_completed()).count();
            let completed_count = app.selected_youth_badges.iter().filter(|mb| mb.is_completed()).count();
            let eagle_count = app.selected_youth_badges.iter()
                .filter(|mb| mb.is_completed() && mb.is_eagle_required.unwrap_or(false))
                .count();

            lines.push(Line::from(vec![
                Span::styled("Completed: ", styles::muted_style()),
                Span::styled(format!("{}", completed_count), styles::highlight_style()),
                Span::styled(" | In Progress: ", styles::muted_style()),
                Span::styled(format!("{}", in_progress_count), styles::highlight_style()),
                Span::styled(" | Eagle: ", styles::muted_style()),
                Span::styled(format!("{}/13", eagle_count), styles::highlight_style()),
            ]));
            lines.push(Line::from(""));

            if app.selected_youth_badges.is_empty() {
                lines.push(Line::from(Span::styled(
                    "Press Enter to load merit badge progress",
                    styles::muted_style(),
                )));
            } else {
                lines.push(Line::from(Span::styled("Merit Badges (Enter to view requirements)", styles::highlight_style())));
                lines.push(Line::from(""));

                // Get sorted badges using the same method as input handler
                let sorted_badges = get_sorted_badges(&app.selected_youth_badges);

                for (display_idx, badge) in sorted_badges.iter().enumerate() {
                    let is_selected = display_idx == app.advancement_badge_selection && focused;
                    let prefix = if is_selected { "▶ " } else { "  " };

                    let eagle_marker = if badge.is_eagle_required.unwrap_or(false) { "*" } else { " " };

                    let (status_text, status_style) = if badge.is_completed() {
                        let date = badge.date_completed.as_ref()
                            .map(|d| d.chars().take(10).collect::<String>())
                            .unwrap_or_else(|| "Done".to_string());
                        (date, styles::success_style())
                    } else if let Some(pct) = badge.progress_percent() {
                        (format!("{:>3}%", pct), styles::highlight_style())
                    } else {
                        ("  -".to_string(), styles::muted_style())
                    };

                    let name_style = if is_selected { styles::selected_style() } else { styles::list_item_style() };

                    lines.push(Line::from(vec![
                        Span::raw(prefix),
                        Span::styled(eagle_marker, styles::highlight_style()),
                        Span::raw(" "),
                        Span::styled(format!("{:<30}", truncate(&badge.name, 30)), name_style),
                        Span::styled(status_text, status_style),
                    ]));
                }
            }
            lines
        }
        None => vec![Line::from(Span::styled(
            "Select a scout from the list",
            styles::muted_style(),
        ))],
    };

    let block = Block::default()
        .title(" Merit Badges ")
        .title_style(styles::title_style())
        .borders(Borders::ALL)
        .border_style(styles::border_style(focused));

    let paragraph = Paragraph::new(content).block(block);
    frame.render_widget(paragraph, area);
}

fn render_badge_requirements_view(frame: &mut Frame, app: &App, selected: Option<&&crate::models::Youth>, area: Rect, focused: bool) {
    let sorted_badges = get_sorted_badges(&app.selected_youth_badges);
    let badge_name = sorted_badges
        .get(app.advancement_badge_selection)
        .map(|b| b.name.clone())
        .unwrap_or_else(|| "Merit Badge".to_string());

    let mut lines = vec![];

    // Header
    if let Some(youth) = selected {
        lines.push(Line::from(vec![
            Span::styled(youth.full_name(), styles::title_style()),
            Span::styled(" - ", styles::muted_style()),
            Span::styled(&badge_name, styles::highlight_style()),
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
            let is_selected = i == app.requirement_selection;
            let check = if req.is_completed() { "✓" } else { "○" };
            let check_style = if req.is_completed() { styles::success_style() } else { styles::muted_style() };

            let req_num = req.number();
            // Strip HTML tags and truncate
            let raw_text = req.text();
            let clean_text = strip_html(&raw_text);
            let req_text = truncate(&clean_text, 45);

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

    let title = format!(" {} Requirements ", badge_name);

    let block = Block::default()
        .title(title)
        .title_style(styles::title_style())
        .borders(Borders::ALL)
        .border_style(styles::border_style(focused));

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

fn render_badges_column(
    frame: &mut Frame,
    badges: &[&crate::models::MeritBadgeProgress],
    title: &str,
    area: Rect,
    focused: bool,
    show_percent: bool,
) {
    let rows: Vec<Row> = if badges.is_empty() {
        vec![Row::new(vec![
            Cell::from(""),
            Cell::from(Span::styled("None", styles::muted_style())),
            Cell::from(""),
        ])]
    } else {
        badges.iter().map(|mb| {
            let eagle_marker = if mb.is_eagle_required.unwrap_or(false) { "*" } else { " " };

            let info = if show_percent {
                mb.progress_percent()
                    .map(|p| format!("{:>3}%", p))
                    .unwrap_or_else(|| "  -".to_string())
            } else {
                mb.date_completed.as_ref()
                    .map(|d| d.chars().take(10).collect::<String>())
                    .unwrap_or_default()
            };

            let name_style = if show_percent {
                styles::list_item_style()
            } else {
                styles::success_style()
            };

            Row::new(vec![
                Cell::from(Span::styled(eagle_marker, styles::highlight_style())),
                Cell::from(Span::styled(mb.name.clone(), name_style)),
                Cell::from(Span::styled(info, styles::muted_style())),
            ])
        }).collect()
    };

    let widths = [
        Constraint::Length(1),    // Eagle marker
        Constraint::Fill(1),      // Badge name (flexible)
        Constraint::Length(10),   // Percent or date
    ];

    let block = Block::default()
        .title(title)
        .title_style(if show_percent { styles::highlight_style() } else { styles::success_style() })
        .borders(Borders::ALL)
        .border_style(styles::border_style(focused));

    let table = Table::new(rows, widths)
        .block(block);

    frame.render_widget(table, area);
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
    // Also clean up multiple spaces and newlines
    result.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}…", &s[..max_len.saturating_sub(1)])
    }
}
