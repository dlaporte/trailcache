use chrono::{NaiveDate, Utc};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::collections::HashMap;

use crate::app::{App, ScoutRank};
use crate::ui::styles;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
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

fn render_patrols(frame: &mut Frame, app: &App, area: Rect) {
    let mut lines = vec![];

    // Get patrols with their scouts and rank breakdown
    let patrol_data = get_patrol_rank_breakdown(app);

    // Sort patrols alphabetically by name for stable ordering
    let mut patrol_names: Vec<&String> = patrol_data.keys().collect();
    patrol_names.sort();

    for patrol_name in patrol_names {
        if let Some((count, rank_counts)) = patrol_data.get(patrol_name) {
            // Patrol name with count
            lines.push(Line::from(vec![
                Span::styled(patrol_name.to_string(), styles::highlight_style()),
                Span::styled(format!(" ({})", count), styles::muted_style()),
            ]));

            // Rank breakdown indented
            let rank_order = ["Eagle", "Life", "Star", "First Class", "Second Class", "Tenderfoot", "Scout", "Crossover"];
            for rank in rank_order.iter() {
                // Map "None" to "Crossover" for display
                let display_rank = if *rank == "Crossover" { "None" } else { rank };
                if let Some(rc) = rank_counts.get(display_rank) {
                    if *rc > 0 {
                        lines.push(Line::from(vec![
                            Span::raw("  "),
                            Span::styled(format!("{}: {}", rank, rc), styles::muted_style()),
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

fn render_positions(frame: &mut Frame, app: &App, area: Rect) {
    let mut lines = vec![];

    // Collect all youth with positions of responsibility
    // Tuple: (position_name, display_position (with patrol if applicable), holder_name)
    let mut positions: Vec<(&str, String, String)> = Vec::new();

    for youth in &app.youth {
        if let Some(ref pos) = youth.position {
            // Skip "Scouts BSA" (just means member) and "Scout"
            if !pos.is_empty() && pos != "Scouts BSA" && pos != "Scout" {
                // For patrol leaders and assistant patrol leaders, include the patrol name
                let display_pos = if (pos == "Patrol Leader" || pos == "Assistant Patrol Leader")
                    && youth.patrol_name.as_ref().map(|p| !p.is_empty()).unwrap_or(false)
                {
                    format!("{} ({})", pos, youth.patrol_name.as_ref().unwrap())
                } else {
                    pos.clone()
                };
                positions.push((pos.as_str(), display_pos, youth.display_name()));
            }
        }
    }

    // Define position priority order (SPL first, then ASPL, then others alphabetically)
    let priority_order = [
        "Senior Patrol Leader",
        "Assistant Senior Patrol Leader",
        "Troop Guide",
        "Patrol Leader",
        "Assistant Patrol Leader",
        "Quartermaster",
        "Scribe",
        "Historian",
        "Librarian",
        "Chaplain Aide",
        "Outdoor Ethics Guide",
        "Den Chief",
        "Instructor",
        "Junior Assistant Scoutmaster",
        "Bugler",
    ];

    // Sort positions by priority order, then alphabetically by display position, then by holder name
    positions.sort_by(|a, b| {
        let a_priority = priority_order.iter().position(|&p| p == a.0).unwrap_or(999);
        let b_priority = priority_order.iter().position(|&p| p == b.0).unwrap_or(999);

        if a_priority != b_priority {
            a_priority.cmp(&b_priority)
        } else if a.1 != b.1 {
            a.1.cmp(&b.1)
        } else {
            a.2.cmp(&b.2)
        }
    });

    if positions.is_empty() {
        lines.push(Line::from(Span::styled("No positions assigned", styles::muted_style())));
    } else {
        // Find the longest display position name for alignment
        let max_pos_len = positions.iter().map(|(_, display, _)| display.len()).max().unwrap_or(0);

        for (_, display_pos, holder) in &positions {
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

fn render_training(frame: &mut Frame, app: &App, area: Rect) {
    let mut lines = vec![];

    let training_stats = calculate_training_stats(app);

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
            Span::styled("Not Trained", styles::error_style()),
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

fn render_renewals(frame: &mut Frame, app: &App, area: Rect) {
    let mut lines = vec![];

    let renewal_stats = calculate_renewal_stats(app);

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

fn render_troop_info(frame: &mut Frame, app: &App, area: Rect) {
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

fn render_unit_left(frame: &mut Frame, app: &App, area: Rect) {
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

        // Charter Status - show status (if available) and expiration date
        if let Some(ref exp_date) = unit_info.charter_expiry {
            if exp_date.len() >= 10 {
                if let Ok(date) = chrono::NaiveDate::parse_from_str(&exp_date[..10], "%Y-%m-%d") {
                    let today = chrono::Utc::now().date_naive();
                    let formatted_date = date.format("%b %d, %Y").to_string();

                    let (status_text, status_style) = if date < today {
                        (format!("Expired {}", formatted_date), styles::error_style())
                    } else {
                        (format!("Expires {}", formatted_date), styles::success_style())
                    };

                    lines.push(Line::from(vec![
                        Span::styled("Charter Status:  ", styles::muted_style()),
                        Span::styled(status_text, status_style),
                    ]));
                }
            }
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

fn render_unit_right(frame: &mut Frame, app: &App, area: Rect) {
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
                let clean_url = website
                    .strip_prefix("https://")
                    .unwrap_or(website)
                    .strip_prefix("http://")
                    .unwrap_or(website);
                lines.push(Line::from(vec![
                    Span::styled("Website:      ", styles::muted_style()),
                    Span::raw(clean_url),
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

// ============ Helper functions ============

fn get_patrol_rank_breakdown(app: &App) -> HashMap<String, (usize, HashMap<String, usize>)> {
    let mut result: HashMap<String, (usize, HashMap<String, usize>)> = HashMap::new();

    for youth in &app.youth {
        let patrol = youth.patrol_name.clone().unwrap_or_else(|| "Unassigned".to_string());
        if patrol.is_empty() {
            continue;
        }

        // Use ScoutRank enum for consistent rank normalization
        let scout_rank = ScoutRank::from_str(youth.current_rank.as_deref());
        // Map "Crossover" back to "None" for data consistency with existing code
        let rank = if scout_rank == ScoutRank::Unknown {
            "None"
        } else {
            scout_rank.display_name()
        };

        let entry = result.entry(patrol).or_insert_with(|| (0, HashMap::new()));
        entry.0 += 1;
        *entry.1.entry(rank.to_string()).or_insert(0) += 1;
    }

    result
}

struct TrainingStats {
    ypt_current: usize,
    ypt_expiring: usize,
    ypt_expired: usize,
    ypt_issues: Vec<(String, String)>, // (name, status)
    position_trained: usize,
    position_not_trained: usize,
    position_not_trained_list: Vec<String>, // names of those not trained
}

fn calculate_training_stats(app: &App) -> TrainingStats {
    let today = Utc::now().date_naive();
    let ninety_days_later = today + chrono::Duration::days(90);

    let mut ypt_current = 0;
    let mut ypt_expiring = 0;
    let mut ypt_expired = 0;
    let mut ypt_issues = Vec::new();
    let mut position_trained = 0;
    let mut position_not_trained = 0;
    let mut position_not_trained_list = Vec::new();

    for adult in &app.adults {
        // YPT status
        if let Some(ref exp_str) = adult.ypt_expired_date {
            if let Ok(exp_date) = NaiveDate::parse_from_str(exp_str, "%Y-%m-%d") {
                if exp_date < today {
                    ypt_expired += 1;
                    let formatted = format!("Expired {}", exp_date.format("%b %d, %Y"));
                    ypt_issues.push((adult.display_name(), formatted));
                } else if exp_date <= ninety_days_later {
                    ypt_expiring += 1;
                    let formatted = format!("Expires {}", exp_date.format("%b %d, %Y"));
                    ypt_issues.push((adult.display_name(), formatted));
                } else {
                    ypt_current += 1;
                }
            }
        }

        // Position training - values are "Trained" or "Not Trained"
        match adult.position_trained.as_deref() {
            Some("Trained") | Some("Y") | Some("Yes") | Some("true") => position_trained += 1,
            Some("Not Trained") | Some("N") | Some("No") | Some("false") => {
                position_not_trained += 1;
                position_not_trained_list.push(adult.display_name());
            }
            _ => {} // Unknown, don't count
        }
    }

    // Sort issues by name
    ypt_issues.sort_by(|a, b| a.0.cmp(&b.0));
    position_not_trained_list.sort();

    TrainingStats {
        ypt_current,
        ypt_expiring,
        ypt_expired,
        ypt_issues,
        position_trained,
        position_not_trained,
        position_not_trained_list,
    }
}

struct RenewalStats {
    scouts_current: usize,
    scouts_expiring: usize,
    scouts_expired: usize,
    scout_issues: Vec<(String, String)>,
    adults_current: usize,
    adults_expiring: usize,
    adults_expired: usize,
    adult_issues: Vec<(String, String)>,
}

fn calculate_renewal_stats(app: &App) -> RenewalStats {
    let today = Utc::now().date_naive();
    let ninety_days_later = today + chrono::Duration::days(90);

    let mut scouts_current = 0;
    let mut scouts_expiring = 0;
    let mut scouts_expired = 0;
    let mut scout_issues = Vec::new();

    let mut adults_current = 0;
    let mut adults_expiring = 0;
    let mut adults_expired = 0;
    let mut adult_issues = Vec::new();

    // Scout renewals
    for youth in &app.youth {
        if let Some(ref reg_info) = youth.registrar_info {
            if let Some(ref exp_str) = reg_info.registration_expire_dt {
                // Try to parse the date (might be in different formats)
                let exp_date = NaiveDate::parse_from_str(exp_str, "%Y-%m-%d")
                    .or_else(|_| NaiveDate::parse_from_str(&exp_str[..10], "%Y-%m-%d"));

                if let Ok(exp_date) = exp_date {
                    if exp_date < today {
                        scouts_expired += 1;
                        let formatted = format!("Expired {}", exp_date.format("%b %d, %Y"));
                        scout_issues.push((youth.display_name(), formatted));
                    } else if exp_date <= ninety_days_later {
                        scouts_expiring += 1;
                        let formatted = format!("Expires {}", exp_date.format("%b %d, %Y"));
                        scout_issues.push((youth.display_name(), formatted));
                    } else {
                        scouts_current += 1;
                    }
                } else {
                    scouts_current += 1; // Assume current if can't parse
                }
            } else {
                scouts_current += 1; // Assume current if no date
            }
        } else {
            scouts_current += 1;
        }
    }

    // Adult renewals
    for adult in &app.adults {
        if let Some(ref reg_info) = adult.registrar_info {
            if let Some(ref exp_str) = reg_info.registration_expire_dt {
                let exp_date = NaiveDate::parse_from_str(exp_str, "%Y-%m-%d")
                    .or_else(|_| NaiveDate::parse_from_str(&exp_str[..10], "%Y-%m-%d"));

                if let Ok(exp_date) = exp_date {
                    if exp_date < today {
                        adults_expired += 1;
                        let formatted = format!("Expired {}", exp_date.format("%b %d, %Y"));
                        adult_issues.push((adult.display_name(), formatted));
                    } else if exp_date <= ninety_days_later {
                        adults_expiring += 1;
                        let formatted = format!("Expires {}", exp_date.format("%b %d, %Y"));
                        adult_issues.push((adult.display_name(), formatted));
                    } else {
                        adults_current += 1;
                    }
                } else {
                    adults_current += 1;
                }
            } else {
                adults_current += 1;
            }
        } else {
            adults_current += 1;
        }
    }

    // Sort issues by name
    scout_issues.sort_by(|a, b| a.0.cmp(&b.0));
    adult_issues.sort_by(|a, b| a.0.cmp(&b.0));

    RenewalStats {
        scouts_current,
        scouts_expiring,
        scouts_expired,
        scout_issues,
        adults_current,
        adults_expiring,
        adults_expired,
        adult_issues,
    }
}

