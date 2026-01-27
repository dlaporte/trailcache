use chrono::{NaiveDate, Utc};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
    Frame,
};

use crate::app::{App, Focus, ScoutDetailView};
use crate::models::ScoutSortColumn;
use crate::ui::styles;
use crate::ui::tabs::advancement::get_sorted_badges;

/// Render the Scouts tab - table with sortable columns
pub fn render_scouts(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    render_scout_table(frame, app, chunks[0]);
    render_scout_detail(frame, app, chunks[1]);
}

fn render_scout_table(frame: &mut Frame, app: &App, area: Rect) {
    let sorted_youth = app.get_sorted_youth();
    let focused = matches!(app.focus, Focus::List);

    // Build header with sort indicators
    let sort_indicator = |col: ScoutSortColumn| {
        if app.scout_sort_column == col {
            if app.scout_sort_ascending { " ▲" } else { " ▼" }
        } else {
            ""
        }
    };

    let header_cells = [
        Cell::from(format!("Name{}", sort_indicator(ScoutSortColumn::Name))),
        Cell::from(format!("Patrol{}", sort_indicator(ScoutSortColumn::Patrol))),
        Cell::from(format!("Rank{}", sort_indicator(ScoutSortColumn::Rank))),
        Cell::from(format!("Gr{}", sort_indicator(ScoutSortColumn::Grade))),
        Cell::from(format!("Age{}", sort_indicator(ScoutSortColumn::Age))),
    ];

    let header = Row::new(header_cells)
        .style(styles::title_style())
        .height(1);

    // Build rows
    let rows: Vec<Row> = sorted_youth.iter().enumerate().map(|(i, youth)| {
        let style = if i == app.roster_selection {
            styles::selected_style()
        } else {
            styles::list_item_style()
        };

        let name = youth.display_name();
        let patrol = youth.patrol();
        let rank = youth.rank();
        let grade = youth.grade_str();
        let age = youth.age_str();

        Row::new(vec![
            Cell::from(name),
            Cell::from(patrol),
            Cell::from(rank),
            Cell::from(format!("{:>2}", grade)),
            Cell::from(format!("{:>2}", age)),
        ]).style(style)
    }).collect();

    // Use percentage for name to ensure consistent width with Adults tab
    let widths = [
        Constraint::Percentage(38), // Name - same width as Adults tab
        Constraint::Fill(2),        // Patrol
        Constraint::Fill(2),        // Rank
        Constraint::Length(3),      // Grade
        Constraint::Length(4),      // Age
    ];

    let sort_help = "[n]ame [p]atrol [r]ank [g]rade [a]ge";
    let title = format!(" Scouts ({}) - {} ", app.youth.len(), sort_help);

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
    state.select(Some(app.roster_selection));

    frame.render_stateful_widget(table, area, &mut state);
}

fn render_scout_detail(frame: &mut Frame, app: &App, area: Rect) {
    let sorted_youth = app.get_sorted_youth();
    let selected = sorted_youth.get(app.roster_selection);
    let focused = matches!(app.focus, Focus::Detail);

    // Render content based on view (view toggle is now in the tab bar)
    match app.scout_detail_view {
        ScoutDetailView::Details => render_details_view(frame, app, selected, area, focused),
        ScoutDetailView::Ranks => render_ranks_view(frame, app, selected, area, focused),
        ScoutDetailView::MeritBadges => render_badges_view(frame, app, selected, area, focused),
    }
}

fn render_details_view(frame: &mut Frame, app: &App, selected: Option<&&crate::models::Youth>, area: Rect, focused: bool) {
    let placeholder = "-";

    let content = match selected {
        Some(youth) => {
            let mut lines = vec![];

            // Name header (display_name already includes nickname if different from first name)
            lines.push(Line::from(Span::styled(youth.display_name(), styles::title_style())));

            lines.push(Line::from(""));

            // Unit Info section (always show all fields)
            lines.push(Line::from(Span::styled("Unit Info", styles::highlight_style())));

            lines.push(Line::from(vec![
                Span::styled("Patrol:     ", styles::muted_style()),
                Span::raw(youth.patrol()),
            ]));

            lines.push(Line::from(vec![
                Span::styled("Rank:       ", styles::muted_style()),
                Span::raw(youth.rank()),
            ]));

            let position = youth.position_display().unwrap_or_else(|| placeholder.to_string());
            lines.push(Line::from(vec![
                Span::styled("Position:   ", styles::muted_style()),
                Span::raw(position),
            ]));

            // Membership status from registrarInfo
            if let Some(ref reg_info) = youth.registrar_info {
                if let Some(ref exp_date) = reg_info.registration_expire_dt {
                    if let Ok(date) = chrono::NaiveDate::parse_from_str(&exp_date[..10], "%Y-%m-%d") {
                        let today = chrono::Utc::now().date_naive();
                        let formatted_date = date.format("%b %d, %Y").to_string();

                        let (status_text, status_style) = if date < today {
                            (format!("Expired {}", formatted_date), styles::error_style())
                        } else {
                            (format!("Expires {}", formatted_date), styles::success_style())
                        };

                        lines.push(Line::from(vec![
                            Span::styled("Membership: ", styles::muted_style()),
                            Span::styled(status_text, status_style),
                        ]));
                    }
                }
            }

            let bsa_id = youth.member_id.clone().unwrap_or_else(|| placeholder.to_string());
            lines.push(Line::from(vec![
                Span::styled("BSA ID:     ", styles::muted_style()),
                Span::raw(bsa_id),
            ]));

            lines.push(Line::from(""));

            // Basic Info section (always show all fields)
            lines.push(Line::from(Span::styled("Basic Info", styles::highlight_style())));

            let age_str = youth.age()
                .map(|age| {
                    youth.date_of_birth()
                        .map(|dob| format!("{} (born {})", age, dob.format("%b %d, %Y")))
                        .unwrap_or_else(|| age.to_string())
                })
                .unwrap_or_else(|| placeholder.to_string());
            lines.push(Line::from(vec![
                Span::styled("Age:    ", styles::muted_style()),
                Span::raw(age_str),
            ]));

            let gender_str = youth.gender.as_deref().unwrap_or(placeholder);
            lines.push(Line::from(vec![
                Span::styled("Gender: ", styles::muted_style()),
                Span::raw(gender_str),
            ]));

            let grade_str = youth.grade.map(|g| g.to_string()).unwrap_or_else(|| placeholder.to_string());
            lines.push(Line::from(vec![
                Span::styled("Grade:  ", styles::muted_style()),
                Span::raw(grade_str),
            ]));

            lines.push(Line::from(""));

            // Contact section (always show all fields)
            lines.push(Line::from(Span::styled("Contact", styles::highlight_style())));

            let phone = youth.phone().unwrap_or_else(|| placeholder.to_string());
            lines.push(Line::from(vec![
                Span::styled("Phone:   ", styles::muted_style()),
                Span::raw(phone),
            ]));

            let email = youth.email().map(|e| truncate(&e, 28)).unwrap_or_else(|| placeholder.to_string());
            lines.push(Line::from(vec![
                Span::styled("Email:   ", styles::muted_style()),
                Span::raw(email),
            ]));

            let addr_line1 = youth.primary_address_info.as_ref()
                .and_then(|a| a.address1.clone())
                .filter(|a| !a.trim().is_empty())
                .unwrap_or_else(|| placeholder.to_string());
            lines.push(Line::from(vec![
                Span::styled("Address: ", styles::muted_style()),
                Span::raw(addr_line1),
            ]));

            let addr_line2 = youth.primary_address_info.as_ref()
                .and_then(|a| {
                    a.city_state().map(|cs| {
                        format!("{} {}", cs, a.zip_code.as_deref().unwrap_or(""))
                    })
                })
                .unwrap_or_default();
            lines.push(Line::from(vec![
                Span::raw("         "), // 9 spaces to align with "Address: "
                Span::raw(addr_line2),
            ]));

            lines.push(Line::from(""));

            // Parents section
            lines.push(Line::from(Span::styled("Parents/Guardians", styles::highlight_style())));

            if let Some(user_id) = youth.user_id {
                let parents = app.get_parents_for_youth(user_id);
                if parents.is_empty() {
                    lines.push(Line::from(vec![
                        Span::styled("          ", styles::muted_style()),
                        Span::styled(placeholder, styles::muted_style()),
                    ]));
                } else {
                    for parent in parents.iter().take(2) {
                        // Name
                        lines.push(Line::from(vec![
                            Span::styled("  ", styles::muted_style()),
                            Span::styled(parent.full_name(), styles::title_style()),
                        ]));
                        // Phone
                        let phone = parent.phone().unwrap_or_else(|| placeholder.to_string());
                        lines.push(Line::from(vec![
                            Span::styled("    Phone:   ", styles::muted_style()),
                            Span::raw(phone),
                        ]));
                        // Email
                        let email = parent.email.as_deref().unwrap_or(placeholder);
                        lines.push(Line::from(vec![
                            Span::styled("    Email:   ", styles::muted_style()),
                            Span::raw(truncate(email, 26)),
                        ]));
                        // Address - multiple lines
                        let addr1 = parent.address1.as_deref()
                            .filter(|a| !a.trim().is_empty())
                            .unwrap_or(placeholder);
                        lines.push(Line::from(vec![
                            Span::styled("    Address: ", styles::muted_style()),
                            Span::raw(addr1.to_string()),
                        ]));
                        // City, State ZIP - align under street address
                        if parent.city.is_some() || parent.state.is_some() {
                            let city = parent.city.as_deref().unwrap_or("");
                            let state = parent.state.as_deref().unwrap_or("");
                            let zip = parent.zip.as_deref().unwrap_or("");
                            lines.push(Line::from(vec![
                                Span::raw("             "), // 4 spaces + 9 for "Address: " = 13 total
                                Span::raw(format!("{}, {} {}", city, state, zip)),
                            ]));
                        }
                        lines.push(Line::from(""));
                    }
                }
            } else {
                lines.push(Line::from(vec![
                    Span::styled("          ", styles::muted_style()),
                    Span::styled(placeholder, styles::muted_style()),
                ]));
            }

            lines
        }
        None => vec![Line::from(Span::styled(
            "No scout selected",
            styles::muted_style(),
        ))],
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(styles::border_style(focused));

    let paragraph = Paragraph::new(content).block(block);
    frame.render_widget(paragraph, area);
}

fn render_ranks_view(frame: &mut Frame, app: &App, selected: Option<&&crate::models::Youth>, area: Rect, focused: bool) {
    // If viewing requirements, show requirements detail
    if app.viewing_requirements {
        render_rank_requirements_view(frame, app, selected, area, focused);
        return;
    }

    let placeholder = "-";

    let content = match selected {
        Some(youth) => {
            let mut lines = vec![];

            // Scout name header
            lines.push(Line::from(Span::styled(
                youth.display_name(),
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

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(styles::border_style(focused));

    let paragraph = Paragraph::new(content).block(block);
    frame.render_widget(paragraph, area);
}

fn render_rank_requirements_view(frame: &mut Frame, app: &App, selected: Option<&&crate::models::Youth>, area: Rect, focused: bool) {
    let rank_name = app.selected_youth_ranks
        .get(app.advancement_rank_selection)
        .map(|r| r.rank_name.clone())
        .unwrap_or_else(|| "Rank".to_string());

    let mut lines = vec![];

    // Header
    if let Some(youth) = selected {
        lines.push(Line::from(Span::styled(
            youth.display_name(),
            styles::title_style(),
        )));
        lines.push(Line::from(vec![
            Span::styled(&rank_name, styles::highlight_style()),
            Span::styled(" - Press Esc to go back", styles::muted_style()),
        ]));
    }
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
        // Calculate available width for requirement text (area width minus borders and prefix)
        let text_width = (area.width as usize).saturating_sub(12); // 2 border + 2 prefix + 1 check + 1 space + 5 req_num + 1 padding

        for (i, req) in app.selected_rank_requirements.iter().enumerate() {
            let is_selected = i == app.requirement_selection;
            let check = if req.is_completed() { "✓" } else { "○" };
            let check_style = if req.is_completed() { styles::success_style() } else { styles::muted_style() };

            let req_num = req.number();
            let req_text = truncate(&req.text(), text_width.min(50));
            // Pad to full width to clear any artifacts from previous renders
            let req_text_padded = format!("{:<width$}", req_text, width = text_width);

            // Highlight selected requirement
            let prefix = if is_selected { "▶ " } else { "  " };
            let text_style = if is_selected { styles::selected_style() } else { styles::list_item_style() };

            lines.push(Line::from(vec![
                Span::raw(prefix),
                Span::styled(check, check_style),
                Span::raw(" "),
                Span::styled(format!("{:<5}", req_num), styles::highlight_style()),
                Span::styled(req_text_padded, text_style),
            ]));

            // Show completion date if completed and selected
            if is_selected && req.is_completed() {
                if let Some(ref date) = req.date_completed {
                    if !date.is_empty() {
                        lines.push(Line::from(vec![
                            Span::raw("          "),
                            Span::styled("Completed: ", styles::muted_style()),
                            Span::styled(format!("{:<width$}", date.chars().take(10).collect::<String>(), width = text_width.saturating_sub(10)), styles::highlight_style()),
                        ]));
                    }
                }
            }
        }
    }

    let block = Block::default()
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
                youth.display_name(),
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
        lines.push(Line::from(Span::styled(
            youth.display_name(),
            styles::title_style(),
        )));
        // Show badge name with version if available
        let badge_display = if let Some(ref version) = app.selected_badge_version {
            format!("{} ({})", badge_name, version)
        } else {
            badge_name.clone()
        };
        lines.push(Line::from(vec![
            Span::styled(badge_display, styles::highlight_style()),
            Span::styled(" - Press Esc to go back", styles::muted_style()),
        ]));
    }
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
        // Calculate available width for requirement text (area width minus borders and prefix)
        let text_width = (area.width as usize).saturating_sub(12); // 2 border + 2 prefix + 1 check + 1 space + 5 req_num + 1 padding

        for (i, req) in app.selected_badge_requirements.iter().enumerate() {
            let is_selected = i == app.requirement_selection;
            let check = if req.is_completed() { "✓" } else { "○" };
            let check_style = if req.is_completed() { styles::success_style() } else { styles::muted_style() };

            let req_num = req.number();
            // Summarize to first sentence and truncate
            let raw_text = req.text();
            let summary = summarize_requirement(&raw_text);
            let req_text = truncate(&summary, text_width.min(45));
            // Pad to full width to clear any artifacts from previous renders
            let req_text_padded = format!("{:<width$}", req_text, width = text_width);

            let prefix = if is_selected { "▶ " } else { "  " };
            let text_style = if is_selected { styles::selected_style() } else { styles::list_item_style() };

            lines.push(Line::from(vec![
                Span::raw(prefix),
                Span::styled(check, check_style),
                Span::raw(" "),
                Span::styled(format!("{:<5}", req_num), styles::highlight_style()),
                Span::styled(req_text_padded, text_style),
            ]));

            // Show completion date if completed and selected
            if is_selected && req.is_completed() {
                if let Some(ref date) = req.date_completed {
                    if !date.is_empty() {
                        lines.push(Line::from(vec![
                            Span::raw("          "),
                            Span::styled("Completed: ", styles::muted_style()),
                            Span::styled(format!("{:<width$}", date.chars().take(10).collect::<String>(), width = text_width.saturating_sub(10)), styles::highlight_style()),
                        ]));
                    }
                }
            }
        }
    }

    let block = Block::default()
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
    // Also clean up multiple spaces and newlines
    result.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Summarize a requirement - use AI summary if available, otherwise truncate
fn summarize_requirement(s: &str) -> String {
    // First, try to get an AI-generated summary
    if let Some(summary) = crate::summaries::get_summary(s) {
        return summary.to_string();
    }

    // Fall back to basic summarization
    let clean = strip_html(s);

    // If already short, return as-is
    if clean.len() <= 50 {
        return clean;
    }

    // Try to find a natural break point (first sentence or clause)
    // Look for ". " (end of sentence) or ", " (clause) or " and " or " or "
    let break_points = [". ", ", and ", ", or ", "; "];
    let mut best_pos = None;

    for bp in break_points {
        if let Some(pos) = clean.find(bp) {
            // Only use if it creates a reasonable summary (20-80 chars)
            if pos >= 20 && pos <= 80 {
                best_pos = Some(pos);
                break;
            }
        }
    }

    if let Some(pos) = best_pos {
        let mut result = clean[..pos].to_string();
        // Add ellipsis if we truncated
        if !result.ends_with('.') {
            result.push_str("...");
        }
        result
    } else {
        // No good break point, just use first 50 chars
        clean
    }
}

/// Render the Adults tab
pub fn render_adults(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    render_adult_table(frame, app, chunks[0]);
    render_adult_detail(frame, app, chunks[1]);
}

fn render_adult_table(frame: &mut Frame, app: &App, area: Rect) {
    let header_cells = [
        Cell::from("Name"),
        Cell::from("Position"),
    ];

    let header = Row::new(header_cells)
        .style(styles::title_style())
        .height(1);

    let rows: Vec<Row> = app.adults.iter().enumerate().map(|(i, adult)| {
        let style = if i == app.patrol_selection {
            styles::selected_style()
        } else {
            styles::list_item_style()
        };

        let name = adult.display_name();
        let position = adult.role();

        Row::new(vec![
            Cell::from(name),
            Cell::from(position),
        ]).style(style)
    }).collect();

    let widths = [
        Constraint::Percentage(38), // Name - same width as Scouts tab
        Constraint::Fill(1),        // Position
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .title(format!(" Adults ({}) ", app.adults.len()))
                .title_style(styles::title_style())
                .borders(Borders::ALL)
                .border_style(styles::border_style(true))
        )
        .row_highlight_style(styles::selected_style());

    let mut state = TableState::default();
    state.select(Some(app.patrol_selection));

    frame.render_stateful_widget(table, area, &mut state);
}

fn render_adult_detail(frame: &mut Frame, app: &App, area: Rect) {
    let selected = app.adults.get(app.patrol_selection);

    let content = match selected {
        Some(adult) => {
            let mut lines = vec![];

            // Name
            lines.push(Line::from(Span::styled(
                adult.display_name_full(),
                styles::title_style(),
            )));
            lines.push(Line::from(""));

            // Unit Info section
            lines.push(Line::from(Span::styled("Unit Info", styles::highlight_style())));

            lines.push(Line::from(vec![
                Span::styled("Position:   ", styles::muted_style()),
                Span::raw(adult.role()),
            ]));

            // Membership status from registrarInfo
            if let Some(ref reg_info) = adult.registrar_info {
                if let Some(ref exp_date) = reg_info.registration_expire_dt {
                    if let Ok(date) = chrono::NaiveDate::parse_from_str(&exp_date[..10], "%Y-%m-%d") {
                        let today = chrono::Utc::now().date_naive();
                        let formatted_date = date.format("%b %d, %Y").to_string();

                        let (status_text, status_style) = if date < today {
                            (format!("Expired {}", formatted_date), styles::error_style())
                        } else {
                            (format!("Expires {}", formatted_date), styles::success_style())
                        };

                        lines.push(Line::from(vec![
                            Span::styled("Membership: ", styles::muted_style()),
                            Span::styled(status_text, status_style),
                        ]));
                    }
                }
            }

            let bsa_id = adult.member_id.as_deref().unwrap_or("-");
            lines.push(Line::from(vec![
                Span::styled("BSA ID:     ", styles::muted_style()),
                Span::raw(bsa_id),
            ]));

            lines.push(Line::from(""));

            // Training section
            lines.push(Line::from(Span::styled("Training", styles::highlight_style())));

            let (ypt_text, ypt_style) = if let Some(ref exp_str) = adult.ypt_expired_date {
                if let Ok(exp_date) = NaiveDate::parse_from_str(exp_str, "%Y-%m-%d") {
                    let today = Utc::now().date_naive();
                    let formatted_date = exp_date.format("%b %d, %Y").to_string();

                    if exp_date < today {
                        (format!("Expired {}", formatted_date), styles::error_style())
                    } else if (exp_date - today).num_days() < 90 {
                        (format!("Expires {}", formatted_date), styles::error_style())
                    } else {
                        (format!("Expires {}", formatted_date), styles::success_style())
                    }
                } else {
                    ("-".to_string(), styles::muted_style())
                }
            } else {
                ("-".to_string(), styles::muted_style())
            };
            lines.push(Line::from(vec![
                Span::styled("YPT:      ", styles::muted_style()),
                Span::styled(ypt_text, ypt_style),
            ]));

            let (trained_text, trained_style) = match adult.position_trained.as_deref() {
                Some("Trained") => ("Trained", styles::success_style()),
                Some("Not Trained") => ("Not Trained", styles::error_style()),
                _ => ("-", styles::muted_style()),
            };
            lines.push(Line::from(vec![
                Span::styled("Position: ", styles::muted_style()),
                Span::styled(trained_text, trained_style),
            ]));

            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled("Contact", styles::highlight_style())));

            // Phone
            if let Some(phone) = adult.phone() {
                lines.push(Line::from(vec![
                    Span::styled("Phone:   ", styles::muted_style()),
                    Span::raw(phone),
                ]));
            }

            // Email
            if let Some(email) = adult.email() {
                lines.push(Line::from(vec![
                    Span::styled("Email:   ", styles::muted_style()),
                    Span::raw(email),
                ]));
            }

            // Address
            let addr_line1 = adult.primary_address_info.as_ref()
                .and_then(|a| a.address1.clone())
                .filter(|a| !a.trim().is_empty())
                .unwrap_or_else(|| "-".to_string());
            lines.push(Line::from(vec![
                Span::styled("Address: ", styles::muted_style()),
                Span::raw(addr_line1),
            ]));

            // City, State ZIP on second line
            if let Some(ref addr_info) = adult.primary_address_info {
                let addr_line2 = addr_info.city_state().map(|cs| {
                    format!("{} {}", cs, addr_info.zip_code.as_deref().unwrap_or(""))
                }).unwrap_or_default();

                if !addr_line2.is_empty() {
                    lines.push(Line::from(vec![
                        Span::raw("         "), // 9 spaces to align with "Address: "
                        Span::raw(addr_line2),
                    ]));
                }
            }

            lines
        }
        None => vec![Line::from(Span::styled(
            "No adult selected",
            styles::muted_style(),
        ))],
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(styles::border_style(false));

    let paragraph = Paragraph::new(content).block(block);
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
