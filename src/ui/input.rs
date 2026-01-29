//! Keyboard input handling for the TUI.
//!
//! This module handles all keyboard events and translates them into
//! application state changes.

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{
    can_add_password_char, can_add_username_char, AdvancementView, App, AppState,
    EventDetailView, Focus, LoginFocus, ScoutDetailView, Tab, PAGE_SCROLL_SIZE,
};
use crate::models::{EventSortColumn, ScoutSortColumn};

/// Direction for cycling through views
enum CycleDirection {
    Forward,
    Backward,
}

/// Cycle the scout detail view and load data if needed.
/// Returns the user_id if data fetch was initiated.
async fn cycle_scout_detail_view(app: &mut App, direction: CycleDirection) {
    // Get user_id before modifying app
    let user_id = app.get_sorted_youth()
        .get(app.roster_selection)
        .and_then(|y| y.user_id);

    let new_view = match direction {
        CycleDirection::Forward => match app.scout_detail_view {
            ScoutDetailView::Details => ScoutDetailView::Ranks,
            ScoutDetailView::Ranks => ScoutDetailView::MeritBadges,
            ScoutDetailView::MeritBadges => ScoutDetailView::Leadership,
            ScoutDetailView::Leadership => ScoutDetailView::Details,
        },
        CycleDirection::Backward => match app.scout_detail_view {
            ScoutDetailView::Details => ScoutDetailView::Leadership,
            ScoutDetailView::Ranks => ScoutDetailView::Details,
            ScoutDetailView::MeritBadges => ScoutDetailView::Ranks,
            ScoutDetailView::Leadership => ScoutDetailView::MeritBadges,
        },
    };

    app.scout_detail_view = new_view;
    // Keep advancement_view in sync for navigation
    app.advancement_view = match new_view {
        ScoutDetailView::Ranks => AdvancementView::Ranks,
        ScoutDetailView::MeritBadges => AdvancementView::MeritBadges,
        ScoutDetailView::Details | ScoutDetailView::Leadership => app.advancement_view, // unchanged
    };
    app.viewing_requirements = false;
    // Reset selection when switching views (ranks start at top/Eagle since reversed)
    app.advancement_rank_selection = app.selected_youth_ranks.len().saturating_sub(1);
    app.advancement_badge_selection = 0;
    app.leadership_selection = 0;

    // Load data if switching to data views
    if let Some(uid) = user_id {
        match new_view {
            ScoutDetailView::Ranks | ScoutDetailView::MeritBadges => {
                app.fetch_youth_progress(uid).await;
            }
            ScoutDetailView::Leadership => {
                app.fetch_youth_leadership(uid).await;
            }
            ScoutDetailView::Details => {}
        }
    }
}

/// Handle keyboard input. Returns true if the app should quit.
pub async fn handle_input(app: &mut App, key: KeyEvent) -> Result<bool> {
    // Handle login overlay
    if matches!(app.state, AppState::LoggingIn) {
        return handle_login_input(app, key).await;
    }

    // Handle help overlay
    if matches!(app.state, AppState::ShowingHelp) {
        if matches!(key.code, KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q')) {
            app.state = AppState::Normal;
        }
        return Ok(false);
    }

    // Handle quit confirmation
    if matches!(app.state, AppState::ConfirmingQuit) {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                if app.caching_in_progress {
                    app.status_message = Some("Cannot quit while caching in progress. Please wait...".to_string());
                    app.state = AppState::Normal;
                    return Ok(false);
                }
                app.state = AppState::Quitting;
                return Ok(true);
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                app.state = AppState::Normal;
            }
            _ => {}
        }
        return Ok(false);
    }

    // Handle offline mode confirmation
    if matches!(app.state, AppState::ConfirmingOffline) {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                app.state = AppState::Normal;
                app.go_offline().await;
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                app.state = AppState::Normal;
            }
            _ => {}
        }
        return Ok(false);
    }

    // Handle online mode confirmation (when returning from offline)
    if matches!(app.state, AppState::ConfirmingOnline) {
        match key.code {
            KeyCode::Char('o') | KeyCode::Char('O') => {
                app.state = AppState::Normal;
                app.go_online();
            }
            _ => {
                // Any other key stays offline
                app.state = AppState::Normal;
            }
        }
        return Ok(false);
    }

    // Handle search mode
    if matches!(app.state, AppState::Searching) {
        return handle_search_input(app, key).await;
    }

    // Global keys
    match key.code {
        KeyCode::Char('q') => {
            if app.caching_in_progress {
                app.status_message = Some("Cannot quit while caching in progress. Please wait...".to_string());
                return Ok(false);
            }
            app.state = AppState::ConfirmingQuit;
            return Ok(false);
        }
        KeyCode::Char('?') => {
            app.state = AppState::ShowingHelp;
            return Ok(false);
        }
        KeyCode::Char('1') => {
            app.current_tab = Tab::Scouts;
            app.focus = Focus::List;
        }
        KeyCode::Char('2') => {
            app.current_tab = Tab::Ranks;
            app.focus = Focus::List;
        }
        KeyCode::Char('3') => {
            app.current_tab = Tab::Badges;
            app.focus = Focus::List;
        }
        KeyCode::Char('4') => {
            app.current_tab = Tab::Events;
            app.focus = Focus::List;
        }
        KeyCode::Char('5') => {
            app.current_tab = Tab::Adults;
            app.focus = Focus::List;
        }
        KeyCode::Char('6') => {
            app.current_tab = Tab::Unit;
            app.focus = Focus::List;
        }
        KeyCode::Left => {
            // If on Scouts tab with detail focus, cycle detail views
            if app.current_tab == Tab::Scouts && app.focus == Focus::Detail {
                cycle_scout_detail_view(app, CycleDirection::Backward).await;
            } else if app.current_tab == Tab::Events && app.focus == Focus::Detail {
                // Cycle Events detail views (only if RSVP is enabled for selected event)
                let rsvp_enabled = app.get_sorted_events()
                    .get(app.event_selection)
                    .map(|e| e.rsvp)
                    .unwrap_or(false);
                if rsvp_enabled {
                    app.event_detail_view = match app.event_detail_view {
                        EventDetailView::Details => EventDetailView::Rsvp,
                        EventDetailView::Rsvp => EventDetailView::Details,
                    };
                }
            } else {
                app.current_tab = app.current_tab.prev();
                app.focus = Focus::List;
            }
        }
        KeyCode::Right => {
            // If on Scouts tab with detail focus, cycle detail views
            if app.current_tab == Tab::Scouts && app.focus == Focus::Detail {
                cycle_scout_detail_view(app, CycleDirection::Forward).await;
            } else if app.current_tab == Tab::Events && app.focus == Focus::Detail {
                // Cycle Events detail views (only if RSVP is enabled for selected event)
                let rsvp_enabled = app.get_sorted_events()
                    .get(app.event_selection)
                    .map(|e| e.rsvp)
                    .unwrap_or(false);
                if rsvp_enabled {
                    app.event_detail_view = match app.event_detail_view {
                        EventDetailView::Details => EventDetailView::Rsvp,
                        EventDetailView::Rsvp => EventDetailView::Details,
                    };
                }
            } else {
                app.current_tab = app.current_tab.next();
                app.focus = Focus::List;
            }
        }
        KeyCode::Char('u') => {
            if !app.offline_mode {
                app.refresh_all_background().await;
            }
        }
        KeyCode::Char('o') => {
            if app.offline_mode {
                app.state = AppState::ConfirmingOnline;
            } else {
                app.state = AppState::ConfirmingOffline;
            }
        }
        KeyCode::Char('/') => {
            app.state = AppState::Searching;
            app.search_query.clear();
        }
        KeyCode::Tab => {
            // Toggle focus between list and detail panels
            app.focus = match app.focus {
                Focus::List => Focus::Detail,
                Focus::Detail => Focus::List,
            };
        }
        KeyCode::Esc => {
            // Check if we're viewing requirements on Scouts tab - go back
            if app.current_tab == Tab::Scouts && app.viewing_requirements {
                app.viewing_requirements = false;
                app.selected_rank_requirements.clear();
                app.selected_badge_requirements.clear();
                app.requirement_selection = 0;
            } else if app.current_tab == Tab::Scouts && app.scout_detail_view != ScoutDetailView::Details {
                // Go back to details view from Ranks/MeritBadges/Leadership
                app.scout_detail_view = ScoutDetailView::Details;
                app.focus = Focus::List;
            } else if app.current_tab == Tab::Events && app.event_detail_view == EventDetailView::Rsvp {
                // Go back to details view from RSVP
                app.event_detail_view = EventDetailView::Details;
            } else if app.current_tab == Tab::Ranks && app.ranks_viewing_requirements {
                // Go back from requirements view to scout list
                app.ranks_viewing_requirements = false;
                app.selected_rank_requirements.clear();
                app.ranks_requirement_selection = 0;
            } else if app.current_tab == Tab::Badges && app.badges_viewing_requirements {
                // Go back from requirements view to scout list
                app.badges_viewing_requirements = false;
                app.selected_badge_requirements.clear();
                app.badges_requirement_selection = 0;
            } else {
                app.search_query.clear();
                app.focus = Focus::List;
            }
        }
        _ => {
            // Tab-specific input
            match app.current_tab {
                Tab::Scouts => handle_scouts_input(app, key).await?,
                Tab::Adults => handle_adults_input(app, key).await?,
                Tab::Events => handle_events_input(app, key).await?,
                Tab::Unit => handle_dashboard_input(app, key).await?,
                Tab::Ranks => handle_ranks_input(app, key).await?,
                Tab::Badges => handle_badges_input(app, key).await?,
            }
        }
    }

    Ok(false)
}

async fn handle_search_input(app: &mut App, key: KeyEvent) -> Result<bool> {
    match key.code {
        KeyCode::Esc => {
            app.state = AppState::Normal;
            app.search_query.clear();
        }
        KeyCode::Enter => {
            app.state = AppState::Normal;
            // Keep search query active
        }
        KeyCode::Backspace => {
            app.search_query.pop();
        }
        KeyCode::Char(c) => {
            app.search_query.push(c);
            // Reset selection when search changes
            app.roster_selection = 0;
        }
        _ => {}
    }
    Ok(false)
}

async fn handle_login_input(app: &mut App, key: KeyEvent) -> Result<bool> {
    match key.code {
        KeyCode::Esc => {
            // Quit if on login screen
            app.state = AppState::Quitting;
            return Ok(true);
        }
        KeyCode::Down | KeyCode::Tab => {
            // Move to next field
            app.login_focus = match app.login_focus {
                LoginFocus::Username => LoginFocus::Password,
                LoginFocus::Password => LoginFocus::Button,
                LoginFocus::Button => LoginFocus::Username,
            };
        }
        KeyCode::Up | KeyCode::BackTab => {
            // Move to previous field
            app.login_focus = match app.login_focus {
                LoginFocus::Username => LoginFocus::Button,
                LoginFocus::Password => LoginFocus::Username,
                LoginFocus::Button => LoginFocus::Password,
            };
        }
        KeyCode::Enter => {
            match app.login_focus {
                LoginFocus::Username => {
                    // Move to password
                    app.login_focus = LoginFocus::Password;
                }
                LoginFocus::Password => {
                    // Move to button
                    app.login_focus = LoginFocus::Button;
                }
                LoginFocus::Button => {
                    // Attempt login
                    let _ = app.attempt_login().await;
                    // If successful, state will be Normal
                    // If failed, login_error will be set
                    if app.state == AppState::Normal {
                        // Login succeeded, refresh data
                        app.refresh_all_background().await;
                    }
                }
            }
        }
        KeyCode::Backspace => {
            match app.login_focus {
                LoginFocus::Username => {
                    app.login_username.pop();
                }
                LoginFocus::Password => {
                    app.login_password.pop();
                }
                LoginFocus::Button => {}
            }
        }
        KeyCode::Char(c) => {
            match app.login_focus {
                LoginFocus::Username => {
                    if can_add_username_char(app.login_username.len(), c) {
                        app.login_username.push(c);
                    }
                }
                LoginFocus::Password => {
                    if can_add_password_char(app.login_password.len(), c) {
                        app.login_password.push(c);
                    }
                }
                LoginFocus::Button => {
                    // Ignore character input on button
                }
            }
        }
        _ => {}
    }
    Ok(false)
}

async fn handle_scouts_input(app: &mut App, key: KeyEvent) -> Result<()> {
    let max_index = app.youth.len().saturating_sub(1);

    // Detail view switching - 'd' and 'm' work regardless of focus
    // 'r' is handled separately below based on focus
    match key.code {
        KeyCode::Char('d') => {
            app.scout_detail_view = ScoutDetailView::Details;
            app.viewing_requirements = false;
            return Ok(());
        }
        KeyCode::Char('m') => {
            // Get user_id before mutating app
            let user_id = app.get_sorted_youth()
                .get(app.roster_selection)
                .and_then(|y| y.user_id);

            app.scout_detail_view = ScoutDetailView::MeritBadges;
            app.advancement_view = AdvancementView::MeritBadges;
            app.focus = Focus::Detail;
            app.viewing_requirements = false;
            app.selected_rank_requirements.clear();
            app.selected_badge_requirements.clear();
            app.advancement_badge_selection = 0;
            // Always load progress (will use cache if available)
            if let Some(uid) = user_id {
                app.fetch_youth_progress(uid).await;
            }
            return Ok(());
        }
        KeyCode::Char('r') => {
            // 'r' behavior depends on focus:
            // - List focus: sort by rank
            // - Detail focus: switch to ranks view
            if app.focus == Focus::Detail {
                // Switch to Ranks view
                let user_id = app.get_sorted_youth()
                    .get(app.roster_selection)
                    .and_then(|y| y.user_id);

                app.scout_detail_view = ScoutDetailView::Ranks;
                app.advancement_view = AdvancementView::Ranks;
                app.viewing_requirements = false;
                app.selected_rank_requirements.clear();
                app.selected_badge_requirements.clear();
                // Start at top (Eagle) since display is reversed
                app.advancement_rank_selection = app.selected_youth_ranks.len().saturating_sub(1);
                // Always load progress (will use cache if available)
                if let Some(uid) = user_id {
                    app.fetch_youth_progress(uid).await;
                }
            } else {
                // Sort by rank
                app.toggle_scout_sort(ScoutSortColumn::Rank);
            }
            return Ok(());
        }
        KeyCode::Char('l') => {
            // Switch to Leadership view
            let user_id = app.get_sorted_youth()
                .get(app.roster_selection)
                .and_then(|y| y.user_id);

            app.scout_detail_view = ScoutDetailView::Leadership;
            app.focus = Focus::Detail;
            app.viewing_requirements = false;
            app.leadership_selection = 0;
            // Load leadership data
            if let Some(uid) = user_id {
                app.fetch_youth_leadership(uid).await;
            }
            return Ok(());
        }
        _ => {}
    }

    // Get sorted_youth reference for navigation
    let sorted_youth = app.get_sorted_youth();

    // Handle navigation based on current view and focus
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => {
            if app.viewing_requirements {
                // Navigate through requirements
                let max = match app.advancement_view {
                    AdvancementView::Ranks => app.selected_rank_requirements.len(),
                    AdvancementView::MeritBadges => app.selected_badge_requirements.len(),
                }.saturating_sub(1);
                app.requirement_selection = (app.requirement_selection + 1).min(max);
            } else if app.focus == Focus::Detail {
                // Navigate through ranks/badges/leadership
                match app.scout_detail_view {
                    ScoutDetailView::Ranks => {
                        // Reversed display order, so down = decrement
                        app.advancement_rank_selection = app.advancement_rank_selection.saturating_sub(1);
                    }
                    ScoutDetailView::MeritBadges => {
                        let max = app.selected_youth_badges.len().saturating_sub(1);
                        app.advancement_badge_selection = (app.advancement_badge_selection + 1).min(max);
                    }
                    ScoutDetailView::Leadership => {
                        // Leadership view is not navigable
                    }
                    _ => {}
                }
            } else {
                // Navigate scout list
                let old_selection = app.roster_selection;
                app.roster_selection = (app.roster_selection + 1).min(max_index);
                if old_selection != app.roster_selection {
                    // Clear progress data when changing scout
                    app.selected_youth_ranks.clear();
                    app.selected_youth_badges.clear();
                    app.selected_youth_leadership.clear();
                    app.selected_rank_requirements.clear();
                    app.selected_badge_requirements.clear();
                    app.viewing_requirements = false;
                    app.advancement_rank_selection = app.selected_youth_ranks.len().saturating_sub(1);
                    app.advancement_badge_selection = 0;
                    app.leadership_selection = 0;
                }
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.viewing_requirements {
                app.requirement_selection = app.requirement_selection.saturating_sub(1);
            } else if app.focus == Focus::Detail {
                match app.scout_detail_view {
                    ScoutDetailView::Ranks => {
                        // Reversed display order, so up = increment
                        let max = app.selected_youth_ranks.len().saturating_sub(1);
                        app.advancement_rank_selection = (app.advancement_rank_selection + 1).min(max);
                    }
                    ScoutDetailView::MeritBadges => {
                        app.advancement_badge_selection = app.advancement_badge_selection.saturating_sub(1);
                    }
                    ScoutDetailView::Leadership => {
                        // Leadership view is not navigable
                    }
                    _ => {}
                }
            } else {
                let old_selection = app.roster_selection;
                app.roster_selection = app.roster_selection.saturating_sub(1);
                if old_selection != app.roster_selection {
                    app.selected_youth_ranks.clear();
                    app.selected_youth_badges.clear();
                    app.selected_youth_leadership.clear();
                    app.selected_rank_requirements.clear();
                    app.selected_badge_requirements.clear();
                    app.viewing_requirements = false;
                    app.advancement_rank_selection = app.selected_youth_ranks.len().saturating_sub(1);
                    app.advancement_badge_selection = 0;
                    app.leadership_selection = 0;
                }
            }
        }
        KeyCode::Home => {
            if app.focus == Focus::List {
                app.roster_selection = 0;
                app.selected_youth_ranks.clear();
                app.selected_youth_badges.clear();
                app.selected_youth_leadership.clear();
            }
        }
        KeyCode::End => {
            if app.focus == Focus::List {
                app.roster_selection = max_index;
                app.selected_youth_ranks.clear();
                app.selected_youth_badges.clear();
                app.selected_youth_leadership.clear();
            }
        }
        KeyCode::PageDown => {
            if app.focus == Focus::List {
                app.roster_selection = (app.roster_selection + PAGE_SCROLL_SIZE).min(max_index);
                app.selected_youth_ranks.clear();
                app.selected_youth_badges.clear();
                app.selected_youth_leadership.clear();
            }
        }
        KeyCode::PageUp => {
            if app.focus == Focus::List {
                app.roster_selection = app.roster_selection.saturating_sub(PAGE_SCROLL_SIZE);
                app.selected_youth_ranks.clear();
                app.selected_youth_badges.clear();
                app.selected_youth_leadership.clear();
            }
        }
        KeyCode::Enter => {
            match app.focus {
                Focus::List => {
                    // Load progress for selected scout and switch to detail
                    if let Some(youth) = sorted_youth.get(app.roster_selection) {
                        if let Some(user_id) = youth.user_id {
                            app.fetch_youth_progress(user_id).await;
                            app.focus = Focus::Detail;
                        }
                    }
                }
                Focus::Detail => {
                    if !app.viewing_requirements {
                        match app.scout_detail_view {
                            ScoutDetailView::Ranks => {
                                // Load requirements for selected rank
                                if let Some(rank) = app.selected_youth_ranks.get(app.advancement_rank_selection) {
                                    let rank_id = rank.rank_id;
                                    if let Some(youth) = sorted_youth.get(app.roster_selection) {
                                        if let Some(user_id) = youth.user_id {
                                            app.fetch_rank_requirements(user_id, rank_id).await;
                                        }
                                    }
                                }
                            }
                            ScoutDetailView::MeritBadges => {
                                // Load requirements for selected merit badge
                                let sorted_badges = crate::ui::tabs::advancement::get_sorted_badges(&app.selected_youth_badges);
                                if let Some(badge) = sorted_badges.get(app.advancement_badge_selection) {
                                    let badge_id = badge.id;
                                    if let Some(youth) = sorted_youth.get(app.roster_selection) {
                                        if let Some(user_id) = youth.user_id {
                                            app.fetch_badge_requirements(user_id, badge_id).await;
                                        }
                                    }
                                }
                            }
                            ScoutDetailView::Details | ScoutDetailView::Leadership => {}
                        }
                    }
                }
            }
        }
        // Sort keys (only in list focus)
        KeyCode::Char('n') if app.focus == Focus::List => {
            app.toggle_scout_sort(ScoutSortColumn::Name);
        }
        KeyCode::Char('g') if app.focus == Focus::List => {
            app.toggle_scout_sort(ScoutSortColumn::Grade);
        }
        KeyCode::Char('a') if app.focus == Focus::List => {
            app.toggle_scout_sort(ScoutSortColumn::Age);
        }
        KeyCode::Char('p') if app.focus == Focus::List => {
            app.toggle_scout_sort(ScoutSortColumn::Patrol);
        }
        KeyCode::Char('s') if app.focus == Focus::List => {
            app.scout_sort_column = app.scout_sort_column.next();
            app.scout_sort_ascending = true;
            app.roster_selection = 0;
        }
        KeyCode::Char('S') if app.focus == Focus::List => {
            app.scout_sort_ascending = !app.scout_sort_ascending;
        }
        _ => {}
    }
    Ok(())
}

async fn handle_adults_input(app: &mut App, key: KeyEvent) -> Result<()> {
    let max_index = app.adults.len().saturating_sub(1);

    match key.code {
        KeyCode::Char('j') | KeyCode::Down => {
            app.adults_selection = (app.adults_selection + 1).min(max_index);
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.adults_selection = app.adults_selection.saturating_sub(1);
        }
        KeyCode::Home => {
            app.adults_selection = 0;
        }
        KeyCode::End => {
            app.adults_selection = max_index;
        }
        KeyCode::PageDown => {
            app.adults_selection = (app.adults_selection + PAGE_SCROLL_SIZE).min(max_index);
        }
        KeyCode::PageUp => {
            app.adults_selection = app.adults_selection.saturating_sub(PAGE_SCROLL_SIZE);
        }
        _ => {}
    }
    Ok(())
}

async fn handle_events_input(app: &mut App, key: KeyEvent) -> Result<()> {
    let sorted_events = app.get_sorted_events();
    let max_event = sorted_events.len().saturating_sub(1);

    // Check if selected event has RSVP enabled
    let rsvp_enabled = sorted_events
        .get(app.event_selection)
        .map(|e| e.rsvp)
        .unwrap_or(false);

    match app.focus {
        Focus::List => {
            match key.code {
                KeyCode::Char('j') | KeyCode::Down => {
                    app.event_selection = (app.event_selection + 1).min(max_event);
                    app.event_guest_selection = 0;
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    app.event_selection = app.event_selection.saturating_sub(1);
                    app.event_guest_selection = 0;
                }
                KeyCode::Enter => {
                    app.focus = Focus::Detail;
                }
                // Sort keys - toggle ascending/descending if same column
                KeyCode::Char('n') => {
                    app.toggle_event_sort(EventSortColumn::Name);
                }
                KeyCode::Char('d') => {
                    app.toggle_event_sort(EventSortColumn::Date);
                }
                KeyCode::Char('l') => {
                    app.toggle_event_sort(EventSortColumn::Location);
                }
                KeyCode::Char('t') => {
                    app.toggle_event_sort(EventSortColumn::Type);
                }
                _ => {}
            }
        }
        Focus::Detail => {
            match key.code {
                KeyCode::Char('d') => {
                    app.event_detail_view = EventDetailView::Details;
                }
                KeyCode::Char('r') if rsvp_enabled => {
                    app.event_detail_view = EventDetailView::Rsvp;
                }
                KeyCode::Enter if rsvp_enabled => {
                    app.event_detail_view = EventDetailView::Rsvp;
                }
                KeyCode::Esc => {
                    if app.event_detail_view == EventDetailView::Rsvp {
                        app.event_detail_view = EventDetailView::Details;
                    } else {
                        app.focus = Focus::List;
                    }
                }
                KeyCode::Left | KeyCode::Right if rsvp_enabled => {
                    app.event_detail_view = match app.event_detail_view {
                        EventDetailView::Details => EventDetailView::Rsvp,
                        EventDetailView::Rsvp => EventDetailView::Details,
                    };
                }
                _ => {}
            }
        }
    }
    Ok(())
}

async fn handle_dashboard_input(_app: &mut App, _key: KeyEvent) -> Result<()> {
    // Dashboard tab is display-only, no special input handling needed
    // Navigation between tabs is handled by global keys
    Ok(())
}

async fn handle_ranks_input(app: &mut App, key: KeyEvent) -> Result<()> {
    use crate::ui::tabs::ranks::{get_ranks_with_scouts, get_rank_list};

    let rank_list = get_rank_list(&app.youth, &app.all_youth_ranks, app.ranks_sort_by_count, app.ranks_sort_ascending);
    let grouped = get_ranks_with_scouts(&app.youth, &app.all_youth_ranks);
    let max_rank = rank_list.len().saturating_sub(1);

    // Get scouts for current selection using sorted list
    let selected_rank_name = rank_list.get(app.ranks_selection)
        .map(|(name, _)| name.as_str())
        .unwrap_or("");
    let max_scout = grouped.iter()
        .find(|(name, _)| name == selected_rank_name)
        .map(|(_, scouts)| scouts.len().saturating_sub(1))
        .unwrap_or(0);

    match app.focus {
        Focus::List => {
            match key.code {
                KeyCode::Char('j') | KeyCode::Down => {
                    app.ranks_selection = (app.ranks_selection + 1).min(max_rank);
                    app.ranks_scout_selection = 0;
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    app.ranks_selection = app.ranks_selection.saturating_sub(1);
                    app.ranks_scout_selection = 0;
                }
                KeyCode::Enter => {
                    app.focus = Focus::Detail;
                    app.ranks_scout_selection = 0;
                }
                KeyCode::Home => {
                    app.ranks_selection = 0;
                    app.ranks_scout_selection = 0;
                }
                KeyCode::End => {
                    app.ranks_selection = max_rank;
                    app.ranks_scout_selection = 0;
                }
                KeyCode::Char('n') => {
                    app.toggle_ranks_sort_by_name();
                }
                KeyCode::Char('c') => {
                    app.toggle_ranks_sort_by_count();
                }
                _ => {}
            }
        }
        Focus::Detail => {
            if app.ranks_viewing_requirements {
                // Navigate within requirements
                let max_req = app.selected_rank_requirements.len().saturating_sub(1);
                match key.code {
                    KeyCode::Char('j') | KeyCode::Down => {
                        app.ranks_requirement_selection = (app.ranks_requirement_selection + 1).min(max_req);
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        app.ranks_requirement_selection = app.ranks_requirement_selection.saturating_sub(1);
                    }
                    KeyCode::Esc => {
                        // Exit requirements view but stay in right panel
                        app.ranks_viewing_requirements = false;
                        app.selected_rank_requirements.clear();
                        app.ranks_requirement_selection = 0;
                        app.ranks_scout_selection = 0;
                    }
                    _ => {}
                }
            } else {
                // Navigate scout list
                match key.code {
                    KeyCode::Char('j') | KeyCode::Down => {
                        app.ranks_scout_selection = (app.ranks_scout_selection + 1).min(max_scout);
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        app.ranks_scout_selection = app.ranks_scout_selection.saturating_sub(1);
                    }
                    KeyCode::Enter => {
                        // Load rank requirements for selected scout
                        if let Some(scouts) = grouped.iter()
                            .find(|(name, _)| name == selected_rank_name)
                            .map(|(_, s)| s)
                        {
                            if let Some(srp) = scouts.get(app.ranks_scout_selection) {
                                if let (Some(user_id), Some(rank)) = (srp.youth.user_id, &srp.rank) {
                                    // Fetch requirements for this specific rank
                                    app.fetch_rank_requirements(user_id, rank.rank_id).await;
                                    app.ranks_viewing_requirements = true;
                                    app.ranks_requirement_selection = 0;
                                }
                            }
                        }
                    }
                    KeyCode::Esc => {
                        app.focus = Focus::List;
                    }
                    KeyCode::Home => {
                        app.ranks_scout_selection = 0;
                    }
                    KeyCode::End => {
                        app.ranks_scout_selection = max_scout;
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(())
}

async fn handle_badges_input(app: &mut App, key: KeyEvent) -> Result<()> {
    use crate::ui::tabs::badges::{get_badges_with_scouts, get_badge_list};

    let badge_list = get_badge_list(&app.youth, &app.all_youth_badges, app.badges_sort_by_count, app.badges_sort_ascending);
    let grouped = get_badges_with_scouts(&app.youth, &app.all_youth_badges);
    let max_badge = badge_list.len().saturating_sub(1);

    // Get scouts for current selection using sorted list
    let selected_badge_name = badge_list.get(app.badges_selection)
        .map(|(name, _, _)| name.as_str())
        .unwrap_or("");
    let max_scout = grouped.iter()
        .find(|(name, _, _)| name == selected_badge_name)
        .map(|(_, _, scouts)| scouts.len().saturating_sub(1))
        .unwrap_or(0);

    match app.focus {
        Focus::List => {
            match key.code {
                KeyCode::Char('j') | KeyCode::Down => {
                    if !badge_list.is_empty() {
                        app.badges_selection = (app.badges_selection + 1).min(max_badge);
                        app.badges_scout_selection = 0;
                    }
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    app.badges_selection = app.badges_selection.saturating_sub(1);
                    app.badges_scout_selection = 0;
                }
                KeyCode::Enter => {
                    if !badge_list.is_empty() {
                        app.focus = Focus::Detail;
                        app.badges_scout_selection = 0;
                    }
                }
                KeyCode::Home => {
                    app.badges_selection = 0;
                    app.badges_scout_selection = 0;
                }
                KeyCode::End => {
                    app.badges_selection = max_badge;
                    app.badges_scout_selection = 0;
                }
                KeyCode::Char('n') => {
                    app.toggle_badges_sort_by_name();
                }
                KeyCode::Char('c') => {
                    app.toggle_badges_sort_by_count();
                }
                _ => {}
            }
        }
        Focus::Detail => {
            if app.badges_viewing_requirements {
                // Navigate within requirements
                let max_req = app.selected_badge_requirements.len().saturating_sub(1);
                match key.code {
                    KeyCode::Char('j') | KeyCode::Down => {
                        app.badges_requirement_selection = (app.badges_requirement_selection + 1).min(max_req);
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        app.badges_requirement_selection = app.badges_requirement_selection.saturating_sub(1);
                    }
                    KeyCode::Esc => {
                        // Exit requirements view but stay in right panel
                        app.badges_viewing_requirements = false;
                        app.selected_badge_requirements.clear();
                        app.badges_requirement_selection = 0;
                        app.badges_scout_selection = 0;
                    }
                    _ => {}
                }
            } else {
                // Navigate scout list
                match key.code {
                    KeyCode::Char('j') | KeyCode::Down => {
                        app.badges_scout_selection = (app.badges_scout_selection + 1).min(max_scout);
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        app.badges_scout_selection = app.badges_scout_selection.saturating_sub(1);
                    }
                    KeyCode::Enter => {
                        // Load badge requirements for selected scout
                        if let Some(scouts) = grouped.iter()
                            .find(|(name, _, _)| name == selected_badge_name)
                            .map(|(_, _, s)| s)
                        {
                            if let Some(sbp) = scouts.get(app.badges_scout_selection) {
                                if let Some(user_id) = sbp.youth.user_id {
                                    // Fetch requirements for this specific badge
                                    app.fetch_badge_requirements(user_id, sbp.badge.id).await;
                                    app.badges_viewing_requirements = true;
                                    app.badges_requirement_selection = 0;
                                }
                            }
                        }
                    }
                    KeyCode::Esc => {
                        app.focus = Focus::List;
                    }
                    KeyCode::Home => {
                        app.badges_scout_selection = 0;
                    }
                    KeyCode::End => {
                        app.badges_scout_selection = max_scout;
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(())
}
