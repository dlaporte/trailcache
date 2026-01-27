//! Application state management for Trailcache.
//!
//! This module contains the core `App` struct that manages all application state,
//! including UI state, cached data, session management, and background task coordination.

use std::collections::HashMap;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use futures::stream::{self, StreamExt};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::api::ApiClient;
use crate::auth::{CredentialStore, Session};
use crate::cache::CacheManager;
use crate::config::Config;

use crate::models::{
    Adult, AdvancementDashboard, Commissioner, Event, EventGuest, EventSortColumn,
    Key3Leaders, MeritBadgeProgress, MeritBadgeRequirement, OrgProfile, Parent, Patrol,
    RankProgress, RankRequirement, ReadyToAward, ScoutSortColumn, UnitInfo, Youth,
};
use crate::utils::{cmp_ignore_case, contains_ignore_case};

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
// Rank Ordering
// ============================================================================

/// Scout rank for sorting purposes
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScoutRank {
    Unknown = 0,
    Scout = 1,
    Tenderfoot = 2,
    SecondClass = 3,
    FirstClass = 4,
    Star = 5,
    Life = 6,
    Eagle = 7,
}

impl ScoutRank {
    /// Parse a rank string into a ScoutRank enum value.
    /// Handles variations like "Eagle Scout", "Life Scout", etc.
    pub fn from_str(s: Option<&str>) -> Self {
        match s {
            Some(rank) => {
                let lower = rank.to_lowercase();
                if lower.contains("eagle") {
                    ScoutRank::Eagle
                } else if lower.contains("life") {
                    ScoutRank::Life
                } else if lower.contains("star") {
                    ScoutRank::Star
                } else if lower.contains("first class") {
                    ScoutRank::FirstClass
                } else if lower.contains("second class") {
                    ScoutRank::SecondClass
                } else if lower.contains("tenderfoot") {
                    ScoutRank::Tenderfoot
                } else if lower == "scout" {
                    ScoutRank::Scout
                } else {
                    ScoutRank::Unknown
                }
            }
            None => ScoutRank::Unknown,
        }
    }

    /// Get the numeric order for sorting (0 = Unknown/Crossover, 7 = Eagle).
    pub fn order(&self) -> usize {
        *self as usize
    }

    /// Get the display name for this rank.
    pub fn display_name(&self) -> &'static str {
        match self {
            ScoutRank::Unknown => "Crossover",
            ScoutRank::Scout => "Scout",
            ScoutRank::Tenderfoot => "Tenderfoot",
            ScoutRank::SecondClass => "Second Class",
            ScoutRank::FirstClass => "First Class",
            ScoutRank::Star => "Star",
            ScoutRank::Life => "Life",
            ScoutRank::Eagle => "Eagle",
        }
    }

}

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
    /// Requirements for a specific rank (user_id, rank_id, requirements)
    RankRequirements(i64, i64, Vec<RankRequirement>),
    /// Requirements for a specific merit badge (user_id, badge_id, requirements, version)
    BadgeRequirements(i64, i64, Vec<MeritBadgeRequirement>, Option<String>),
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
    /// An error occurred during refresh
    Error(String),
}

// ============================================================================
// Main Application Struct
// ============================================================================

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
    pub selected_rank_requirements: Vec<RankRequirement>,
    pub selected_badge_requirements: Vec<MeritBadgeRequirement>,
    pub selected_badge_version: Option<String>,
    pub viewing_requirements: bool,
    pub requirement_selection: usize,

    // Background task channel
    refresh_rx: Option<mpsc::Receiver<RefreshResult>>,
    refresh_tx: mpsc::Sender<RefreshResult>,

    // Status message
    pub status_message: Option<String>,

    // Cache ages for status bar
    pub cache_ages: crate::cache::manager::CacheAges,

    // Offline mode - when true, only use cached data
    pub offline_mode: bool,

    // Flag to trigger requirements fetch after main refresh completes
    pending_offline_requirements_fetch: bool,
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
        debug!(org_guid = ?config.organization_guid, "Config loaded");

        let cache_dir = config.cache_dir().unwrap_or_else(|_| PathBuf::from("./cache"));
        debug!(?cache_dir, "Cache directory configured");

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

        let cache = CacheManager::new(cache_dir)?;

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
            ranks_sort_ascending: true,

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
            selected_rank_requirements: Vec::new(),
            selected_badge_requirements: Vec::new(),
            selected_badge_version: None,
            viewing_requirements: false,
            requirement_selection: 0,

            refresh_rx: Some(rx),
            refresh_tx: tx,

            status_message: None,
            cache_ages: Default::default(),
            offline_mode,
            pending_offline_requirements_fetch: false,
        })
    }

    // =========================================================================
    // Authentication
    // =========================================================================

    /// Check if the user is authenticated with a valid session
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

        match self.api.authenticate(&username, &password).await {
            Ok(session_data) => {
                if let Err(e) = CredentialStore::store(&username, &password) {
                    warn!(error = %e, "Failed to store credentials");
                }

                self.config.last_username = Some(username);
                self.config.organization_guid = Some(session_data.organization_guid.clone());

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

                self.login_password.clear();
                self.state = AppState::Normal;
                info!("Login successful");
                Ok(())
            }
            Err(e) => {
                error!(error = %e, "Login failed");
                // Provide user-friendly error messages based on error type
                let user_message = if e.to_string().contains("401")
                    || e.to_string().to_lowercase().contains("unauthorized")
                {
                    "Invalid username or password".to_string()
                } else if e.to_string().to_lowercase().contains("network")
                    || e.to_string().to_lowercase().contains("connect")
                {
                    "Unable to connect to server. Check your internet connection.".to_string()
                } else if e.to_string().to_lowercase().contains("timeout") {
                    "Connection timed out. Please try again.".to_string()
                } else {
                    format!("Login failed: {}", e)
                };
                self.login_error = Some(user_message);
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
    pub async fn load_from_cache(&mut self) -> Result<()> {
        if let Ok(Some(cached)) = self.cache.load_youth() {
            self.youth = cached.data;
        }

        if let Ok(Some(cached)) = self.cache.load_adults() {
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
            self.unit_info = Some(cached.data);
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

        tokio::spawn(async move {
            Self::execute_background_refresh(tx, org_guid, token, user_id).await;
        });

        self.offline_mode = true;
        self.config.offline_mode = true;
        let _ = self.config.save();
        self.pending_offline_requirements_fetch = true;
        self.status_message = Some("Caching data for offline mode...".to_string());
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

        // Create one base API client and clone for parallel tasks.
        // Cloning is cheap - shares the underlying connection pool via Arc.
        let base_api = match ApiClient::new() {
            Ok(api) => api,
            Err(e) => {
                error!(error = %e, "Failed to create API client");
                Self::send_result(&tx, RefreshResult::Error("Failed to create API client".to_string())).await;
                return;
            }
        };

        // Clone the base client for each parallel fetch.
        // This is very efficient - both client and token use Arc internally,
        // so cloning is just incrementing reference counts (no String allocation).
        let api1 = base_api.with_token(Arc::clone(&token));
        let api2 = base_api.with_token(Arc::clone(&token));
        let api3 = base_api.with_token(Arc::clone(&token));
        let api4 = base_api.with_token(Arc::clone(&token));
        let api5 = base_api.with_token(Arc::clone(&token));
        let api6 = base_api.with_token(Arc::clone(&token));
        let api7 = base_api.with_token(Arc::clone(&token));
        let api8 = base_api.with_token(Arc::clone(&token));
        let api9 = base_api.with_token(Arc::clone(&token));
        let api10 = base_api.with_token(Arc::clone(&token));

        // Clone org_guid for each task
        let org1 = Arc::clone(&org_guid);
        let org2 = Arc::clone(&org_guid);
        let org3 = Arc::clone(&org_guid);
        let org4 = Arc::clone(&org_guid);
        let org5 = Arc::clone(&org_guid);
        let org6 = Arc::clone(&org_guid);
        let org7 = Arc::clone(&org_guid);
        let org8 = Arc::clone(&org_guid);

        // Fetch all main data in parallel
        let (youth_res, adults_res, parents_res, patrols_res, events_res, dashboard_res, ready_res, key3_res, pin_res, profile_res) = tokio::join!(
            api1.fetch_youth(&org1),
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

        // Extract youth user IDs before moving youth_res (for advancement fetch later)
        let youth_user_ids: Vec<i64> = youth_res
            .as_ref()
            .map(|list| list.iter().filter_map(|y| y.user_id).collect())
            .unwrap_or_default();

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

        // Fetch commissioners separately using a clone of the base client
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

        // Fetch rank and merit badge progress for all youth (for Ranks/Badges tabs)
        Self::handle_all_youth_advancement_refresh(&tx, &youth_user_ids, &token).await;

        info!("Background refresh complete");
        Self::send_result(&tx, RefreshResult::RefreshComplete).await;
    }

    /// Fetch rank and merit badge progress for all youth members.
    /// This populates all_youth_ranks and all_youth_badges HashMaps for the Ranks/Badges tabs.
    async fn handle_all_youth_advancement_refresh(
        tx: &mpsc::Sender<RefreshResult>,
        user_ids: &[i64],
        token: &Arc<String>,
    ) {
        if user_ids.is_empty() {
            return;
        }

        debug!(count = user_ids.len(), "Fetching ranks and badges for all youth");

        // Create API client with Arc-wrapped token (cheap clone)
        let api = match ApiClient::new() {
            Ok(mut api) => {
                api.set_token(Arc::clone(token));
                api
            }
            Err(e) => {
                error!(error = %e, "Failed to create API client for youth advancement");
                return;
            }
        };

        // Fetch ranks and badges for all youth with limited concurrency
        const MAX_CONCURRENT: usize = 5;
        for chunk in user_ids.chunks(MAX_CONCURRENT) {
            let futures: Vec<_> = chunk
                .iter()
                .map(|&user_id| {
                    let api = api.clone();
                    async move {
                        let ranks = api.fetch_youth_ranks(user_id).await.ok();
                        let badges = api.fetch_youth_merit_badges(user_id).await.ok();
                        (user_id, ranks, badges)
                    }
                })
                .collect();

            let results = futures::future::join_all(futures).await;
            for (user_id, ranks, badges) in results {
                if let Some(ranks) = ranks {
                    Self::send_result(tx, RefreshResult::YouthRanks(user_id, ranks)).await;
                }
                if let Some(badges) = badges {
                    Self::send_result(tx, RefreshResult::YouthMeritBadges(user_id, badges)).await;
                }
            }
        }

        debug!("All youth advancement fetching complete");
    }

    /// Fetch all requirements for offline mode.
    /// This fetches rank and badge requirements for all youth's active ranks and badges.
    async fn fetch_all_requirements_for_offline(
        tx: &mpsc::Sender<RefreshResult>,
        all_youth_ranks: &HashMap<i64, Vec<RankProgress>>,
        all_youth_badges: &HashMap<i64, Vec<MeritBadgeProgress>>,
        token: &Arc<String>,
    ) {
        info!("Fetching all requirements for offline mode");

        // Create API client with Arc-wrapped token (cheap clone)
        let api = match ApiClient::new() {
            Ok(mut api) => {
                api.set_token(Arc::clone(token));
                api
            }
            Err(e) => {
                error!(error = %e, "Failed to create API client for requirements fetch");
                return;
            }
        };

        // Collect all rank requirements to fetch (user_id, rank_id)
        // Fetch ALL ranks for complete offline access
        let mut rank_reqs_to_fetch: Vec<(i64, i64)> = Vec::new();
        for (&user_id, ranks) in all_youth_ranks.iter() {
            for rank in ranks {
                rank_reqs_to_fetch.push((user_id, rank.rank_id));
            }
        }

        // Collect all badge requirements to fetch (user_id, badge_id)
        // Fetch ALL badges for complete offline access
        let mut badge_reqs_to_fetch: Vec<(i64, i64)> = Vec::new();
        for (&user_id, badges) in all_youth_badges.iter() {
            for badge in badges {
                badge_reqs_to_fetch.push((user_id, badge.id));
            }
        }

        info!(
            rank_count = rank_reqs_to_fetch.len(),
            badge_count = badge_reqs_to_fetch.len(),
            "Fetching requirements for offline"
        );

        // Fetch rank requirements with limited concurrency
        const MAX_CONCURRENT: usize = 5;
        for chunk in rank_reqs_to_fetch.chunks(MAX_CONCURRENT) {
            let futures: Vec<_> = chunk
                .iter()
                .map(|&(user_id, rank_id)| {
                    let api = api.clone();
                    async move {
                        let reqs = api.fetch_rank_requirements(user_id, rank_id).await.ok();
                        (user_id, rank_id, reqs)
                    }
                })
                .collect();

            let results = futures::future::join_all(futures).await;
            for (user_id, rank_id, reqs) in results {
                if let Some(reqs) = reqs {
                    Self::send_result(tx, RefreshResult::RankRequirements(user_id, rank_id, reqs)).await;
                }
            }
        }

        // Fetch badge requirements with limited concurrency
        for chunk in badge_reqs_to_fetch.chunks(MAX_CONCURRENT) {
            let futures: Vec<_> = chunk
                .iter()
                .map(|&(user_id, badge_id)| {
                    let api = api.clone();
                    async move {
                        let result = api.fetch_badge_requirements(user_id, badge_id).await.ok();
                        (user_id, badge_id, result)
                    }
                })
                .collect();

            let results = futures::future::join_all(futures).await;
            for (user_id, badge_id, result) in results {
                if let Some((reqs, version)) = result {
                    Self::send_result(tx, RefreshResult::BadgeRequirements(user_id, badge_id, reqs, version)).await;
                }
            }
        }

        info!("All requirements fetching complete");
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
                debug!("{} fetched successfully", name);
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
                if let Err(e) = self.cache.save_youth(&data) {
                    warn!(error = %e, "Failed to cache youth data");
                }
                self.youth = data;
                self.cache_ages = self.cache.get_cache_ages();
            }
            RefreshResult::Adults(data) => {
                let deduped = Self::deduplicate_adults(data);
                if let Err(e) = self.cache.save_adults(&deduped) {
                    warn!(error = %e, "Failed to cache adults data");
                }
                self.adults = deduped;
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
                self.unit_info = Some(data);
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
                self.selected_youth_ranks = data;
            }
            RefreshResult::YouthMeritBadges(user_id, data) => {
                if let Err(e) = self.cache.save_youth_merit_badges(user_id, &data) {
                    warn!(error = %e, "Failed to cache youth merit badges");
                }
                // Store in all_youth_badges for the Badges tab aggregate view
                self.all_youth_badges.insert(user_id, data.clone());
                self.selected_youth_badges = data;
            }
            RefreshResult::RankRequirements(user_id, rank_id, data) => {
                // Cache the requirements
                if let Err(e) = self.cache.save_rank_requirements(user_id, rank_id, &data) {
                    warn!(error = %e, "Failed to cache rank requirements");
                }
                self.selected_rank_requirements = data;
                self.viewing_requirements = true;
                self.requirement_selection = 0;
            }
            RefreshResult::BadgeRequirements(user_id, badge_id, data, version) => {
                // Cache the requirements
                if let Err(e) = self.cache.save_badge_requirements(user_id, badge_id, &data, &version) {
                    warn!(error = %e, "Failed to cache badge requirements");
                }
                self.selected_badge_requirements = data;
                self.selected_badge_version = version;
                self.viewing_requirements = true;
                self.requirement_selection = 0;
            }
            RefreshResult::RefreshComplete => {
                // Check if we need to fetch all requirements for offline mode
                if self.pending_offline_requirements_fetch {
                    self.pending_offline_requirements_fetch = false;
                    self.status_message = Some("Caching requirements for offline...".to_string());

                    // Clone data needed for the async task
                    let tx = self.refresh_tx.clone();
                    let all_youth_ranks = self.all_youth_ranks.clone();
                    let all_youth_badges = self.all_youth_badges.clone();
                    let token = match self.session.token() {
                        Some(t) => Arc::new(t.to_string()),
                        None => {
                            self.status_message = Some("Offline mode ready (no requirements cached)".to_string());
                            return;
                        }
                    };

                    tokio::spawn(async move {
                        Self::fetch_all_requirements_for_offline(&tx, &all_youth_ranks, &all_youth_badges, &token).await;
                        Self::send_result(&tx, RefreshResult::RefreshComplete).await;
                    });
                } else {
                    // Only clear status if it's a progress message, preserve errors
                    if let Some(ref msg) = self.status_message {
                        if !msg.starts_with("Error:") {
                            self.status_message = None;
                        }
                    }
                }
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
            let api = match ApiClient::new() {
                Ok(mut api) => {
                    api.set_token((*token).clone());
                    api
                }
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
        let token = match self.session.token() {
            Some(t) => t.to_string(),
            None => return,
        };

        let tx = self.refresh_tx.clone();

        tokio::spawn(async move {
            let api = match ApiClient::new() {
                Ok(mut api) => {
                    api.set_token(token);
                    api
                }
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
            let api = match ApiClient::new() {
                Ok(mut api) => {
                    api.set_token(token);
                    api
                }
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

    /// Fetch rank requirements for a specific youth and rank
    pub async fn fetch_rank_requirements(&mut self, user_id: i64, rank_id: i64) {
        if user_id <= 0 || rank_id <= 0 {
            warn!(user_id, rank_id, "Invalid IDs for rank requirements fetch");
            return;
        }

        // In offline mode, use cached data only
        if self.offline_mode {
            if let Ok(Some(cached)) = self.cache.load_rank_requirements(user_id, rank_id) {
                self.selected_rank_requirements = cached.data;
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
            let api = match ApiClient::new() {
                Ok(mut api) => {
                    api.set_token(token);
                    api
                }
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

        // In offline mode, use cached data only
        if self.offline_mode {
            if let Ok(Some(cached)) = self.cache.load_badge_requirements(user_id, badge_id) {
                let (reqs, version) = cached.data;
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
            let api = match ApiClient::new() {
                Ok(mut api) => {
                    api.set_token(token);
                    api
                }
                Err(e) => {
                    error!(error = %e, "Failed to create API client for badge requirements");
                    return;
                }
            };

            if let Ok((reqs, version)) = api.fetch_badge_requirements(uid, bid).await {
                Self::send_result(&tx, RefreshResult::BadgeRequirements(uid, bid, reqs, version)).await;
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
        contains_ignore_case(&youth.first_name, query)
            || contains_ignore_case(&youth.last_name, query)
            || youth.patrol_name
                .as_ref()
                .map(|s| contains_ignore_case(s, query))
                .unwrap_or(false)
            || youth.current_rank
                .as_ref()
                .map(|s| contains_ignore_case(s, query))
                .unwrap_or(false)
            || youth.email()
                .as_ref()
                .map(|s| contains_ignore_case(s, query))
                .unwrap_or(false)
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
            // Case-insensitive name comparison without allocation
            let name_cmp = |x: &Youth, y: &Youth| {
                cmp_ignore_case(&x.last_name, &y.last_name)
                    .then_with(|| cmp_ignore_case(&x.first_name, &y.first_name))
            };

            let cmp = match self.scout_sort_column {
                ScoutSortColumn::Name => name_cmp(a, b),
                ScoutSortColumn::Rank => {
                    let rank_a = ScoutRank::from_str(a.current_rank.as_deref());
                    let rank_b = ScoutRank::from_str(b.current_rank.as_deref());
                    // Reversed so ascending shows highest rank first
                    rank_b.cmp(&rank_a).then_with(|| name_cmp(a, b))
                }
                ScoutSortColumn::Grade => a.grade.cmp(&b.grade).then_with(|| name_cmp(a, b)),
                ScoutSortColumn::Age => a.age().cmp(&b.age()).then_with(|| name_cmp(a, b)),
                ScoutSortColumn::Patrol => {
                    cmp_ignore_case(
                        a.patrol_name.as_deref().unwrap_or(""),
                        b.patrol_name.as_deref().unwrap_or(""),
                    )
                    .then_with(|| name_cmp(a, b))
                }
            };

            if self.scout_sort_ascending {
                cmp
            } else {
                cmp.reverse()
            }
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

        sorted.sort_by(|a, b| {
            let rank_a = ScoutRank::from_str(a.current_rank.as_deref());
            let rank_b = ScoutRank::from_str(b.current_rank.as_deref());

            rank_b.cmp(&rank_a)
                .then_with(|| cmp_ignore_case(&a.last_name, &b.last_name))
                .then_with(|| cmp_ignore_case(&a.first_name, &b.first_name))
        });

        sorted
    }

    /// Get events sorted by current sort settings, filtered by search query
    pub fn get_sorted_events(&self) -> Vec<&Event> {
        let mut sorted: Vec<&Event> = self.events.iter().collect();

        // Apply search filter (searches name, location, type)
        if !self.search_query.is_empty() {
            let query = self.search_query.to_lowercase();
            sorted.retain(|e| {
                contains_ignore_case(&e.name, &query)
                    || e.location
                        .as_ref()
                        .map(|s| contains_ignore_case(s, &query))
                        .unwrap_or(false)
                    || contains_ignore_case(e.derived_type(), &query)
            });
        }

        sorted.sort_by(|a, b| {
            let name_cmp = |x: &Event, y: &Event| cmp_ignore_case(&x.name, &y.name);

            let cmp = match self.event_sort_column {
                EventSortColumn::Name => name_cmp(a, b),
                EventSortColumn::Date => {
                    let date_a = a.start_date.as_deref().unwrap_or("");
                    let date_b = b.start_date.as_deref().unwrap_or("");
                    date_a.cmp(date_b).then_with(|| name_cmp(a, b))
                }
                EventSortColumn::Location => {
                    cmp_ignore_case(
                        a.location.as_deref().unwrap_or(""),
                        b.location.as_deref().unwrap_or(""),
                    )
                    .then_with(|| name_cmp(a, b))
                }
                EventSortColumn::Type => {
                    cmp_ignore_case(a.derived_type(), b.derived_type())
                        .then_with(|| name_cmp(a, b))
                }
            };

            if self.event_sort_ascending {
                cmp
            } else {
                cmp.reverse()
            }
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

    /// Deduplicate adults by person_guid, combining multiple positions.
    ///
    /// The Scouting API returns duplicate entries for adults who hold multiple
    /// positions (e.g., both "Assistant Scoutmaster" and "Committee Member").
    /// This function merges those duplicates into single entries with combined
    /// position strings (e.g., "Assistant Scoutmaster, Committee Member").
    ///
    /// Adults without a person_guid are kept as separate entries to avoid
    /// incorrectly merging unrelated records.
    fn deduplicate_adults(adults: Vec<Adult>) -> Vec<Adult> {
        let mut by_guid: HashMap<String, Adult> = HashMap::new();
        let mut no_guid_counter: usize = 0;

        for adult in adults {
            let guid = adult.person_guid.clone().unwrap_or_default();
            if guid.is_empty() {
                // Use a separate counter for deterministic synthetic keys
                by_guid.insert(format!("_no_guid_{}", no_guid_counter), adult);
                no_guid_counter += 1;
                continue;
            }

            if let Some(existing) = by_guid.get_mut(&guid) {
                if let Some(new_pos) = &adult.position {
                    if let Some(existing_pos) = &existing.position {
                        // Split existing positions and check for exact match
                        let existing_positions: Vec<&str> = existing_pos
                            .split(", ")
                            .map(|s| s.trim())
                            .collect();
                        if !existing_positions.contains(&new_pos.as_str()) {
                            existing.position = Some(format!("{}, {}", existing_pos, new_pos));
                        }
                    } else {
                        existing.position = Some(new_pos.clone());
                    }
                }
            } else {
                by_guid.insert(guid, adult);
            }
        }

        let mut result: Vec<Adult> = by_guid.into_values().collect();
        result.sort_by(|a, b| a.last_name.cmp(&b.last_name).then(a.first_name.cmp(&b.first_name)));
        result
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
        assert_eq!(ScoutRank::from_str(Some("Scout")), ScoutRank::Scout);
        assert_eq!(ScoutRank::from_str(Some("Tenderfoot")), ScoutRank::Tenderfoot);
        assert_eq!(ScoutRank::from_str(Some("Second Class")), ScoutRank::SecondClass);
        assert_eq!(ScoutRank::from_str(Some("First Class")), ScoutRank::FirstClass);
        assert_eq!(ScoutRank::from_str(Some("Star")), ScoutRank::Star);
        assert_eq!(ScoutRank::from_str(Some("Life")), ScoutRank::Life);
        assert_eq!(ScoutRank::from_str(Some("Eagle")), ScoutRank::Eagle);
    }

    #[test]
    fn test_scout_rank_from_str_case_insensitive() {
        assert_eq!(ScoutRank::from_str(Some("EAGLE")), ScoutRank::Eagle);
        assert_eq!(ScoutRank::from_str(Some("eagle")), ScoutRank::Eagle);
        assert_eq!(ScoutRank::from_str(Some("Eagle Scout")), ScoutRank::Eagle);
        assert_eq!(ScoutRank::from_str(Some("star scout")), ScoutRank::Star);
        assert_eq!(ScoutRank::from_str(Some("LIFE SCOUT")), ScoutRank::Life);
    }

    #[test]
    fn test_scout_rank_from_str_contains() {
        // Test that rank names embedded in other text are found
        assert_eq!(ScoutRank::from_str(Some("Eagle Scout - Silver Palm")), ScoutRank::Eagle);
        assert_eq!(ScoutRank::from_str(Some("Life Scout")), ScoutRank::Life);
    }

    #[test]
    fn test_scout_rank_from_str_unknown() {
        assert_eq!(ScoutRank::from_str(Some("")), ScoutRank::Unknown);
        assert_eq!(ScoutRank::from_str(Some("Invalid Rank")), ScoutRank::Unknown);
        assert_eq!(ScoutRank::from_str(None), ScoutRank::Unknown);
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
