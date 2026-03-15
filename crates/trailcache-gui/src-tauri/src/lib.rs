mod commands;
pub mod dto;
mod state;

use tauri::Manager;
use state::GuiAppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize tracing so trailcache-core log output appears on stderr during dev
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    if cfg!(debug_assertions) {
        tracing_subscriber::fmt()
            .with_env_filter("trailcache_core=debug,warn")
            .init();
    }

    #[allow(unused_mut)]
    let mut builder = tauri::Builder::default();

    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_window_state::Builder::new().build());
    }

    builder
        .setup(|app| {
            let config_dir = app.path().app_config_dir().ok();
            let cache_dir = app.path().app_cache_dir().ok();
            let app_state = GuiAppState::new(config_dir, cache_dir)
                .expect("Failed to initialize app state");
            app.manage(app_state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Auth
            commands::login,
            commands::get_saved_username,
            commands::logout,
            commands::quit_app,
            // Data
            commands::get_youth,
            commands::get_adults,
            commands::get_parents,
            commands::get_events,
            commands::get_event_guests,
            commands::get_patrols,
            commands::get_unit_info,
            commands::get_key3,
            commands::get_org_profile,
            commands::get_commissioners,
            // Advancement
            commands::get_advancement_dashboard,
            commands::get_youth_ranks,
            commands::get_youth_merit_badges,
            commands::get_youth_leadership,
            commands::get_youth_awards,
            commands::get_rank_requirements,
            commands::get_badge_requirements,
            // Aggregate (pivot tabs)
            commands::get_all_youth_ranks,
            commands::get_all_youth_badges,
            // Cache
            commands::get_cache_ages,
            commands::refresh_data,
            // Offline mode
            commands::get_offline_mode,
            commands::set_offline_mode,
            commands::cache_for_offline,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
