use std::sync::OnceLock;

#[derive(Debug, Clone)]
pub struct Config {
    pub cred_dir: String,
    pub port: u16,
    pub core_server: String,
    pub globus_collection_path: String,
    pub num_req_worker_threads: u32,
}

static CONFIG: OnceLock<Config> = OnceLock::new();

impl Config {
    pub fn get() -> &'static Config {
        CONFIG.get().expect("Config not initialized")
    }
    pub fn init(cfg: Config) {
        let _ = CONFIG.set(cfg);
    }
}