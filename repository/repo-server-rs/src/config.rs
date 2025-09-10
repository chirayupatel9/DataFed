// src/config.rs
use std::{fs, path::Path};
use std::io::Write;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
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
            core_server: "tcp://datafed-core:7512".to_string(),
            cred_dir: "/opt/datafed/keys/".to_string(),
            port: 9000,
            timeout: 5000,
            num_req_worker_threads: 4,
            globus_collection_path: Some("/mnt/datafed-repo".to_string()),
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
        let key_path = format!("{}datafed-core-key.pub", self.cred_dir);
        fs::read_to_string(&key_path)
            .map_err(|e| format!("Failed to load core public key from {}: {}", key_path, e))
            .map(|key| key.trim().to_string())
    }

    /// Load the repo server's public key from the credentials directory
    pub fn load_repo_public_key(&self) -> Result<String, String> {
        let key_path = format!("{}datafed-repo-key.pub", self.cred_dir);
        fs::read_to_string(&key_path)
            .map_err(|e| format!("Failed to load repo public key from {}: {}", key_path, e))
            .map(|key| key.trim().to_string())
    }

    /// Load the repo server's private key from the credentials directory
    pub fn load_repo_private_key(&self) -> Result<String, String> {
        let key_path = format!("{}datafed-repo-key.priv", self.cred_dir);
        fs::read_to_string(&key_path)
            .map_err(|e| format!("Failed to load repo private key from {}: {}", key_path, e))
            .map(|key| key.trim().to_string())
    }

    pub fn load<P: AsRef<Path>>(p: P) -> Result<Self, String> {
        // Read the TOML file
        let content = fs::read_to_string(&p)
            .map_err(|e| format!("Failed to read config file {}: {}", p.as_ref().display(), e))?;
        
        // Parse the TOML content
        let mut cfg: Config = toml::from_str(&content)
            .map_err(|e| format!("Failed to parse TOML config: {}", e))?;
        
        // Normalize the configuration
        cfg.normalize();
        
        Ok(cfg)
    }

    /// Generate new server key pair using ZMQ curve keypair
    /// This mimics the C++ generateKeys function from common/source/Util.cpp
    pub fn generate_keys(&self) -> Result<(String, String), String> {
        // Use sodiumoxide for crypto operations (equivalent to zmq_curve_keypair)
        use sodiumoxide::crypto::box_::gen_keypair;
        
        // Initialize sodiumoxide if not already done
        if !sodiumoxide::init().is_ok() {
            return Err("Failed to initialize sodiumoxide".to_string());
        }
        
        // Generate keypair
        let (public_key, secret_key) = gen_keypair();
        
        // Convert to base64 strings (ZMQ curve format)
        let pub_key_str = base64::encode(public_key.0);
        let priv_key_str = base64::encode(secret_key.0);
        
        Ok((pub_key_str, priv_key_str))
    }

    /// Generate and save key files to the credentials directory
    /// This mimics the C++ key generation logic from repository/server/main.cpp
    pub fn generate_and_save_keys(&self) -> Result<(), String> {
        // Ensure credentials directory exists
        if let Err(e) = fs::create_dir_all(&self.cred_dir) {
            return Err(format!("Failed to create credentials directory {}: {}", self.cred_dir, e));
        }

        // Generate key pair
        let (pub_key, priv_key) = self.generate_keys()?;

        // Write public key file
        let pub_key_path = format!("{}datafed-repo-key.pub", self.cred_dir);
        let mut pub_file = fs::File::create(&pub_key_path)
            .map_err(|e| format!("Failed to create public key file {}: {}", pub_key_path, e))?;
        pub_file.write_all(pub_key.as_bytes())
            .map_err(|e| format!("Failed to write public key to {}: {}", pub_key_path, e))?;

        // Write private key file
        let priv_key_path = format!("{}datafed-repo-key.priv", self.cred_dir);
        let mut priv_file = fs::File::create(&priv_key_path)
            .map_err(|e| format!("Failed to create private key file {}: {}", priv_key_path, e))?;
        priv_file.write_all(priv_key.as_bytes())
            .map_err(|e| format!("Failed to write private key to {}: {}", priv_key_path, e))?;

        println!("Generated keys successfully:");
        println!("  Public key: {}", pub_key_path);
        println!("  Private key: {}", priv_key_path);

        Ok(())
    }
}
