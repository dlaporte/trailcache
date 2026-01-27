use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
    Frame,
};

use crate::app::{App, EventDetailView, Focus};
use crate::models::RsvpStatus;
use crate::ui::styles;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    render_event_list(frame, app, chunks[0]);
    render_event_detail(frame, app, chunks[1]);
}

fn render_event_list(frame: &mut Frame, app: &App, area: Rect) {
    let focused = matches!(app.focus, Focus::List);

    // Header row
    let header_cells = [
        Cell::from("Name"),
        Cell::from("Date"),
        Cell::from("Location"),
        Cell::from("Type"),
    ];
    let header = Row::new(header_cells)
        .style(styles::title_style())
        .height(1);

    let sorted_events = app.get_sorted_events();

    // Data rows
    let rows: Vec<Row> = sorted_events
        .iter()
        .enumerate()
        .map(|(i, event)| {
            let style = if i == app.event_selection {
                styles::selected_style()
            } else {
                styles::list_item_style()
            };

            let name = &event.name;
            let date = event.formatted_date();
            let location = event.location.as_deref().unwrap_or("-");
            let event_type = event.derived_type();

            Row::new(vec![
                Cell::from(name.as_str()),
                Cell::from(date),
                Cell::from(location),
                Cell::from(event_type),
            ]).style(style)
        })
        .collect();

    // Column widths: Name (38%), Date, Location, Type
    let widths = [
        Constraint::Percentage(38),  // Name - same as Scouts/Adults tabs
        Constraint::Length(20),      // Date: "Jan 26, 2026" + generous padding
        Constraint::Fill(1),         // Location
        Constraint::Length(12),      // Type
    ];

    let sort_help = "[n]ame [d]ate [l]ocation [t]ype";
    let title = format!(" Events ({}) - {} ", app.events.len(), sort_help);

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
    state.select(Some(app.event_selection));

    frame.render_stateful_widget(table, area, &mut state);
}

fn render_event_detail(frame: &mut Frame, app: &App, area: Rect) {
    let focused = matches!(app.focus, Focus::Detail);

    match app.event_detail_view {
        EventDetailView::Details => render_details_view(frame, app, area, focused),
        EventDetailView::Rsvp => render_rsvp_view(frame, app, area, focused),
    }
}

fn render_details_view(frame: &mut Frame, app: &App, area: Rect, focused: bool) {
    let sorted_events = app.get_sorted_events();
    let selected = sorted_events.get(app.event_selection).copied();

    let content = match selected {
        Some(event) => {
            let mut lines = vec![];

            // Event name
            lines.push(Line::from(Span::styled(
                &event.name,
                styles::title_style(),
            )));
            lines.push(Line::from(""));

            // Start date/time
            lines.push(Line::from(vec![
                Span::styled("Start:    ", styles::muted_style()),
                Span::raw(event.formatted_start_datetime()),
            ]));

            // End date/time
            lines.push(Line::from(vec![
                Span::styled("End:      ", styles::muted_style()),
                Span::raw(event.formatted_end_datetime()),
            ]));

            // Location
            if let Some(ref location) = event.location {
                if !location.is_empty() {
                    lines.push(Line::from(vec![
                        Span::styled("Location: ", styles::muted_style()),
                        Span::raw(location.clone()),
                    ]));
                }
            }

            // Event type
            lines.push(Line::from(vec![
                Span::styled("Type:     ", styles::muted_style()),
                Span::raw(event.derived_type()),
            ]));

            lines.push(Line::from(""));

            // Description (with HTML stripped)
            if let Some(ref desc) = event.description {
                if !desc.is_empty() {
                    let clean_desc = strip_html(desc);
                    let trimmed = clean_desc.trim();
                    if !trimmed.is_empty() {
                        lines.push(Line::from(Span::styled("Description", styles::highlight_style())));
                        for line in wrap_text(trimmed, (area.width as usize).saturating_sub(4)) {
                            lines.push(Line::from(line));
                        }
                        lines.push(Line::from(""));
                    }
                }
            }

            // RSVP section
            lines.push(Line::from(Span::styled("RSVP", styles::highlight_style())));

            if event.rsvp {
                // RSVP is required - show counts from invited_users
                let adult_yes = event.invited_users.iter()
                    .filter(|u| u.is_adult)
                    .filter(|u| matches!(u.status(), RsvpStatus::Going))
                    .count();
                let adult_no = event.invited_users.iter()
                    .filter(|u| u.is_adult)
                    .filter(|u| matches!(u.status(), RsvpStatus::NotGoing))
                    .count();
                let scout_yes = event.invited_users.iter()
                    .filter(|u| !u.is_adult)
                    .filter(|u| matches!(u.status(), RsvpStatus::Going))
                    .count();
                let scout_no = event.invited_users.iter()
                    .filter(|u| !u.is_adult)
                    .filter(|u| matches!(u.status(), RsvpStatus::NotGoing))
                    .count();

                lines.push(Line::from(vec![
                    Span::styled("  Adults: ", styles::muted_style()),
                    Span::styled(format!("{}", adult_yes), styles::success_style()),
                    Span::styled(" yes, ", styles::muted_style()),
                    Span::styled(format!("{}", adult_no), styles::error_style()),
                    Span::styled(" no", styles::muted_style()),
                ]));

                lines.push(Line::from(vec![
                    Span::styled("  Scouts: ", styles::muted_style()),
                    Span::styled(format!("{}", scout_yes), styles::success_style()),
                    Span::styled(" yes, ", styles::muted_style()),
                    Span::styled(format!("{}", scout_no), styles::error_style()),
                    Span::styled(" no", styles::muted_style()),
                ]));
            } else {
                lines.push(Line::from(Span::styled(
                    "  Not Required",
                    styles::muted_style(),
                )));
            }

            lines.push(Line::from(""));

            // Permission slips
            lines.push(Line::from(Span::styled("Permission Slips", styles::highlight_style())));
            if event.slips_required {
                lines.push(Line::from(Span::styled(
                    "  Required",
                    styles::highlight_style(),
                )));
            } else {
                lines.push(Line::from(Span::styled(
                    "  Not Required",
                    styles::muted_style(),
                )));
            }

            lines
        }
        None => vec![Line::from(Span::styled(
            "Select an event from the list",
            styles::muted_style(),
        ))],
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(styles::border_style(focused));

    let paragraph = Paragraph::new(content).block(block);
    frame.render_widget(paragraph, area);
}

fn render_rsvp_view(frame: &mut Frame, app: &App, area: Rect, focused: bool) {
    let sorted_events = app.get_sorted_events();
    let selected = sorted_events.get(app.event_selection).copied();

    let content = match selected {
        Some(event) => {
            let mut lines = vec![];

            // Event name
            lines.push(Line::from(Span::styled(
                &event.name,
                styles::title_style(),
            )));
            lines.push(Line::from(Span::styled(
                "Press Esc or 'd' to go back",
                styles::muted_style(),
            )));
            lines.push(Line::from(""));

            // Filter to only Yes and No (exclude NoResponse)
            let adults: Vec<_> = event.invited_users.iter()
                .filter(|u| u.is_adult)
                .filter(|u| matches!(u.status(), RsvpStatus::Going | RsvpStatus::NotGoing))
                .collect();

            let scouts: Vec<_> = event.invited_users.iter()
                .filter(|u| !u.is_adult)
                .filter(|u| matches!(u.status(), RsvpStatus::Going | RsvpStatus::NotGoing))
                .collect();

            if adults.is_empty() && scouts.is_empty() {
                lines.push(Line::from(Span::styled(
                    "No RSVPs yet",
                    styles::muted_style(),
                )));
            } else {

                // Adults section
                if !adults.is_empty() {
                    lines.push(Line::from(Span::styled(
                        format!("Adults ({})", adults.len()),
                        styles::highlight_style(),
                    )));

                    for guest in &adults {
                        let (status_char, status_style) = match guest.status() {
                            RsvpStatus::Going => ("Y", styles::success_style()),
                            RsvpStatus::NotGoing => ("N", styles::error_style()),
                            _ => ("-", styles::muted_style()),
                        };

                        lines.push(Line::from(vec![
                            Span::styled(format!("  [{}] ", status_char), status_style),
                            Span::raw(guest.display_name()),
                        ]));
                    }
                    lines.push(Line::from(""));
                }

                // Scouts section
                if !scouts.is_empty() {
                    lines.push(Line::from(Span::styled(
                        format!("Scouts ({})", scouts.len()),
                        styles::highlight_style(),
                    )));

                    for guest in &scouts {
                        let (status_char, status_style) = match guest.status() {
                            RsvpStatus::Going => ("Y", styles::success_style()),
                            RsvpStatus::NotGoing => ("N", styles::error_style()),
                            _ => ("-", styles::muted_style()),
                        };

                        lines.push(Line::from(vec![
                            Span::styled(format!("  [{}] ", status_char), status_style),
                            Span::raw(guest.display_name()),
                        ]));
                    }
                }
            }

            lines
        }
        None => vec![Line::from(Span::styled(
            "Select an event from the list",
            styles::muted_style(),
        ))],
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(styles::border_style(focused));

    let paragraph = Paragraph::new(content).block(block);
    frame.render_widget(paragraph, area);
}

fn wrap_text(s: &str, max_width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in s.split_whitespace() {
        if current_line.is_empty() {
            current_line = word.to_string();
        } else if current_line.len() + 1 + word.len() <= max_width {
            current_line.push(' ');
            current_line.push_str(word);
        } else {
            lines.push(current_line);
            current_line = word.to_string();
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    lines
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

    // Clean up HTML entities
    result
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}
