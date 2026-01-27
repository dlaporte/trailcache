use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::app::{App, Focus};
use crate::ui::styles;
use crate::utils::format::format_phone;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    render_patrol_list(frame, app, chunks[0]);
    render_patrol_members(frame, app, chunks[1]);
}

fn render_patrol_list(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .patrols
        .iter()
        .enumerate()
        .map(|(i, patrol)| {
            let count = patrol.member_count.unwrap_or(0);
            let line = Line::from(format!(
                "{:<20} ({} members)",
                truncate(&patrol.name, 20),
                count
            ));

            let style = if i == app.adults_selection {
                styles::selected_style()
            } else {
                styles::list_item_style()
            };

            ListItem::new(line).style(style)
        })
        .collect();

    let focused = matches!(app.focus, Focus::List);
    let block = Block::default()
        .title(format!(" Patrols ({}) ", app.patrols.len()))
        .title_style(styles::title_style())
        .borders(Borders::ALL)
        .border_style(styles::border_style(focused));

    let list = List::new(items).block(block);

    let mut state = ListState::default();
    state.select(Some(app.adults_selection));

    frame.render_stateful_widget(list, area, &mut state);
}

fn render_patrol_members(frame: &mut Frame, app: &App, area: Rect) {
    let selected_patrol = app.patrols.get(app.adults_selection);

    let focused = matches!(app.focus, Focus::Detail);

    let (title, content) = match selected_patrol {
        Some(patrol) => {
            let members = app.get_patrol_members(&patrol.guid);
            let title = format!(" {} ", patrol.name);

            let mut lines = vec![];

            // Patrol leader info
            if let Some(ref leader_name) = patrol.patrol_leader_name {
                lines.push(Line::from(vec![
                    Span::styled("Patrol Leader: ", styles::highlight_style()),
                    Span::raw(leader_name.clone()),
                ]));
                lines.push(Line::from(""));
            }

            // Members header
            lines.push(Line::from(Span::styled(
                format!("Members ({})", members.len()),
                styles::title_style(),
            )));
            lines.push(Line::from(""));

            // Member list
            for (i, youth) in members.iter().enumerate() {
                let is_leader = youth.is_patrol_leader.unwrap_or(false);

                let leader_badge = if is_leader { " [PL]" } else { "" };

                let phone = youth
                    .phone_number
                    .as_ref()
                    .map(|p| format_phone(p))
                    .unwrap_or_else(|| "No phone".to_string());

                let rank = youth
                    .current_rank
                    .clone()
                    .unwrap_or_else(|| "Scout".to_string());

                let style = if i == app.patrol_member_selection && focused {
                    styles::selected_style()
                } else {
                    styles::list_item_style()
                };

                lines.push(Line::styled(
                    format!(
                        "  {}{} - {} - {}",
                        youth.full_name(),
                        leader_badge,
                        rank,
                        phone
                    ),
                    style,
                ));
            }

            if members.is_empty() {
                lines.push(Line::from(Span::styled(
                    "  No members in this patrol",
                    styles::muted_style(),
                )));
            }

            (title, lines)
        }
        None => (
            " No Patrol Selected ".to_string(),
            vec![Line::from(Span::styled(
                "Select a patrol from the list",
                styles::muted_style(),
            ))],
        ),
    };

    let block = Block::default()
        .title(title)
        .title_style(styles::title_style())
        .borders(Borders::ALL)
        .border_style(styles::border_style(focused));

    let paragraph = Paragraph::new(content).block(block);
    frame.render_widget(paragraph, area);
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}
