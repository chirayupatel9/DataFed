use serde::Deserialize;
use std::{fs, path::Path};

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub core_server: String,
    pub cred_dir: String,
    pub port: u16,
    pub timeout: u32,
    pub num_req_worker_threads: usize,
    pub globus_collection_path: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            core_server: "tcp://127.0.0.1:9998".into(),
            cred_dir: "/opt/datafed/credentials".into(),
            port: 9999,
            timeout: 20_000,
            num_req_worker_threads: 4,
            globus_collection_path: None,
        }
    }
}

impl Config {
    pub fn load<P: AsRef<Path>>(p: P) -> Result<Self, String> {
        let txt = fs::read_to_string(&p).map_err(|e| format!("read config: {e}"))?;
        let mut cfg: Self = toml::from_str(&txt)
            .map_err(|e| format!("parse toml: {e}"))?;
        // fill any missing with defaults (if you allow partial files)
        let def = Self::default();
        if cfg.core_server.is_empty() { cfg.core_server = def.core_server; }
        if cfg.cred_dir.is_empty()   { cfg.cred_dir = def.cred_dir; }
        if cfg.port == 0             { cfg.port = def.port; }
        if cfg.timeout == 0          { cfg.timeout = def.timeout; }
        if cfg.num_req_worker_threads == 0 { cfg.num_req_worker_threads = def.num_req_worker_threads; }
        Ok(cfg)
    }
}
