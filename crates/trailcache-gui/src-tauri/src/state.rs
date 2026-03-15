use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::Mutex;

use trailcache_core::api::ApiClient;
use trailcache_core::auth::Session;
use trailcache_core::cache::CacheManager;
use trailcache_core::config::Config;

/// GUI application state managed by Tauri.
/// All fields wrapped in Arc<Mutex<>> for safe concurrent access from commands.
pub struct GuiAppState {
    pub api_client: Arc<Mutex<ApiClient>>,
    pub session: Arc<Mutex<Session>>,
    pub cache: Arc<Mutex<CacheManager>>,
    pub config: Arc<Mutex<Config>>,
}

impl GuiAppState {
    pub fn new(config_dir: Option<PathBuf>, cache_base_dir: Option<PathBuf>) -> anyhow::Result<Self> {
        let mut config = if let Some(ref dir) = config_dir {
            Config::load_from(dir.clone())?
        } else {
            Config::load()?
        };

        if let Some(dir) = cache_base_dir {
            config.set_cache_dir(dir);
        }

        let cache_dir = config.cache_dir().unwrap_or_else(|_| PathBuf::from("./cache"));

        let api_client = ApiClient::new()?;
        let session = Session::new(cache_dir.clone());
        let cache = CacheManager::new_without_encryption(cache_dir)?;

        Ok(Self {
            api_client: Arc::new(Mutex::new(api_client)),
            session: Arc::new(Mutex::new(session)),
            cache: Arc::new(Mutex::new(cache)),
            config: Arc::new(Mutex::new(config)),
        })
    }
}
