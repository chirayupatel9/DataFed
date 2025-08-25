// src/config.rs
use std::{fs, path::Path};

#[derive(Clone, Debug)]
pub struct Config {
    pub core_server: String,               // e.g., tcp://localhost:9998
    pub cred_dir: String,                  // ensure trailing '/'
    pub port: u16,                         // external listen port
    pub timeout: u32,                      // ms
    pub num_req_worker_threads: usize,     // worker threads
    pub globus_collection_path: Option<String>, // base path (no trailing '/')
}

impl Default for Config {
    fn default() -> Self {
        Self {
            core_server: "tcp://localhost:9998".to_string(),
            cred_dir: "/mnt/storage/rust/DataFed/".to_string(),
            port: 9000,
            timeout: 5000,
            num_req_worker_threads: 4,
            globus_collection_path: Some("/data".to_string()),
        }
    }
}

impl Config {
    pub fn normalize(&mut self) {
        if let Some(p) = &mut self.globus_collection_path {
            while p.ends_with('/') && p.len() > 1 { p.pop(); }
        }
        if !self.cred_dir.ends_with('/') { self.cred_dir.push('/'); }
    }

    /// Load the core server's public key from the credentials directory
    pub fn load_core_public_key(&self) -> Result<String, String> {
        let key_path = format!("{}mock-datafed-core-key.pub", self.cred_dir);
        fs::read_to_string(&key_path)
            .map_err(|e| format!("Failed to load core public key from {}: {}", key_path, e))
            .map(|key| key.trim().to_string())
    }

    pub fn load<P: AsRef<Path>>(p: P) -> Result<Self, String> {
        // If you have a real config format, parse it here.
        // For now, load defaults and normalize.
        let _ = fs::read_to_string(&p).map_err(|e| format!("read config: {e}"))?;
        let mut cfg = Self::default();
        cfg.normalize();
        Ok(cfg)
    }
}
