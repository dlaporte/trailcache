//! Application state management for Trailcache.
//!
//! This module contains the core `App` struct that manages all application state,
//! including UI state, cached data, session management, and background task coordination.

use std::collections::HashMap;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;
use futures::stream::{self, StreamExt};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use trailcache_core::api::ApiClient;
use trailcache_core::auth::{CredentialStore, Session};
use trailcache_core::cache::CacheManager;
use trailcache_core::config::Config;

use trailcache_core::models::{
    sort_requirements, Adult, AdvancementDashboard, Commissioner, Event, EventGuest,
    EventSortColumn, Key3Leaders, LeadershipPosition, MeritBadgeProgress,
    MeritBadgeRequirement, OrgProfile, Award, Parent, Patrol, RankProgress, RankRequirement,
    ReadyToAward, ScoutSortColumn, UnitInfo, Youth,
};
use trailcache_core::models::advancement::CounselorInfo;


use ratatui::layout::Rect;
use ratatui::widgets::TableState;

// ============================================================================
// Constants
// ============================================================================

/// Buffer size for the background task message channel.
/// 32 is sufficient for typical refresh operations (~10 API calls) with headroom.
const CHANNEL_BUFFER_SIZE: usize = 32;

/// Maximum length for username input.
/// Scouting.org usernames are typically email addresses, 50 chars covers most.
const MAX_USERNAME_LENGTH: usize = 50;

/// Maximum length for password input.
/// 128 chars accommodates password managers and passphrases.
const MAX_PASSWORD_LENGTH: usize = 128;

/// Number of items to scroll on page up/down.
/// 10 rows provides a good balance of speed without losing context.
pub const PAGE_SCROLL_SIZE: usize = 10;

/// Maximum concurrent API requests for event details.
/// Limits parallel requests to avoid overwhelming the server or hitting rate limits.
const MAX_CONCURRENT_REQUESTS: usize = 10;

/// Maximum number of event guest lists to cache.
/// Limits memory usage while keeping recently viewed events accessible.
const MAX_EVENT_GUESTS_CACHE_SIZE: usize = 50;

// ============================================================================
// Helper Functions
// ============================================================================

/// Create an authenticated API client with the given token.
/// This is a free function to allow use inside spawned async tasks.
fn create_authenticated_api(token: String) -> Result<ApiClient> {
    let mut api = ApiClient::new()?;
    api.set_token(token);
    Ok(api)
}

// Re-export ScoutRank from core for use in TUI modules
pub use trailcache_core::models::ScoutRank;

// ============================================================================
// UI State Types
// ============================================================================

/// Main navigation tabs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Scouts,
    Ranks,
    Badges,
    Events,
    Adults,
    Unit,
}

impl Tab {
    /// Get the display title for this tab.
    pub fn title(&self) -> &'static str {
        match self {
            Tab::Scouts => "Scouts",
            Tab::Ranks => "Ranks",
            Tab::Badges => "Badges",
            Tab::Events => "Events",
            Tab::Adults => "Adults",
            Tab::Unit => "Unit",
        }
    }

    /// Get the next tab (wrapping around)
    pub fn next(&self) -> Self {
        match self {
            Tab::Scouts => Tab::Ranks,
            Tab::Ranks => Tab::Badges,
            Tab::Badges => Tab::Events,
            Tab::Events => Tab::Adults,
            Tab::Adults => Tab::Unit,
            Tab::Unit => Tab::Scouts,
        }
    }

    /// Get the previous tab (wrapping around)
    pub fn prev(&self) -> Self {
        match self {
            Tab::Scouts => Tab::Unit,
            Tab::Ranks => Tab::Scouts,
            Tab::Badges => Tab::Ranks,
            Tab::Events => Tab::Badges,
            Tab::Adults => Tab::Events,
            Tab::Unit => Tab::Adults,
        }
    }
}

/// Sub-view for scout detail panel
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScoutDetailView {
    Details,
    Ranks,
    MeritBadges,
    Leadership,
    Awards,
}

/// Sub-view for event detail panel
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventDetailView {
    Details,
    Rsvp,
}

/// Advancement tab sub-view
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdvancementView {
    Ranks,
    MeritBadges,
}

/// Current UI focus area (list panel or detail panel)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    List,
    Detail,
}

/// Overall application state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppState {
    Normal,
    Searching,
    ShowingHelp,
    LoggingIn,
    ConfirmingQuit,
    ConfirmingOffline,
    ConfirmingOnline,
    Quitting,
}

/// Login form focus state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LoginFocus {
    Username,
    Password,
    Button,
}

// ============================================================================
// Background Task Results
// ============================================================================

/// Result types from background refresh tasks.
///
/// These variants are sent through an MPSC channel from the background refresh
/// task back to the main application. Each variant represents a different type
/// of data that was fetched from the API.
#[allow(dead_code)]
enum RefreshResult {
    /// Youth roster data fetched successfully
    Youth(Vec<Youth>),
    /// Adult roster data fetched successfully
    Adults(Vec<Adult>),
    /// Parent contact data fetched successfully
    Parents(Vec<Parent>),
    /// Patrol structure data fetched successfully
    Patrols(Vec<Patrol>),
    /// Calendar events fetched successfully
    Events(Vec<Event>),
    /// Detailed event info (RSVP list) for a single event
    EventDetail(Event),
    /// Advancement statistics dashboard
    AdvancementDashboard(AdvancementDashboard),
    /// Awards ready to be presented
    ReadyToAward(Vec<ReadyToAward>),
    /// Guest list for a specific event (event_id, guests)
    EventGuests(i64, Vec<EventGuest>),
    /// Rank progress for a specific youth (user_id, ranks)
    YouthRanks(i64, Vec<RankProgress>),
    /// Merit badge progress for a specific youth (user_id, badges)
    YouthMeritBadges(i64, Vec<MeritBadgeProgress>),
    /// Leadership position history for a specific youth (user_id, positions)
    YouthLeadership(i64, Vec<LeadershipPosition>),
    /// Awards for a specific youth (user_id, awards)
    YouthAwards(i64, Vec<Award>),
    /// Requirements for a specific rank (user_id, rank_id, requirements)
    RankRequirements(i64, i64, Vec<RankRequirement>),
    /// Requirements for a specific merit badge (user_id, badge_id, requirements, version, counselor)
    BadgeRequirements(i64, i64, Vec<MeritBadgeRequirement>, Option<String>, Option<CounselorInfo>),
    /// Key 3 leadership positions (SM, CC, COR)
    Key3(Key3Leaders),
    /// Unit PIN information (charter, contact info)
    UnitPinInfo(UnitInfo),
    /// Organization profile information
    OrgProfile(OrgProfile),
    /// Assigned commissioners for the unit
    Commissioners(Vec<Commissioner>),
    /// Signal that all refresh tasks have completed
    RefreshComplete,
    /// Progress update for offline caching (current, total, description)
    CachingProgress(usize, usize, String),
    /// Offline caching is complete
    CachingComplete,
    /// An error occurred during refresh
    Error(String),
}

// ============================================================================
// Main Application Struct
// ============================================================================

#[derive(Default)]
pub struct LayoutAreas {
    pub title_bar: Rect,
    pub tabs_bar: Rect,
    pub left_panel: Rect,
    pub right_panel: Rect,
    pub detail_tabs_area: Rect,
}

/// Main application state container
#[allow(dead_code)]
pub struct App {
    // Core services
    pub config: Config,
    pub session: Session,
    pub api: ApiClient,
    pub cache: CacheManager,

    // UI State
    pub state: AppState,
    pub current_tab: Tab,
    pub focus: Focus,
    pub search_query: String,
    pub advancement_view: AdvancementView,
    pub scout_sort_column: ScoutSortColumn,
    pub scout_sort_ascending: bool,
    pub scout_detail_view: ScoutDetailView,
    pub event_detail_view: EventDetailView,
    pub event_sort_column: EventSortColumn,
    pub event_sort_ascending: bool,
    pub viewing_rsvp_list: bool,
    pub layout_areas: LayoutAreas,
    pub left_table_state: TableState,
    pub right_table_state: TableState,
    pub last_click: Option<(u16, u16, Instant)>,

    // Login form state
    pub login_username: String,
    pub login_password: String,
    pub login_focus: LoginFocus,
    pub login_error: Option<String>,

    // Selection indices
    pub roster_selection: usize,
    pub adults_selection: usize,
    pub patrol_member_selection: usize,
    pub advancement_selection: usize,
    pub advancement_rank_selection: usize,
    pub advancement_badge_selection: usize,
    pub event_selection: usize,
    pub event_guest_selection: usize,

    // Ranks tab state
    pub ranks_selection: usize,
    pub ranks_scout_selection: usize,
    pub ranks_viewing_requirements: bool,
    pub ranks_requirement_selection: usize,
    pub ranks_sort_by_count: bool,
    pub ranks_sort_ascending: bool,

    // Badges tab state
    pub badges_selection: usize,
    pub badges_scout_selection: usize,
    pub badges_viewing_requirements: bool,
    pub badges_requirement_selection: usize,
    pub badges_sort_by_count: bool,
    pub badges_sort_ascending: bool,

    // Cached data
    pub youth: Vec<Youth>,
    pub adults: Vec<Adult>,
    pub parents: Vec<Parent>,
    pub patrols: Vec<Patrol>,
    pub events: Vec<Event>,
    pub advancement_dashboard: AdvancementDashboard,
    pub ready_to_award: Vec<ReadyToAward>,
    pub event_guests: HashMap<i64, Vec<EventGuest>>,
    /// Tracks event_guests keys in access order (oldest first) for LRU eviction
    event_guests_order: Vec<i64>,

    /// Merit badge progress for all youth, keyed by user_id
    pub all_youth_badges: HashMap<i64, Vec<MeritBadgeProgress>>,

    /// Rank progress for all youth, keyed by user_id
    pub all_youth_ranks: HashMap<i64, Vec<RankProgress>>,

    // Unit info (domain types)
    pub key3: Key3Leaders,
    pub unit_info: Option<UnitInfo>,
    pub org_profile: OrgProfile,
    pub commissioners: Vec<Commissioner>,

    // Individual youth data
    pub selected_youth_ranks: Vec<RankProgress>,
    pub selected_youth_badges: Vec<MeritBadgeProgress>,
    pub selected_youth_leadership: Vec<LeadershipPosition>,
    pub selected_youth_awards: Vec<Award>,
    pub awards_loaded: bool,
    pub selected_rank_requirements: Vec<RankRequirement>,
    pub selected_badge_requirements: Vec<MeritBadgeRequirement>,
    pub selected_badge_version: Option<String>,
    pub selected_badge_counselor: Option<CounselorInfo>,
    pub viewing_requirements: bool,
    pub requirement_selection: usize,
    pub leadership_selection: usize,
    pub awards_selection: usize,

    // Track which requirements are currently being viewed (to prevent overwrites from background fetches)
    viewing_rank_user_id: Option<i64>,
    viewing_rank_id: Option<i64>,
    viewing_badge_user_id: Option<i64>,
    viewing_badge_id: Option<i64>,

    // Background task channel
    refresh_rx: Option<mpsc::Receiver<RefreshResult>>,
    refresh_tx: mpsc::Sender<RefreshResult>,

    // Status message
    pub status_message: Option<String>,

    // Cache ages for status bar
    pub cache_ages: trailcache_core::cache::CacheAges,

    // Offline mode - when true, only use cached data
    pub offline_mode: bool,

    // Offline caching progress tracking
    pub caching_in_progress: bool,
    pub caching_current: usize,
    pub caching_total: usize,
    pub caching_description: String,
}

impl App {
    /// Create a new application instance
    pub async fn new() -> Result<Self> {
        debug!("App::new() starting");
        let config = match Config::load() {
            Ok(c) => c,
            Err(e) => {
                warn!(error = %e, "Failed to load config, using defaults");
                Config::default()
            }
        };
        info!(
            org_guid = ?config.organization_guid,
            offline_mode = config.offline_mode,
            "Config loaded"
        );

        let cache_dir = config.cache_dir().unwrap_or_else(|_| PathBuf::from("./cache"));
        info!(?cache_dir, "Cache directory");

        // Load session from disk if it exists
        let mut session = Session::new(cache_dir.clone());
        let load_result = session.load();
        debug!(?load_result, has_data = session.data.is_some(), "Session loaded");

        let mut api = ApiClient::new()?;

        // If we have a valid session, set the token on the API client
        if let Some(ref data) = session.data {
            debug!(expired = data.is_expired(), "Session found");
            if !data.is_expired() {
                api.set_token(data.token.clone());
                debug!("Token set on API client");
            }
        } else {
            debug!("No session data found");
        }

        // Create cache manager without encryption initially - will be enabled after login
        let cache = CacheManager::new_without_encryption(cache_dir)?;

        let (tx, rx) = mpsc::channel(CHANNEL_BUFFER_SIZE);

        // Get credentials from env vars or config
        let login_username = std::env::var("SCOUTBOOK_USERNAME")
            .ok()
            .or_else(|| config.last_username.clone())
            .unwrap_or_default();

        let login_password = std::env::var("SCOUTBOOK_PASSWORD").unwrap_or_default();

        let offline_mode = config.offline_mode;

        Ok(Self {
            config,
            session,
            api,
            cache,

            state: AppState::Normal,
            current_tab: Tab::Scouts,
            focus: Focus::List,
            search_query: String::new(),
            advancement_view: AdvancementView::Ranks,
            scout_sort_column: ScoutSortColumn::Name,
            scout_sort_ascending: true,
            scout_detail_view: ScoutDetailView::Details,
            event_detail_view: EventDetailView::Details,
            event_sort_column: EventSortColumn::Date,
            event_sort_ascending: true,
            viewing_rsvp_list: false,
            layout_areas: LayoutAreas::default(),
            left_table_state: TableState::default(),
            right_table_state: TableState::default(),
            last_click: None,

            login_username,
            login_password,
            login_focus: LoginFocus::Username,
            login_error: None,

            roster_selection: 0,
            adults_selection: 0,
            patrol_member_selection: 0,
            advancement_selection: 0,
            advancement_rank_selection: 0,
            advancement_badge_selection: 0,
            event_selection: 0,
            event_guest_selection: 0,

            ranks_selection: 0,
            ranks_scout_selection: 0,
            ranks_viewing_requirements: false,
            ranks_requirement_selection: 0,
            ranks_sort_by_count: false,
            ranks_sort_ascending: false,

            badges_selection: 0,
            badges_scout_selection: 0,
            badges_viewing_requirements: false,
            badges_requirement_selection: 0,
            badges_sort_by_count: false,
            badges_sort_ascending: true,

            youth: Vec::new(),
            adults: Vec::new(),
            parents: Vec::new(),
            patrols: Vec::new(),
            events: Vec::new(),
            advancement_dashboard: AdvancementDashboard::default(),
            ready_to_award: Vec::new(),
            event_guests: HashMap::new(),
            event_guests_order: Vec::new(),
            all_youth_badges: HashMap::new(),
            all_youth_ranks: HashMap::new(),

            key3: Default::default(),
            unit_info: None,
            org_profile: Default::default(),
            commissioners: Vec::new(),

            selected_youth_ranks: Vec::new(),
            selected_youth_badges: Vec::new(),
            selected_youth_leadership: Vec::new(),
            selected_youth_awards: Vec::new(),
            awards_loaded: false,
            selected_rank_requirements: Vec::new(),
            selected_badge_requirements: Vec::new(),
            selected_badge_version: None,
            selected_badge_counselor: None,
            viewing_requirements: false,
            requirement_selection: 0,
            leadership_selection: 0,
            awards_selection: 0,

            viewing_rank_user_id: None,
            viewing_rank_id: None,
            viewing_badge_user_id: None,
            viewing_badge_id: None,

            refresh_rx: Some(rx),
            refresh_tx: tx,

            status_message: None,
            cache_ages: Default::default(),
            offline_mode,

            caching_in_progress: false,
            caching_current: 0,
            caching_total: 0,
            caching_description: String::new(),
        })
    }

    // =========================================================================
    // Authentication
    // =========================================================================

    /// Check if the user is authenticated with a valid session
    #[allow(dead_code)]
    pub async fn is_authenticated(&self) -> bool {
        self.session.data.as_ref().map(|d| !d.is_expired()).unwrap_or(false)
    }

    /// Interactive login (used for CLI mode)
    #[allow(dead_code)]
    pub async fn login_interactive(&mut self) -> Result<()> {
        println!("\n=== Trailcache Login ===\n");

        let username = if let Some(ref last_user) = self.config.last_username {
            if CredentialStore::has_credentials(last_user) {
                print!("Username [{}]: ", last_user);
                io::stdout().flush()?;

                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                let input = input.trim();

                if input.is_empty() {
                    last_user.clone()
                } else {
                    input.to_string()
                }
            } else {
                Self::prompt_username()?
            }
        } else {
            Self::prompt_username()?
        };

        let password = if CredentialStore::has_credentials(&username) {
            print!("Use stored password? [Y/n]: ");
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;

            if input.trim().to_lowercase() != "n" {
                CredentialStore::get_password(&username)?
            } else {
                Self::prompt_password()?
            }
        } else {
            Self::prompt_password()?
        };

        println!("\nAuthenticating...");

        let session_data = self.api.authenticate(&username, &password).await?;

        CredentialStore::store(&username, &password)?;

        self.config.last_username = Some(username);
        self.config.organization_guid = Some(session_data.organization_guid.clone());
        self.config.save()?;

        self.session.update(session_data);
        self.session.save()?;

        if let Some(ref data) = self.session.data {
            self.api.set_token(data.token.clone());
        }

        println!("Login successful!\n");
        Ok(())
    }

    #[allow(dead_code)]
    fn prompt_username() -> Result<String> {
        print!("Username: ");
        io::stdout().flush()?;

        let mut username = String::new();
        io::stdin().read_line(&mut username)?;
        Ok(username.trim().to_string())
    }

    #[allow(dead_code)]
    fn prompt_password() -> Result<String> {
        let password = rpassword::prompt_password("Password: ")?;
        Ok(password)
    }

    /// Attempt login with the credentials from the login form
    pub async fn attempt_login(&mut self) -> Result<()> {
        let username = self.login_username.clone();
        let password = self.login_password.clone();

        if username.is_empty() || password.is_empty() {
            self.login_error = Some("Username and password required".to_string());
            return Err(anyhow::anyhow!("Username and password required"));
        }

        self.login_error = None;

        // If offline mode, just derive key and load cache (no API call)
        if self.offline_mode {
            if let Some(ref org_guid) = self.config.organization_guid {
                // Derive encryption key from password
                self.cache.set_password(&password, org_guid);

                // Try to load cache to verify password is correct
                if let Err(e) = self.load_from_cache() {
                    self.login_error = Some("Failed to decrypt cache. Wrong password?".to_string());
                    return Err(e);
                }

                // Check if we actually loaded any data
                if self.youth.is_empty() {
                    self.login_error = Some("No cached data found. Go online to download.".to_string());
                    return Err(anyhow::anyhow!("No cached data"));
                }

                self.config.last_username = Some(username);
                let _ = self.config.save();
                self.login_password.clear();
                self.state = AppState::Normal;
                self.status_message = Some("Offline mode - loaded from cache".to_string());
                info!("Offline login successful - loaded from cache");
                return Ok(());
            } else {
                self.login_error = Some("No organization configured. Go online first.".to_string());
                return Err(anyhow::anyhow!("No organization GUID"));
            }
        }

        // Online mode - authenticate with API
        match self.api.authenticate(&username, &password).await {
            Ok(session_data) => {
                if let Err(e) = CredentialStore::store(&username, &password) {
                    warn!(error = %e, "Failed to store credentials");
                }

                self.config.last_username = Some(username);
                self.config.organization_guid = Some(session_data.organization_guid.clone());

                // Enable cache encryption with password-derived key
                self.cache.set_password(&password, &session_data.organization_guid);

                if let Err(e) = self.config.save() {
                    warn!(error = %e, "Failed to save config");
                }

                self.session.update(session_data);

                if let Err(e) = self.session.save() {
                    warn!(error = %e, "Failed to save session");
                }

                if let Some(ref data) = self.session.data {
                    self.api.set_token(data.token.clone());
                }

                // Load any existing cache and refresh
                let _ = self.load_from_cache();
                if self.is_cache_stale() {
                    self.refresh_all_background().await;
                }

                self.login_password.clear();
                self.state = AppState::Normal;
                info!("Login successful");
                Ok(())
            }
            Err(e) => {
                // Don't log to stderr - it corrupts the TUI display
                // Show error, clear password, and focus password field for retry
                self.login_error = Some("LOGIN FAILED".to_string());
                self.login_password.clear();
                self.login_focus = LoginFocus::Password;
                Err(e)
            }
        }
    }

    /// Start the login process (show login overlay)
    pub fn start_login(&mut self) {
        self.state = AppState::LoggingIn;
        self.login_focus = if self.login_username.is_empty() {
            LoginFocus::Username
        } else {
            LoginFocus::Password
        };
        self.login_error = None;
    }

    // =========================================================================
    // Cache Management
    // =========================================================================

    /// Load all data from cache
    pub fn load_from_cache(&mut self) -> Result<()> {
        info!(cache_dir = ?self.cache.cache_dir(), "Loading from cache");

        match self.cache.load_youth() {
            Ok(Some(cached)) => {
                info!(count = cached.data.len(), age = %cached.age_display(), "Loaded youth from cache");
                self.youth = cached.data;
            }
            Ok(None) => {
                info!("No youth cache found");
            }
            Err(e) => {
                warn!(error = %e, "Failed to load youth cache");
            }
        }

        if let Ok(Some(cached)) = self.cache.load_adults() {
            info!(count = cached.data.len(), "Loaded adults from cache");
            self.adults = cached.data;
        }

        if let Ok(Some(cached)) = self.cache.load_parents() {
            self.parents = cached.data;
        }

        if let Ok(Some(cached)) = self.cache.load_patrols() {
            self.patrols = cached.data;
        }

        if let Ok(Some(cached)) = self.cache.load_events() {
            self.events = cached.data;
        }

        if let Ok(Some(cached)) = self.cache.load_advancement_dashboard() {
            self.advancement_dashboard = cached.data;
        }

        if let Ok(Some(cached)) = self.cache.load_ready_to_award() {
            self.ready_to_award = cached.data;
        }

        if let Ok(Some(cached)) = self.cache.load_unit_info() {
            self.unit_info = Some(cached.data.with_computed_fields());
        }

        if let Ok(Some(cached)) = self.cache.load_key3() {
            self.key3 = cached.data;
        }

        if let Ok(Some(cached)) = self.cache.load_org_profile() {
            self.org_profile = cached.data;
        }

        if let Ok(Some(cached)) = self.cache.load_commissioners() {
            self.commissioners = cached.data;
        }

        // Load per-youth ranks and badges for Ranks/Badges tabs
        for youth in &self.youth {
            if let Some(user_id) = youth.user_id {
                if let Ok(Some(cached)) = self.cache.load_youth_ranks(user_id) {
                    self.all_youth_ranks.insert(user_id, cached.data);
                }
                if let Ok(Some(cached)) = self.cache.load_youth_merit_badges(user_id) {
                    self.all_youth_badges.insert(user_id, cached.data);
                }
            }
        }

        self.cache_ages = self.cache.get_cache_ages();
        Ok(())
    }

    /// Check if any cache data is stale
    pub fn is_cache_stale(&self) -> bool {
        self.cache.any_stale()
    }

    // =========================================================================
    // Background Data Refresh
    // =========================================================================

    /// Spawn a background task to refresh all data
    pub async fn refresh_all_background(&mut self) {
        // Don't refresh in offline mode
        if self.offline_mode {
            return;
        }

        info!("Starting background refresh of all data");

        let org_guid = match &self.config.organization_guid {
            Some(guid) => Arc::new(guid.clone()),
            None => {
                warn!("No organization GUID configured");
                return;
            }
        };

        let token = match self.session.token() {
            Some(t) => Arc::new(t.to_string()),
            None => {
                warn!("No token available for refresh");
                return;
            }
        };

        let user_id = match self.session.user_id() {
            Some(id) => id,
            None => {
                warn!("No user_id available for refresh");
                return;
            }
        };

        let tx = self.refresh_tx.clone();

        tokio::spawn(async move {
            Self::execute_background_refresh(tx, org_guid, token, user_id).await;
        });

        self.status_message = Some("Refreshing data...".to_string());
    }

    /// Enter offline mode - cache all data and work from cache only.
    pub async fn go_offline(&mut self) {
        info!("Entering offline mode - caching all data");

        // First, do a full refresh to ensure cache is current
        let org_guid = match &self.config.organization_guid {
            Some(guid) => Arc::new(guid.clone()),
            None => {
                warn!("No organization GUID configured");
                self.status_message = Some("Error: No organization configured".to_string());
                return;
            }
        };

        let token = match self.session.token() {
            Some(t) => Arc::new(t.to_string()),
            None => {
                warn!("No token available");
                self.status_message = Some("Error: Not authenticated".to_string());
                return;
            }
        };

        let user_id = match self.session.user_id() {
            Some(id) => id,
            None => {
                warn!("No user_id available");
                self.status_message = Some("Error: Not authenticated".to_string());
                return;
            }
        };

        let tx = self.refresh_tx.clone();
        let cache = self.cache.clone();

        // Set caching in progress state
        self.caching_in_progress = true;
        self.caching_current = 0;
        self.caching_total = 0;
        self.caching_description = "Starting...".to_string();
        self.status_message = Some("Caching data for offline mode: Starting...".to_string());

        tokio::spawn(async move {
            Self::execute_offline_caching(tx, org_guid, token, user_id, cache).await;
        });
    }

    /// Exit offline mode - resume normal online operation.
    /// Forces reauthentication to ensure fresh credentials.
    pub fn go_online(&mut self) {
        info!("Exiting offline mode - forcing reauthentication");
        self.offline_mode = false;
        self.config.offline_mode = false;
        let _ = self.config.save();
        // Force reauthentication when coming back online
        self.start_login();
    }

    /// Check if app should show offline mode prompt on startup
    #[allow(dead_code)]
    pub fn should_prompt_offline_on_startup(&self) -> bool {
        self.offline_mode
    }

    /// Helper to send refresh results, logging any channel errors
    async fn send_result(tx: &mpsc::Sender<RefreshResult>, result: RefreshResult) {
        if let Err(e) = tx.send(result).await {
            error!(error = %e, "Failed to send refresh result - channel closed");
        }
    }

    /// Execute the background refresh task.
    ///
    /// This function runs in a spawned Tokio task and fetches all data from the
    /// Scouting API in parallel. Results are sent back through the MPSC channel
    /// as `RefreshResult` variants.
    ///
    /// # Arguments
    /// * `tx` - Channel sender to communicate results back to main app
    /// * `org_guid` - Organization GUID for API requests
    /// * `token` - Authentication token for API requests
    /// * `user_id` - User ID for user-specific data (events)
    ///
    /// # Behavior
    /// - Creates multiple API clients for parallel requests
    /// - Fetches youth, adults, parents, patrols, events concurrently
    /// - Fetches event details with limited concurrency (MAX_CONCURRENT_REQUESTS)
    /// - Sends RefreshComplete when all fetches are done
    async fn execute_background_refresh(
        tx: mpsc::Sender<RefreshResult>,
        org_guid: Arc<String>,
        token: Arc<String>,
        user_id: i64,
    ) {
        info!("Background refresh task started");

        let base_api = match ApiClient::new() {
            Ok(api) => api,
            Err(e) => {
                error!(error = %e, "Failed to create API client");
                Self::send_result(&tx, RefreshResult::Error("Failed to create API client".to_string())).await;
                return;
            }
        };

        // Create API clients for parallel fetching.
        // The TUI's process_refresh_result handles caching, so we just fetch here.
        let api = base_api.with_token(Arc::clone(&token));
        let api2 = api.clone();
        let api3 = api.clone();
        let api4 = api.clone();
        let api5 = api.clone();
        let api6 = api.clone();
        let api7 = api.clone();
        let api8 = api.clone();
        let api9 = api.clone();
        let api10 = api.clone();

        let org2 = Arc::clone(&org_guid);
        let org3 = Arc::clone(&org_guid);
        let org4 = Arc::clone(&org_guid);
        let org5 = Arc::clone(&org_guid);
        let org6 = Arc::clone(&org_guid);
        let org7 = Arc::clone(&org_guid);
        let org8 = Arc::clone(&org_guid);

        // Fetch all main data in parallel
        let (youth_res, adults_res, parents_res, patrols_res, events_res, dashboard_res, ready_res, key3_res, pin_res, profile_res) = tokio::join!(
            api.fetch_youth(&org_guid),
            api2.fetch_adults(&org2),
            api3.fetch_parents(&org3),
            api4.fetch_patrols(&org4),
            api5.fetch_events(user_id),
            api6.fetch_advancement_dashboard(&org5),
            api7.fetch_ready_to_award(&org6),
            api8.fetch_key3(&org7),
            api9.fetch_unit_pin(&org8),
            api10.fetch_org_profile(&org_guid),
        );

        // Extract youth user IDs before moving youth_res
        let youth_user_ids: Vec<i64> = youth_res
            .as_ref()
            .map(|list| list.iter().filter_map(|y| y.user_id).collect())
            .unwrap_or_default();

        // Deduplicate adults before sending
        let adults_res = adults_res.map(Adult::deduplicate);

        // Process and send results
        Self::send_fetch_result(&tx, "Youth", youth_res, RefreshResult::Youth).await;
        Self::send_fetch_result(&tx, "Adults", adults_res, RefreshResult::Adults).await;
        Self::send_fetch_result_or_empty(&tx, "Parents", parents_res, RefreshResult::Parents, vec![]).await;
        Self::send_fetch_result_or_empty(&tx, "Patrols", patrols_res, RefreshResult::Patrols, vec![]).await;
        Self::send_fetch_result_or_default(&tx, "Dashboard", dashboard_res, RefreshResult::AdvancementDashboard).await;
        Self::send_fetch_result_or_empty(&tx, "ReadyToAward", ready_res, RefreshResult::ReadyToAward, vec![]).await;
        Self::send_key3_result(&tx, key3_res).await;
        Self::send_pin_result(&tx, pin_res).await;
        Self::send_profile_result(&tx, profile_res).await;

        // Handle events with detail fetches
        Self::handle_events_refresh(&tx, events_res, &token).await;

        // Fetch commissioners separately
        let api_commissioners = base_api.with_token(Arc::clone(&token));
        match api_commissioners.fetch_commissioners(&org_guid).await {
            Ok(commissioners) => {
                debug!(count = commissioners.len(), "Commissioners fetched");
                Self::send_result(&tx, RefreshResult::Commissioners(commissioners)).await;
            }
            Err(e) => {
                debug!(error = %e, "Failed to fetch commissioners");
            }
        }

        // Fetch rank and merit badge progress for all youth (TUI-specific)
        Self::handle_all_youth_advancement_refresh(&tx, &youth_user_ids, &token).await;

        info!("Background refresh complete");
        Self::send_result(&tx, RefreshResult::RefreshComplete).await;
    }

    /// Execute offline caching using the shared core function.
    /// Delegates all fetching + caching to `trailcache_core::cache::cache_all_for_offline`,
    /// forwarding progress via the refresh channel.
    async fn execute_offline_caching(
        tx: mpsc::Sender<RefreshResult>,
        org_guid: Arc<String>,
        token: Arc<String>,
        user_id: i64,
        cache: CacheManager,
    ) {
        info!("Offline caching task started");

        let base_api = match ApiClient::new() {
            Ok(api) => api,
            Err(e) => {
                error!(error = %e, "Failed to create API client for offline caching");
                Self::send_result(&tx, RefreshResult::Error("Failed to create API client".to_string())).await;
                return;
            }
        };

        let api = base_api.with_token(token);

        let tx_progress = tx.clone();
        let result = trailcache_core::cache::cache_all_for_offline(
            &api,
            &cache,
            &org_guid,
            user_id,
            |progress| {
                // Forward progress to the TUI's refresh channel (best-effort)
                let _ = tx_progress.try_send(RefreshResult::CachingProgress(
                    progress.current as usize,
                    progress.total as usize,
                    progress.description,
                ));
            },
        )
        .await;

        match result {
            Ok(msg) => info!("{}", msg),
            Err(e) => error!(error = %e, "Offline caching failed"),
        }

        Self::send_result(&tx, RefreshResult::CachingComplete).await;
    }

    /// Fetch rank, merit badge, and leadership progress for all youth members.
    /// This populates all_youth_ranks and all_youth_badges HashMaps for the Ranks/Badges tabs,
    /// and caches leadership data for the Leadership tab.
    async fn handle_all_youth_advancement_refresh(
        tx: &mpsc::Sender<RefreshResult>,
        user_ids: &[i64],
        token: &Arc<String>,
    ) {
        if user_ids.is_empty() {
            return;
        }

        debug!(count = user_ids.len(), "Fetching ranks, badges, and leadership for all youth");

        // Create API client
        let api = match create_authenticated_api(token.to_string()) {
            Ok(api) => api,
            Err(e) => {
                error!(error = %e, "Failed to create API client for youth advancement");
                return;
            }
        };

        // Fetch ranks, badges, and leadership for all youth with limited concurrency
        const MAX_CONCURRENT: usize = 5;
        for chunk in user_ids.chunks(MAX_CONCURRENT) {
            let futures: Vec<_> = chunk
                .iter()
                .map(|&user_id| {
                    let api = api.clone();
                    async move {
                        let ranks = api.fetch_youth_ranks(user_id).await.ok();
                        let badges = api.fetch_youth_merit_badges(user_id).await.ok();
                        let leadership = api.fetch_youth_leadership(user_id).await.ok();
                        (user_id, ranks, badges, leadership)
                    }
                })
                .collect();

            let results = futures::future::join_all(futures).await;
            for (user_id, ranks, badges, leadership) in results {
                if let Some(ranks) = ranks {
                    Self::send_result(tx, RefreshResult::YouthRanks(user_id, ranks)).await;
                }
                if let Some(badges) = badges {
                    Self::send_result(tx, RefreshResult::YouthMeritBadges(user_id, badges)).await;
                }
                if let Some(leadership) = leadership {
                    Self::send_result(tx, RefreshResult::YouthLeadership(user_id, leadership)).await;
                }
            }
        }

        debug!("All youth advancement fetching complete");
    }

    /// Helper to send a successful fetch result or an error
    async fn send_fetch_result<T, F>(
        tx: &mpsc::Sender<RefreshResult>,
        name: &str,
        result: Result<T>,
        wrapper: F,
    ) where
        F: FnOnce(T) -> RefreshResult,
    {
        match result {
            Ok(data) => {
                info!("{} fetched successfully, sending to channel", name);
                Self::send_result(tx, wrapper(data)).await;
            }
            Err(e) => {
                error!(error = %e, "{} fetch failed", name);
                Self::send_result(tx, RefreshResult::Error(format!("{}: {}", name, e))).await;
            }
        }
    }

    /// Helper to send a fetch result or a default empty value
    async fn send_fetch_result_or_empty<T, F>(
        tx: &mpsc::Sender<RefreshResult>,
        name: &str,
        result: Result<T>,
        wrapper: F,
        default: T,
    ) where
        F: FnOnce(T) -> RefreshResult,
    {
        match result {
            Ok(data) => {
                debug!("{} fetched successfully", name);
                Self::send_result(tx, wrapper(data)).await;
            }
            Err(e) => {
                debug!(error = %e, "{} fetch failed, using default", name);
                Self::send_result(tx, wrapper(default)).await;
            }
        }
    }

    /// Helper to send a fetch result or a Default value
    async fn send_fetch_result_or_default<T: Default, F>(
        tx: &mpsc::Sender<RefreshResult>,
        name: &str,
        result: Result<T>,
        wrapper: F,
    ) where
        F: FnOnce(T) -> RefreshResult,
    {
        match result {
            Ok(data) => {
                debug!("{} fetched successfully", name);
                Self::send_result(tx, wrapper(data)).await;
            }
            Err(e) => {
                debug!(error = %e, "{} fetch failed, using default", name);
                Self::send_result(tx, wrapper(T::default())).await;
            }
        }
    }

    async fn send_key3_result(
        tx: &mpsc::Sender<RefreshResult>,
        result: Result<Key3Leaders>,
    ) {
        match result {
            Ok(data) => {
                debug!("Key3 fetched successfully");
                Self::send_result(tx, RefreshResult::Key3(data)).await;
            }
            Err(e) => {
                debug!(error = %e, "Key3 fetch failed");
                Self::send_result(tx, RefreshResult::Key3(Default::default())).await;
            }
        }
    }

    async fn send_pin_result(
        tx: &mpsc::Sender<RefreshResult>,
        result: Result<UnitInfo>,
    ) {
        match result {
            Ok(data) => {
                debug!(website = ?data.website, "Unit info fetched");
                Self::send_result(tx, RefreshResult::UnitPinInfo(data)).await;
            }
            Err(e) => {
                debug!(error = %e, "Unit info fetch failed");
            }
        }
    }

    async fn send_profile_result(
        tx: &mpsc::Sender<RefreshResult>,
        result: Result<OrgProfile>,
    ) {
        match result {
            Ok(data) => {
                debug!("Org profile fetched successfully");
                Self::send_result(tx, RefreshResult::OrgProfile(data)).await;
            }
            Err(e) => {
                debug!(error = %e, "Profile fetch failed");
                Self::send_result(tx, RefreshResult::OrgProfile(Default::default())).await;
            }
        }
    }

    async fn handle_events_refresh(
        tx: &mpsc::Sender<RefreshResult>,
        events_res: Result<Vec<Event>>,
        token: &Arc<String>,
    ) {
        match events_res {
            Ok(data) => {
                info!(count = data.len(), "Events fetched");

                // Extract IDs before sending to avoid cloning the entire events list
                let event_ids: Vec<i64> = data.iter().map(|e| e.id).collect();

                Self::send_result(tx, RefreshResult::Events(data)).await;

                // Fetch detailed info for each event with limited concurrency
                debug!("Fetching event details with max {} concurrent requests...", MAX_CONCURRENT_REQUESTS);

                let tx_clone = tx.clone();
                let token = Arc::clone(token);

                stream::iter(event_ids)
                    .map(|id| {
                        let token = Arc::clone(&token);
                        async move {
                            match ApiClient::new() {
                                Ok(mut api) => {
                                    api.set_token(token);
                                    api.fetch_event_detail(id).await
                                }
                                Err(e) => Err(e),
                            }
                        }
                    })
                    .buffer_unordered(MAX_CONCURRENT_REQUESTS)
                    .for_each(|result| {
                        let tx = tx_clone.clone();
                        async move {
                            if let Ok(detail) = result {
                                debug!(event = %detail.name, users = detail.invited_users.len(), "Event detail fetched");
                                Self::send_result(&tx, RefreshResult::EventDetail(detail)).await;
                            }
                        }
                    })
                    .await;

                debug!("Event details complete");
            }
            Err(e) => {
                error!(error = %e, "Events fetch failed");
                Self::send_result(tx, RefreshResult::Events(vec![])).await;
            }
        }
    }

    /// Check for completed background tasks and process results
    pub async fn check_background_tasks(&mut self) {
        // Collect all pending results first to avoid borrow conflicts
        let results: Vec<RefreshResult> = {
            if let Some(ref mut rx) = self.refresh_rx {
                let mut results = Vec::new();
                while let Ok(result) = rx.try_recv() {
                    results.push(result);
                }
                results
            } else {
                Vec::new()
            }
        };

        // Now process all results
        for result in results {
            self.process_refresh_result(result);
        }
    }

    /// Process a single refresh result from the background task.
    ///
    /// Updates the corresponding app state and caches the data. This is called
    /// by `check_background_tasks` for each result received from the channel.
    fn process_refresh_result(&mut self, result: RefreshResult) {
        match result {
            RefreshResult::Youth(data) => {
                info!(count = data.len(), "Processing Youth result - saving to cache");
                match self.cache.save_youth(&data) {
                    Ok(()) => info!("Youth cache saved successfully"),
                    Err(e) => error!(error = %e, "Failed to cache youth data"),
                }
                self.youth = data;
                if self.roster_selection >= self.youth.len() {
                    self.roster_selection = self.youth.len().saturating_sub(1);
                }
                self.cache_ages = self.cache.get_cache_ages();
            }
            RefreshResult::Adults(data) => {
                // Data is already deduplicated by execute_background_refresh
                info!(count = data.len(), "Processing Adults result - saving to cache");
                match self.cache.save_adults(&data) {
                    Ok(()) => info!("Adults cache saved successfully"),
                    Err(e) => error!(error = %e, "Failed to cache adults data"),
                }
                self.adults = data;
            }
            RefreshResult::Parents(data) => {
                if let Err(e) = self.cache.save_parents(&data) {
                    warn!(error = %e, "Failed to cache parents data");
                }
                self.parents = data;
            }
            RefreshResult::Patrols(data) => {
                if let Err(e) = self.cache.save_patrols(&data) {
                    warn!(error = %e, "Failed to cache patrols data");
                }
                self.patrols = data;
            }
            RefreshResult::Events(data) => {
                if let Err(e) = self.cache.save_events(&data) {
                    warn!(error = %e, "Failed to cache events data");
                }
                self.events = data;
                self.cache_ages = self.cache.get_cache_ages();
            }
            RefreshResult::EventDetail(event) => {
                if let Some(existing) = self.events.iter_mut().find(|e| e.id == event.id) {
                    *existing = event;
                }
                if let Err(e) = self.cache.save_events(&self.events) {
                    warn!(error = %e, "Failed to cache event detail");
                }
            }
            RefreshResult::AdvancementDashboard(data) => {
                if let Err(e) = self.cache.save_advancement_dashboard(&data) {
                    warn!(error = %e, "Failed to cache advancement dashboard");
                }
                self.advancement_dashboard = data;
            }
            RefreshResult::ReadyToAward(data) => {
                if let Err(e) = self.cache.save_ready_to_award(&data) {
                    warn!(error = %e, "Failed to cache ready to award");
                }
                self.ready_to_award = data;
            }
            RefreshResult::Key3(data) => {
                if let Err(e) = self.cache.save_key3(&data) {
                    warn!(error = %e, "Failed to cache key3");
                }
                self.key3 = data;
            }
            RefreshResult::UnitPinInfo(data) => {
                if let Err(e) = self.cache.save_unit_info(&data) {
                    warn!(error = %e, "Failed to cache unit pin info");
                }
                self.unit_info = Some(data.with_computed_fields());
            }
            RefreshResult::OrgProfile(data) => {
                if let Err(e) = self.cache.save_org_profile(&data) {
                    warn!(error = %e, "Failed to cache org profile");
                }
                self.org_profile = data;
            }
            RefreshResult::Commissioners(data) => {
                if let Err(e) = self.cache.save_commissioners(&data) {
                    warn!(error = %e, "Failed to cache commissioners");
                }
                self.commissioners = data;
            }
            RefreshResult::EventGuests(event_id, data) => {
                // LRU eviction: remove oldest entries if cache is at capacity
                if self.event_guests.len() >= MAX_EVENT_GUESTS_CACHE_SIZE
                    && !self.event_guests.contains_key(&event_id)
                {
                    // Remove oldest half based on access order
                    let evict_count = MAX_EVENT_GUESTS_CACHE_SIZE / 2;
                    let to_remove: Vec<_> = self.event_guests_order.drain(..evict_count).collect();
                    for key in &to_remove {
                        self.event_guests.remove(key);
                    }
                    debug!(
                        evicted = to_remove.len(),
                        "Evicted oldest event guests cache entries"
                    );
                }

                // Update access order: remove existing entry if present, then add to end
                if let Some(pos) = self.event_guests_order.iter().position(|&id| id == event_id) {
                    self.event_guests_order.remove(pos);
                }
                self.event_guests_order.push(event_id);
                self.event_guests.insert(event_id, data);
            }
            RefreshResult::YouthRanks(user_id, data) => {
                if let Err(e) = self.cache.save_youth_ranks(user_id, &data) {
                    warn!(error = %e, "Failed to cache youth ranks");
                }
                // Store in all_youth_ranks for the Ranks tab aggregate view
                self.all_youth_ranks.insert(user_id, data.clone());
                // Only update selected view if this is the currently selected scout
                let selected_user_id = self.get_sorted_youth()
                    .get(self.roster_selection)
                    .and_then(|y| y.user_id);
                if selected_user_id == Some(user_id) {
                    self.selected_youth_ranks = data;
                }
            }
            RefreshResult::YouthMeritBadges(user_id, data) => {
                if let Err(e) = self.cache.save_youth_merit_badges(user_id, &data) {
                    warn!(error = %e, "Failed to cache youth merit badges");
                }
                // Store in all_youth_badges for the Badges tab aggregate view
                self.all_youth_badges.insert(user_id, data.clone());
                // Only update selected view if this is the currently selected scout
                let selected_user_id = self.get_sorted_youth()
                    .get(self.roster_selection)
                    .and_then(|y| y.user_id);
                if selected_user_id == Some(user_id) {
                    self.selected_youth_badges = data;
                }
            }
            RefreshResult::YouthLeadership(user_id, mut data) => {
                LeadershipPosition::sort_for_display(&mut data);
                if let Err(e) = self.cache.save_youth_leadership(user_id, &data) {
                    warn!(error = %e, "Failed to cache youth leadership");
                }
                // Only update selected view if this is the currently selected scout
                let selected_user_id = self.get_sorted_youth()
                    .get(self.roster_selection)
                    .and_then(|y| y.user_id);
                if selected_user_id == Some(user_id) {
                    self.selected_youth_leadership = data;
                }
            }
            RefreshResult::YouthAwards(user_id, mut data) => {
                Award::sort_for_display(&mut data);
                if let Err(e) = self.cache.save_youth_awards(user_id, &data) {
                    warn!(error = %e, "Failed to cache youth awards");
                }
                // Only update selected view if this is the currently selected scout
                let selected_user_id = self.get_sorted_youth()
                    .get(self.roster_selection)
                    .and_then(|y| y.user_id);
                if selected_user_id == Some(user_id) {
                    self.selected_youth_awards = data;
                    self.awards_loaded = true;
                }
            }
            RefreshResult::RankRequirements(user_id, rank_id, data) => {
                // Cache the requirements
                if let Err(e) = self.cache.save_rank_requirements(user_id, rank_id, &data) {
                    warn!(error = %e, "Failed to cache rank requirements");
                }
                // Only update selected view if this is the currently viewed rank
                if self.viewing_rank_user_id == Some(user_id) && self.viewing_rank_id == Some(rank_id) {
                    let mut sorted = data;
                    sort_requirements(&mut sorted);
                    self.selected_rank_requirements = sorted;
                    self.viewing_requirements = true;
                    self.requirement_selection = 0;
                }
            }
            RefreshResult::BadgeRequirements(user_id, badge_id, data, version, counselor) => {
                // Cache the requirements
                if let Err(e) = self.cache.save_badge_requirements(user_id, badge_id, &data, &version) {
                    warn!(error = %e, "Failed to cache badge requirements");
                }
                // Only update selected view if this is the currently viewed badge
                if self.viewing_badge_user_id == Some(user_id) && self.viewing_badge_id == Some(badge_id) {
                    let mut sorted = data;
                    sort_requirements(&mut sorted);
                    self.selected_badge_requirements = sorted;
                    self.selected_badge_version = version;
                    self.selected_badge_counselor = counselor;
                    self.viewing_requirements = true;
                    self.requirement_selection = 0;
                }
            }
            RefreshResult::RefreshComplete => {
                // Only clear status if it's a progress message, preserve errors
                if let Some(ref msg) = self.status_message {
                    if !msg.starts_with("Error:") {
                        self.status_message = None;
                    }
                }
            }
            RefreshResult::CachingProgress(current, total, description) => {
                self.caching_current = current;
                self.caching_total = total;
                self.caching_description = description.clone();
                let pct = if total > 0 { (current * 100) / total } else { 0 };
                self.status_message = Some(format!("Caching: {} ({}%)", description, pct));
            }
            RefreshResult::CachingComplete => {
                // Reload in-memory state from the freshly-populated cache
                if let Err(e) = self.load_from_cache() {
                    warn!(error = %e, "Failed to reload from cache after offline caching");
                }

                // Enable offline mode
                self.caching_in_progress = false;
                self.offline_mode = true;
                self.config.offline_mode = true;
                if let Err(e) = self.config.save() {
                    warn!(error = %e, "Failed to save config after offline caching");
                    self.status_message = Some("Caching complete but config save failed".to_string());
                    return;
                }

                info!(cache_dir = ?self.cache.cache_dir(), youth_count = self.youth.len(), "Offline caching complete");
                self.status_message = Some(format!(
                    "Offline mode ready - {} scouts cached",
                    self.youth.len()
                ));
            }
            RefreshResult::Error(msg) => {
                // Show error and log it
                error!(error = %msg, "Background task error");
                // Simplify common error messages for the user
                let user_message = if msg.to_lowercase().contains("rate limit") {
                    "Server is busy. Please wait a moment and try again.".to_string()
                } else if msg.to_lowercase().contains("unauthorized")
                    || msg.to_lowercase().contains("401")
                {
                    // Session expired - prompt for re-login if not offline
                    if !self.offline_mode {
                        self.start_login();
                        self.login_error = Some("Session expired. Please log in again.".to_string());
                    }
                    "Session expired. Please log in again.".to_string()
                } else if msg.to_lowercase().contains("network")
                    || msg.to_lowercase().contains("connect")
                {
                    "Network error. Check your connection.".to_string()
                } else {
                    format!("Error: {}", msg)
                };
                self.status_message = Some(user_message);
            }
        }
    }

    /// Refresh only data for the current tab
    #[allow(dead_code)]
    pub async fn refresh_current_tab(&mut self) {
        // Don't refresh in offline mode
        if self.offline_mode {
            return;
        }

        let org_guid = match &self.config.organization_guid {
            Some(guid) => Arc::new(guid.clone()),
            None => return,
        };

        let token = match self.session.token() {
            Some(t) => Arc::new(t.to_string()),
            None => return,
        };

        let user_id = match self.session.user_id() {
            Some(id) => id,
            None => return,
        };

        let tx = self.refresh_tx.clone();
        let tab = self.current_tab;

        tokio::spawn(async move {
            let api = match create_authenticated_api(token.to_string()) {
                Ok(api) => api,
                Err(e) => {
                    error!(error = %e, "Failed to create API client for tab refresh");
                    return;
                }
            };

            match tab {
                Tab::Scouts => {
                    if let Ok(data) = api.fetch_youth(&org_guid).await {
                        Self::send_result(&tx, RefreshResult::Youth(data)).await;
                    }
                    if let Ok(data) = api.fetch_parents(&org_guid).await {
                        Self::send_result(&tx, RefreshResult::Parents(data)).await;
                    }
                    if let Ok(data) = api.fetch_advancement_dashboard(&org_guid).await {
                        Self::send_result(&tx, RefreshResult::AdvancementDashboard(data)).await;
                    }
                }
                Tab::Adults => {
                    if let Ok(data) = api.fetch_adults(&org_guid).await {
                        Self::send_result(&tx, RefreshResult::Adults(data)).await;
                    }
                }
                Tab::Events => {
                    if let Ok(data) = api.fetch_events(user_id).await {
                        // Extract IDs before sending to avoid cloning entire events list
                        let event_ids: Vec<i64> = data.iter().map(|e| e.id).collect();

                        Self::send_result(&tx, RefreshResult::Events(data)).await;
                        let detail_futures: Vec<_> = event_ids.iter().map(|&id| {
                            api.fetch_event_detail(id)
                        }).collect();

                        let results = futures::future::join_all(detail_futures).await;
                        for detail in results.into_iter().flatten() {
                            Self::send_result(&tx, RefreshResult::EventDetail(detail)).await;
                        }
                    }
                }
                Tab::Unit => {
                    if let Ok(data) = api.fetch_youth(&org_guid).await {
                        Self::send_result(&tx, RefreshResult::Youth(data)).await;
                    }
                    if let Ok(data) = api.fetch_adults(&org_guid).await {
                        Self::send_result(&tx, RefreshResult::Adults(data)).await;
                    }
                    if let Ok(data) = api.fetch_advancement_dashboard(&org_guid).await {
                        Self::send_result(&tx, RefreshResult::AdvancementDashboard(data)).await;
                    }
                }
                Tab::Ranks => {
                    // Ranks tab uses youth data
                    if let Ok(data) = api.fetch_youth(&org_guid).await {
                        Self::send_result(&tx, RefreshResult::Youth(data)).await;
                    }
                }
                Tab::Badges => {
                    // Badges tab uses ready-to-award data
                    if let Ok(data) = api.fetch_ready_to_award(&org_guid).await {
                        Self::send_result(&tx, RefreshResult::ReadyToAward(data)).await;
                    }
                }
            }
        });

        self.status_message = Some(format!("Refreshing {}...", tab.title()));
    }

    /// Fetch event guests for a specific event
    #[allow(dead_code)]
    pub async fn fetch_event_guests(&mut self, event_id: i64) {
        // In offline mode, don't fetch (event guests aren't cached)
        if self.offline_mode {
            return;
        }

        let token = match self.session.token() {
            Some(t) => t.to_string(),
            None => return,
        };

        let tx = self.refresh_tx.clone();

        tokio::spawn(async move {
            let api = match create_authenticated_api(token) {
                Ok(api) => api,
                Err(e) => {
                    error!(error = %e, "Failed to create API client for event guests");
                    return;
                }
            };

            if let Ok(data) = api.fetch_event_guests(event_id).await {
                Self::send_result(&tx, RefreshResult::EventGuests(event_id, data)).await;
            }
        });
    }

    /// Fetch progress data for a specific youth
    pub async fn fetch_youth_progress(&mut self, user_id: i64) {
        if user_id <= 0 {
            warn!(user_id, "Invalid user_id for youth progress fetch");
            return;
        }

        // In offline mode, use cached data only
        if self.offline_mode {
            if let Ok(Some(cached)) = self.cache.load_youth_ranks(user_id) {
                self.selected_youth_ranks = cached.data;
            }
            if let Ok(Some(cached)) = self.cache.load_youth_merit_badges(user_id) {
                self.selected_youth_badges = cached.data;
            }
            return;
        }

        let token = match self.session.token() {
            Some(t) => t.to_string(),
            None => return,
        };

        let tx = self.refresh_tx.clone();

        // Try to load from cache first
        if let Ok(Some(cached)) = self.cache.load_youth_ranks(user_id) {
            if !cached.is_stale() {
                self.selected_youth_ranks = cached.data;
            }
        }
        if let Ok(Some(cached)) = self.cache.load_youth_merit_badges(user_id) {
            if !cached.is_stale() {
                self.selected_youth_badges = cached.data;
            }
        }

        // Fetch fresh data in background
        tokio::spawn(async move {
            let api = match create_authenticated_api(token) {
                Ok(api) => api,
                Err(e) => {
                    error!(error = %e, "Failed to create API client for youth progress");
                    return;
                }
            };

            if let Ok(data) = api.fetch_youth_ranks(user_id).await {
                Self::send_result(&tx, RefreshResult::YouthRanks(user_id, data)).await;
            }

            if let Ok(data) = api.fetch_youth_merit_badges(user_id).await {
                Self::send_result(&tx, RefreshResult::YouthMeritBadges(user_id, data)).await;
            }
        });
    }

    /// Fetch leadership history for a specific youth
    pub async fn fetch_youth_leadership(&mut self, user_id: i64) {
        if user_id <= 0 {
            warn!(user_id, "Invalid user_id for youth leadership fetch");
            return;
        }

        // In offline mode, use cached data only
        if self.offline_mode {
            if let Ok(Some(cached)) = self.cache.load_youth_leadership(user_id) {
                self.selected_youth_leadership = cached.data;
            }
            return;
        }

        let token = match self.session.token() {
            Some(t) => t.to_string(),
            None => return,
        };

        let tx = self.refresh_tx.clone();

        // Try to load from cache first
        if let Ok(Some(cached)) = self.cache.load_youth_leadership(user_id) {
            if !cached.is_stale() {
                self.selected_youth_leadership = cached.data;
                return; // Cache is fresh, no need to fetch
            }
        }

        // Fetch fresh data in background
        tokio::spawn(async move {
            let api = match create_authenticated_api(token) {
                Ok(api) => api,
                Err(e) => {
                    error!(error = %e, "Failed to create API client for youth leadership");
                    return;
                }
            };

            if let Ok(data) = api.fetch_youth_leadership(user_id).await {
                Self::send_result(&tx, RefreshResult::YouthLeadership(user_id, data)).await;
            }
        });
    }

    /// Fetch awards for a specific youth
    pub async fn fetch_youth_awards(&mut self, user_id: i64) {
        if user_id <= 0 {
            warn!(user_id, "Invalid user_id for youth awards fetch");
            return;
        }

        // In offline mode, use cached data only
        if self.offline_mode {
            if let Ok(Some(cached)) = self.cache.load_youth_awards(user_id) {
                self.selected_youth_awards = cached.data;
            }
            self.awards_loaded = true;
            return;
        }

        let token = match self.session.token() {
            Some(t) => t.to_string(),
            None => return,
        };

        let tx = self.refresh_tx.clone();

        // Try to load from cache first
        if let Ok(Some(cached)) = self.cache.load_youth_awards(user_id) {
            if !cached.is_stale() {
                self.selected_youth_awards = cached.data;
                self.awards_loaded = true;
                return; // Cache is fresh, no need to fetch
            }
        }

        // Fetch fresh data in background
        tokio::spawn(async move {
            let api = match create_authenticated_api(token) {
                Ok(api) => api,
                Err(e) => {
                    error!(error = %e, "Failed to create API client for youth awards");
                    // Send empty result so UI shows "No awards" instead of loading forever
                    Self::send_result(&tx, RefreshResult::YouthAwards(user_id, vec![])).await;
                    return;
                }
            };

            // Send result even if empty or on error
            let data = api.fetch_youth_awards(user_id).await.unwrap_or_default();
            Self::send_result(&tx, RefreshResult::YouthAwards(user_id, data)).await;
        });
    }

    /// Fetch rank requirements for a specific youth and rank
    pub async fn fetch_rank_requirements(&mut self, user_id: i64, rank_id: i64) {
        if user_id <= 0 || rank_id <= 0 {
            warn!(user_id, rank_id, "Invalid IDs for rank requirements fetch");
            return;
        }

        // Track which rank requirements we're viewing to prevent overwrites from background fetches
        self.viewing_rank_user_id = Some(user_id);
        self.viewing_rank_id = Some(rank_id);

        // In offline mode, use cached data only
        if self.offline_mode {
            if let Ok(Some(cached)) = self.cache.load_rank_requirements(user_id, rank_id) {
                let mut sorted = cached.data;
                sort_requirements(&mut sorted);
                self.selected_rank_requirements = sorted;
                self.viewing_requirements = true;
                self.requirement_selection = 0;
            }
            return;
        }

        let token = match self.session.token() {
            Some(t) => t.to_string(),
            None => return,
        };

        let tx = self.refresh_tx.clone();
        let uid = user_id;
        let rid = rank_id;

        tokio::spawn(async move {
            let api = match create_authenticated_api(token) {
                Ok(api) => api,
                Err(e) => {
                    error!(error = %e, "Failed to create API client for rank requirements");
                    return;
                }
            };

            if let Ok(data) = api.fetch_rank_requirements(uid, rid).await {
                Self::send_result(&tx, RefreshResult::RankRequirements(uid, rid, data)).await;
            }
        });
    }

    /// Fetch badge requirements for a specific youth and badge
    pub async fn fetch_badge_requirements(&mut self, user_id: i64, badge_id: i64) {
        if user_id <= 0 || badge_id <= 0 {
            warn!(user_id, badge_id, "Invalid IDs for badge requirements fetch");
            return;
        }

        // Track which badge requirements we're viewing to prevent overwrites from background fetches
        self.viewing_badge_user_id = Some(user_id);
        self.viewing_badge_id = Some(badge_id);

        // In offline mode, use cached data only
        if self.offline_mode {
            if let Ok(Some(cached)) = self.cache.load_badge_requirements(user_id, badge_id) {
                let (mut reqs, version) = cached.data;
                sort_requirements(&mut reqs);
                self.selected_badge_requirements = reqs;
                self.selected_badge_version = version;
                self.viewing_requirements = true;
                self.requirement_selection = 0;
            }
            return;
        }

        let token = match self.session.token() {
            Some(t) => t.to_string(),
            None => return,
        };

        let tx = self.refresh_tx.clone();
        let uid = user_id;
        let bid = badge_id;

        tokio::spawn(async move {
            let api = match create_authenticated_api(token) {
                Ok(api) => api,
                Err(e) => {
                    error!(error = %e, "Failed to create API client for badge requirements");
                    return;
                }
            };

            if let Ok((reqs, version, counselor)) = api.fetch_badge_requirements(uid, bid).await {
                Self::send_result(&tx, RefreshResult::BadgeRequirements(uid, bid, reqs, version, counselor)).await;
            }
        });
    }

    // =========================================================================
    // Data Access Methods
    // =========================================================================

    /// Get patrol members for a specific patrol
    #[allow(dead_code)]
    pub fn get_patrol_members(&self, patrol_guid: &str) -> Vec<&Youth> {
        self.youth
            .iter()
            .filter(|y| y.patrol_guid.as_deref() == Some(patrol_guid))
            .collect()
    }

    /// Get parents for a specific youth
    pub fn get_parents_for_youth(&self, youth_user_id: i64) -> Vec<&Parent> {
        self.parents
            .iter()
            .filter(|p| p.youth_user_id == Some(youth_user_id))
            .collect()
    }

    /// Check if a youth matches the search query.
    /// Query should already be lowercased.
    fn youth_matches_search(youth: &Youth, query: &str) -> bool {
        youth.matches_search(query)
    }

    /// Get youth sorted by current sort settings, filtered by search query
    pub fn get_sorted_youth(&self) -> Vec<&Youth> {
        let mut sorted: Vec<&Youth> = self.youth.iter().collect();

        // Apply search filter (searches name, patrol, rank, email)
        if !self.search_query.is_empty() {
            let query = self.search_query.to_lowercase();
            sorted.retain(|y| Self::youth_matches_search(y, &query));
        }

        sorted.sort_by(|a, b| {
            let cmp = Youth::cmp_by_column(a, b, self.scout_sort_column);
            if self.scout_sort_ascending { cmp } else { cmp.reverse() }
        });

        sorted
    }

    /// Get youth sorted by rank (highest to lowest), then alphabetically
    pub fn get_youth_by_rank(&self) -> Vec<&Youth> {
        let mut sorted: Vec<&Youth> = self.youth.iter().collect();

        // Apply search filter
        if !self.search_query.is_empty() {
            let query = self.search_query.to_lowercase();
            sorted.retain(|y| Self::youth_matches_search(y, &query));
        }

        sorted.sort_by(|a, b| Youth::cmp_by_column(a, b, ScoutSortColumn::Rank));

        sorted
    }

    /// Get events sorted by current sort settings, filtered by search query
    pub fn get_sorted_events(&self) -> Vec<&Event> {
        let mut sorted: Vec<&Event> = self.events.iter().collect();

        // Apply search filter (searches name, location, type)
        if !self.search_query.is_empty() {
            let query = self.search_query.to_lowercase();
            sorted.retain(|e| e.matches_search(&query));
        }

        sorted.sort_by(|a, b| {
            let cmp = Event::cmp_by_column(a, b, self.event_sort_column);
            if self.event_sort_ascending { cmp } else { cmp.reverse() }
        });

        sorted
    }

    /// Get the unit name for display
    #[allow(dead_code)]
    pub fn unit_name(&self) -> String {
        self.config
            .unit_name
            .clone()
            .unwrap_or_else(|| "Trailcache".to_string())
    }

    // =========================================================================
    // Sort Toggle Helpers
    // =========================================================================

    /// Toggle scout sort column - if already sorting by this column, flip direction;
    /// otherwise switch to this column with ascending=true. Resets selection to 0.
    pub fn toggle_scout_sort(&mut self, column: ScoutSortColumn) {
        if self.scout_sort_column == column {
            self.scout_sort_ascending = !self.scout_sort_ascending;
        } else {
            self.scout_sort_column = column;
            self.scout_sort_ascending = true;
        }
        self.roster_selection = 0;
    }

    /// Toggle event sort column - if already sorting by this column, flip direction;
    /// otherwise switch to this column with ascending=true. Resets selection to 0.
    pub fn toggle_event_sort(&mut self, column: EventSortColumn) {
        if self.event_sort_column == column {
            self.event_sort_ascending = !self.event_sort_ascending;
        } else {
            self.event_sort_column = column;
            self.event_sort_ascending = true;
        }
        self.event_selection = 0;
    }

    /// Toggle ranks tab sort to sort by name. Resets selections.
    pub fn toggle_ranks_sort_by_name(&mut self) {
        if !self.ranks_sort_by_count {
            self.ranks_sort_ascending = !self.ranks_sort_ascending;
        } else {
            self.ranks_sort_by_count = false;
            self.ranks_sort_ascending = true;
        }
        self.ranks_selection = 0;
        self.ranks_scout_selection = 0;
    }

    /// Toggle ranks tab sort to sort by count. Resets selections.
    pub fn toggle_ranks_sort_by_count(&mut self) {
        if self.ranks_sort_by_count {
            self.ranks_sort_ascending = !self.ranks_sort_ascending;
        } else {
            self.ranks_sort_by_count = true;
            self.ranks_sort_ascending = false; // Default descending for count
        }
        self.ranks_selection = 0;
        self.ranks_scout_selection = 0;
    }

    /// Toggle badges tab sort to sort by name. Resets selections.
    pub fn toggle_badges_sort_by_name(&mut self) {
        if !self.badges_sort_by_count {
            self.badges_sort_ascending = !self.badges_sort_ascending;
        } else {
            self.badges_sort_by_count = false;
            self.badges_sort_ascending = true;
        }
        self.badges_selection = 0;
        self.badges_scout_selection = 0;
    }

    /// Toggle badges tab sort to sort by count. Resets selections.
    pub fn toggle_badges_sort_by_count(&mut self) {
        if self.badges_sort_by_count {
            self.badges_sort_ascending = !self.badges_sort_ascending;
        } else {
            self.badges_sort_by_count = true;
            self.badges_sort_ascending = false; // Default descending for count
        }
        self.badges_selection = 0;
        self.badges_scout_selection = 0;
    }
}

// ============================================================================
// Input validation helpers (exported for use in input.rs)
// ============================================================================


/// Check if a character is valid for input (no control characters)
fn is_valid_input_char(c: char) -> bool {
    // Allow printable ASCII and common extended chars, reject control chars
    !c.is_control()
}

/// Check if a username character should be accepted
pub fn can_add_username_char(current_len: usize, c: char) -> bool {
    current_len < MAX_USERNAME_LENGTH && is_valid_input_char(c)
}

/// Check if a password character should be accepted
pub fn can_add_password_char(current_len: usize, c: char) -> bool {
    current_len < MAX_PASSWORD_LENGTH && is_valid_input_char(c)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // ScoutRank Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_scout_rank_from_str_basic() {
        assert_eq!(ScoutRank::parse(Some("Scout")), ScoutRank::Scout);
        assert_eq!(ScoutRank::parse(Some("Tenderfoot")), ScoutRank::Tenderfoot);
        assert_eq!(ScoutRank::parse(Some("Second Class")), ScoutRank::SecondClass);
        assert_eq!(ScoutRank::parse(Some("First Class")), ScoutRank::FirstClass);
        assert_eq!(ScoutRank::parse(Some("Star")), ScoutRank::Star);
        assert_eq!(ScoutRank::parse(Some("Life")), ScoutRank::Life);
        assert_eq!(ScoutRank::parse(Some("Eagle")), ScoutRank::Eagle);
    }

    #[test]
    fn test_scout_rank_from_str_case_insensitive() {
        assert_eq!(ScoutRank::parse(Some("EAGLE")), ScoutRank::Eagle);
        assert_eq!(ScoutRank::parse(Some("eagle")), ScoutRank::Eagle);
        assert_eq!(ScoutRank::parse(Some("Eagle Scout")), ScoutRank::Eagle);
        assert_eq!(ScoutRank::parse(Some("star scout")), ScoutRank::Star);
        assert_eq!(ScoutRank::parse(Some("LIFE SCOUT")), ScoutRank::Life);
    }

    #[test]
    fn test_scout_rank_from_str_contains() {
        // Test that rank names embedded in other text are found
        assert_eq!(ScoutRank::parse(Some("Eagle Scout - Silver Palm")), ScoutRank::Eagle);
        assert_eq!(ScoutRank::parse(Some("Life Scout")), ScoutRank::Life);
    }

    #[test]
    fn test_scout_rank_from_str_unknown() {
        assert_eq!(ScoutRank::parse(Some("")), ScoutRank::Unknown);
        assert_eq!(ScoutRank::parse(Some("Invalid Rank")), ScoutRank::Unknown);
        assert_eq!(ScoutRank::parse(None), ScoutRank::Unknown);
    }

    #[test]
    fn test_scout_rank_ordering() {
        // Verify that ranks are ordered correctly
        assert!(ScoutRank::Eagle > ScoutRank::Life);
        assert!(ScoutRank::Life > ScoutRank::Star);
        assert!(ScoutRank::Star > ScoutRank::FirstClass);
        assert!(ScoutRank::FirstClass > ScoutRank::SecondClass);
        assert!(ScoutRank::SecondClass > ScoutRank::Tenderfoot);
        assert!(ScoutRank::Tenderfoot > ScoutRank::Scout);
        assert!(ScoutRank::Scout > ScoutRank::Unknown);
    }

    // -------------------------------------------------------------------------
    // Tab Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_tab_next() {
        assert_eq!(Tab::Scouts.next(), Tab::Ranks);
        assert_eq!(Tab::Ranks.next(), Tab::Badges);
        assert_eq!(Tab::Badges.next(), Tab::Events);
        assert_eq!(Tab::Events.next(), Tab::Adults);
        assert_eq!(Tab::Adults.next(), Tab::Unit);
        assert_eq!(Tab::Unit.next(), Tab::Scouts); // Wraps around
    }

    #[test]
    fn test_tab_prev() {
        assert_eq!(Tab::Scouts.prev(), Tab::Unit); // Wraps around
        assert_eq!(Tab::Unit.prev(), Tab::Adults);
        assert_eq!(Tab::Adults.prev(), Tab::Events);
        assert_eq!(Tab::Events.prev(), Tab::Badges);
        assert_eq!(Tab::Badges.prev(), Tab::Ranks);
        assert_eq!(Tab::Ranks.prev(), Tab::Scouts);
    }

    // -------------------------------------------------------------------------
    // Input Validation Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_can_add_username_char() {
        // Valid chars within length
        assert!(can_add_username_char(0, 'a'));
        assert!(can_add_username_char(49, 'z'));
        // Exceeds max length
        assert!(!can_add_username_char(50, 'a'));
        assert!(!can_add_username_char(100, 'a'));
        // Control characters rejected
        assert!(!can_add_username_char(0, '\x00'));
        assert!(!can_add_username_char(0, '\n'));
        assert!(!can_add_username_char(0, '\t'));
    }

    #[test]
    fn test_can_add_password_char() {
        // Valid chars within length (now 128 max)
        assert!(can_add_password_char(0, 'a'));
        assert!(can_add_password_char(127, '!'));
        // Exceeds max length
        assert!(!can_add_password_char(128, 'a'));
        assert!(!can_add_password_char(200, 'a'));
        // Control characters rejected
        assert!(!can_add_password_char(0, '\x00'));
        assert!(!can_add_password_char(0, '\r'));
    }
}
