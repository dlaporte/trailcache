use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::{App, ScoutRank};
use crate::ui::styles;
use trailcache_core::models::{patrol_rank_breakdown, DISPLAY_NOT_TRAINED, RenewalStats, TrainingStats};
use trailcache_core::utils::strip_url_scheme;

pub fn render(frame: &mut Frame, app: &mut App, area: Rect) {
    // Vertical layout:
    // 1. Unit info (full width)
    // 2. Scouts | Patrols (50/50)
    // 3. Renewals | Training (50/50)
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(15),    // Unit section
            Constraint::Percentage(45), // Scouts/Patrols row
            Constraint::Percentage(40), // Renewals/Training row
        ])
        .split(area);

    // Unit at top (full width)
    render_troop_info(frame, app, main_chunks[0]);

    // Middle row: Scouts | Patrols (50/50)
    let middle_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(main_chunks[1]);

    render_positions(frame, app, middle_chunks[0]);
    render_patrols(frame, app, middle_chunks[1]);

    // Bottom row: Renewals | Training (50/50)
    let bottom_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(main_chunks[2]);

    render_renewals(frame, app, bottom_chunks[0]);
    render_training(frame, app, bottom_chunks[1]);
}

fn render_patrols(frame: &mut Frame, app: &mut App, area: Rect) {
    let mut lines = vec![];

    // Get patrols with their scouts and rank breakdown
    let patrol_data = patrol_rank_breakdown(&app.youth);

    // Sort patrols alphabetically by name for stable ordering
    let mut patrol_names: Vec<&String> = patrol_data.keys().collect();
    patrol_names.sort();

    for patrol_name in patrol_names {
        if let Some(breakdown) = patrol_data.get(patrol_name) {
            // Patrol name with count
            lines.push(Line::from(vec![
                Span::styled(patrol_name.to_string(), styles::highlight_style()),
                Span::styled(format!(" ({})", breakdown.member_count), styles::muted_style()),
            ]));

            // Rank breakdown indented using canonical rank order
            for rank in ScoutRank::all_display_order() {
                let key = rank.display_name();
                let display_name = rank.display_name();
                if let Some(&rc) = breakdown.rank_counts.get(key) {
                    if rc > 0 {
                        lines.push(Line::from(vec![
                            Span::raw("  "),
                            Span::styled(format!("{}: {}", display_name, rc), styles::muted_style()),
                        ]));
                    }
                }
            }
        }
    }

    if lines.is_empty() {
        lines.push(Line::from(Span::styled("No patrol data", styles::muted_style())));
    }

    let block = Block::default()
        .title(" Patrols ")
        .title_style(styles::title_style())
        .borders(Borders::ALL)
        .border_style(styles::border_style(true));

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

fn render_positions(frame: &mut Frame, app: &mut App, area: Rect) {
    let mut lines = vec![];

    let positions = trailcache_core::models::youth_position_list(&app.youth);

    if positions.is_empty() {
        lines.push(Line::from(Span::styled("No positions assigned", styles::muted_style())));
    } else {
        // Find the longest display position name for alignment
        let max_pos_len = positions.iter().map(|(display, _)| display.len()).max().unwrap_or(0);

        for (display_pos, holder) in &positions {
            lines.push(Line::from(vec![
                Span::styled(format!("{:<width$}", display_pos, width = max_pos_len + 2), styles::list_item_style()),
                Span::styled(holder, styles::highlight_style()),
            ]));
        }
    }

    let block = Block::default()
        .title(" Positions ")
        .title_style(styles::title_style())
        .borders(Borders::ALL)
        .border_style(styles::border_style(false));

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

fn render_training(frame: &mut Frame, app: &mut App, area: Rect) {
    let mut lines = vec![];

    let training_stats = TrainingStats::from_adults(&app.adults);

    // YPT Summary
    lines.push(Line::from(Span::styled("Youth Protection", styles::highlight_style())));
    lines.push(Line::from(vec![
        Span::styled("  Current: ", styles::muted_style()),
        Span::styled(format!("{}", training_stats.ypt_current), styles::success_style()),
        Span::styled("  Expiring/Expired: ", styles::muted_style()),
        if training_stats.ypt_expiring + training_stats.ypt_expired > 0 {
            Span::styled(format!("{}", training_stats.ypt_expiring + training_stats.ypt_expired), styles::error_style())
        } else {
            Span::styled("0", styles::success_style())
        },
    ]));

    // Calculate max name length across both training sections for alignment
    let ypt_max = training_stats.ypt_issues.iter().map(|(n, _)| n.len()).max().unwrap_or(0);
    let pos_max = training_stats.position_not_trained_list.iter().map(|n| n.len()).max().unwrap_or(0);
    let training_name_width = ypt_max.max(pos_max) + 2;

    // List expired/expiring - show all with aligned values
    for (name, status) in training_stats.ypt_issues.iter() {
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(format!("{:<width$}", name, width = training_name_width), styles::list_item_style()),
            Span::styled(status, styles::error_style()),
        ]));
    }

    lines.push(Line::from(""));

    // Position-Specific Training
    lines.push(Line::from(Span::styled("Position Training", styles::highlight_style())));
    lines.push(Line::from(vec![
        Span::styled("  Trained: ", styles::muted_style()),
        Span::styled(format!("{}", training_stats.position_trained), styles::success_style()),
        Span::styled("  Not Trained: ", styles::muted_style()),
        if training_stats.position_not_trained > 0 {
            Span::styled(format!("{}", training_stats.position_not_trained), styles::error_style())
        } else {
            Span::styled("0", styles::success_style())
        },
    ]));

    // List not trained adults - show all with aligned values
    for name in training_stats.position_not_trained_list.iter() {
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(format!("{:<width$}", name, width = training_name_width), styles::list_item_style()),
            Span::styled(DISPLAY_NOT_TRAINED, styles::error_style()),
        ]));
    }

    let block = Block::default()
        .title(" Training ")
        .title_style(styles::title_style())
        .borders(Borders::ALL)
        .border_style(styles::border_style(false));

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

fn render_renewals(frame: &mut Frame, app: &mut App, area: Rect) {
    let mut lines = vec![];

    let renewal_stats = RenewalStats::compute(&app.youth, &app.adults);

    // Calculate max name length across both scout and adult sections for alignment
    let scout_max = renewal_stats.scout_issues.iter().map(|(n, _)| n.len()).max().unwrap_or(0);
    let adult_max = renewal_stats.adult_issues.iter().map(|(n, _)| n.len()).max().unwrap_or(0);
    let renewal_name_width = scout_max.max(adult_max) + 2;

    // Scouts
    lines.push(Line::from(Span::styled("Scouts", styles::highlight_style())));
    lines.push(Line::from(vec![
        Span::styled("  Current: ", styles::muted_style()),
        Span::styled(format!("{}", renewal_stats.scouts_current), styles::success_style()),
        Span::styled("  Expiring/Expired: ", styles::muted_style()),
        if renewal_stats.scouts_expiring + renewal_stats.scouts_expired > 0 {
            Span::styled(format!("{}", renewal_stats.scouts_expiring + renewal_stats.scouts_expired), styles::error_style())
        } else {
            Span::styled("0", styles::success_style())
        },
    ]));

    // List all scout issues with aligned values
    for (name, date) in renewal_stats.scout_issues.iter() {
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(format!("{:<width$}", name, width = renewal_name_width), styles::list_item_style()),
            Span::styled(date, styles::error_style()),
        ]));
    }

    lines.push(Line::from(""));

    // Adults
    lines.push(Line::from(Span::styled("Adults", styles::highlight_style())));
    lines.push(Line::from(vec![
        Span::styled("  Current: ", styles::muted_style()),
        Span::styled(format!("{}", renewal_stats.adults_current), styles::success_style()),
        Span::styled("  Expiring/Expired: ", styles::muted_style()),
        if renewal_stats.adults_expiring + renewal_stats.adults_expired > 0 {
            Span::styled(format!("{}", renewal_stats.adults_expiring + renewal_stats.adults_expired), styles::error_style())
        } else {
            Span::styled("0", styles::success_style())
        },
    ]));

    // List all adult issues with aligned values
    for (name, date) in renewal_stats.adult_issues.iter() {
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(format!("{:<width$}", name, width = renewal_name_width), styles::list_item_style()),
            Span::styled(date, styles::error_style()),
        ]));
    }

    let block = Block::default()
        .title(" Renewals ")
        .title_style(styles::title_style())
        .borders(Borders::ALL)
        .border_style(styles::border_style(false));

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

fn render_troop_info(frame: &mut Frame, app: &mut App, area: Rect) {
    // Use unit name as title, fallback to "Unit"
    let title = app.unit_info.as_ref()
        .and_then(|u| u.name.as_ref())
        .map(|n| format!(" {} ", n))
        .unwrap_or_else(|| " Unit ".to_string());

    let block = Block::default()
        .title(title)
        .title_style(styles::title_style())
        .borders(Borders::ALL)
        .border_style(styles::border_style(false));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Split inner area into two columns
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner);

    render_unit_left(frame, app, chunks[0]);
    render_unit_right(frame, app, chunks[1]);
}

fn render_unit_left(frame: &mut Frame, app: &mut App, area: Rect) {
    let mut lines = vec![];

    // Key 3 from API
    if let Some(ref sm) = app.key3.scoutmaster {
        lines.push(Line::from(vec![
            Span::styled("Scoutmaster:     ", styles::muted_style()),
            Span::raw(sm.full_name()),
        ]));
    }
    if let Some(ref cc) = app.key3.committee_chair {
        lines.push(Line::from(vec![
            Span::styled("Committee Chair: ", styles::muted_style()),
            Span::raw(cc.full_name()),
        ]));
    }
    if let Some(ref cor) = app.key3.charter_org_rep {
        lines.push(Line::from(vec![
            Span::styled("Charter Org Rep: ", styles::muted_style()),
            Span::raw(cor.full_name()),
        ]));
    }

    if app.key3.scoutmaster.is_some() || app.key3.committee_chair.is_some() || app.key3.charter_org_rep.is_some() {
        lines.push(Line::from(""));
    }

    // Charter info
    if let Some(ref unit_info) = app.unit_info {
        if let Some(ref org_name) = unit_info.charter_org_name {
            if !org_name.is_empty() {
                lines.push(Line::from(vec![
                    Span::styled("Chartered Org:   ", styles::muted_style()),
                    Span::raw(org_name.as_str()),
                ]));
            }
        }

        // Charter Status
        if let Some(ref status_text) = unit_info.charter_status_display {
            let status_style = if unit_info.charter_expired.unwrap_or(false) {
                styles::error_style()
            } else {
                styles::success_style()
            };
            lines.push(Line::from(vec![
                Span::styled("Charter Status:  ", styles::muted_style()),
                Span::styled(status_text.as_str(), status_style),
            ]));
        }

        lines.push(Line::from(""));

        // Council
        if let Some(ref council) = unit_info.council_name {
            if !council.is_empty() {
                lines.push(Line::from(vec![
                    Span::styled("Council:         ", styles::muted_style()),
                    Span::raw(council.as_str()),
                ]));
            }
        }

        // District
        if let Some(ref district) = unit_info.district_name {
            if !district.is_empty() {
                lines.push(Line::from(vec![
                    Span::styled("District:        ", styles::muted_style()),
                    Span::raw(district.as_str()),
                ]));
            }
        }
    }

    // Commissioners
    for commissioner in &app.commissioners {
        lines.push(Line::from(vec![
            Span::styled("Commissioner:    ", styles::muted_style()),
            Span::raw(commissioner.full_name()),
        ]));
    }

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, area);
}

fn render_unit_right(frame: &mut Frame, app: &mut App, area: Rect) {
    let mut lines = vec![];

    if let Some(ref unit_info) = app.unit_info {
        // Contact person
        if let Some(contact) = unit_info.contacts.first() {
            lines.push(Line::from(vec![
                Span::styled("Contact:      ", styles::muted_style()),
                Span::raw(contact.full_name()),
            ]));
            if let Some(ref email) = contact.email {
                if !email.is_empty() {
                    lines.push(Line::from(vec![
                        Span::styled("Email:        ", styles::muted_style()),
                        Span::raw(email.as_str()),
                    ]));
                }
            }
            if let Some(ref phone) = contact.phone {
                if !phone.is_empty() {
                    lines.push(Line::from(vec![
                        Span::styled("Phone:        ", styles::muted_style()),
                        Span::raw(phone.as_str()),
                    ]));
                }
            }
            lines.push(Line::from(""));
        }

        // Website
        if let Some(ref website) = unit_info.website {
            if !website.is_empty() {
                lines.push(Line::from(vec![
                    Span::styled("Website:      ", styles::muted_style()),
                    Span::raw(strip_url_scheme(website)),
                ]));
            }
        }

        // Registration URL
        if let Some(ref reg_url) = unit_info.registration_url {
            if !reg_url.is_empty() {
                lines.push(Line::from(vec![
                    Span::styled("Registration: ", styles::muted_style()),
                    Span::raw(reg_url.as_str()),
                ]));
            }
        }

        lines.push(Line::from(""));

        // Meeting location
        if let Some(ref meeting_loc) = unit_info.meeting_location {
            if meeting_loc.formatted().is_some() {
                lines.push(Line::from(vec![
                    Span::styled("Meeting Location:", styles::muted_style()),
                ]));

                // Show address line 1 if present
                if let Some(ref addr1) = meeting_loc.address_line1 {
                    if !addr1.is_empty() {
                        lines.push(Line::from(vec![
                            Span::raw("  "),
                            Span::raw(addr1.as_str()),
                        ]));
                    }
                }

                // Show city, state, zip
                let mut city_line = String::new();
                if let Some(ref city) = meeting_loc.city {
                    city_line.push_str(city);
                }
                if let Some(ref state) = meeting_loc.state {
                    if !city_line.is_empty() {
                        city_line.push_str(", ");
                    }
                    city_line.push_str(state);
                }
                if let Some(ref zip) = meeting_loc.zip {
                    if !city_line.is_empty() {
                        city_line.push(' ');
                    }
                    city_line.push_str(zip);
                }

                if !city_line.is_empty() {
                    lines.push(Line::from(vec![
                        Span::raw("  "),
                        Span::raw(city_line),
                    ]));
                }
            }
        }
    }

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, area);
}

// Helper functions removed — training stats, renewal stats, and patrol rank breakdown
// are now computed by core: TrainingStats::from_adults(), RenewalStats::compute(),
// patrol_rank_breakdown().

