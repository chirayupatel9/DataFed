use crate::config::Config;
use crate::version::{SharedCoreVersionInfo, fetch_core_server_version, create_shared_version_info};
use crate::worker::{self, ZMQInprocMessenger};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub struct RepoServer {
    cfg: Config,
    running: Arc<AtomicBool>,
    workers: Vec<JoinHandle<()>>,
    version_info: SharedCoreVersionInfo,
}

impl RepoServer {
    pub fn new(cfg: Config) -> Self {
        Self {
            cfg,
            running: Arc::new(AtomicBool::new(false)),
            workers: Vec::new(),
            version_info: create_shared_version_info(),
        }
    }

    pub fn run(&mut self) {
        // A) Core version handshake (retry until success)
        while let Err(e) = self.core_handshake() {
            eprintln!("core handshake failed: {e}; retrying…");
            thread::sleep(Duration::from_secs(2));
        }

        // B) Start the secure TCP ↔ inproc proxy via C++ bridge
        // This mimics the C++ ioSecure() method exactly
        println!("Starting ZMQ proxy (TCP ↔ INPROC)...");
        // Start the C++ proxy via FFI bridge
        // The C++ version creates:
        // - TCP server socket (external, secure) on port 10000
        // - INPROC server socket (internal) binding to "workers" endpoint
        // - ProxyCustom to bridge them
        
        // Load repo server keys for authentication (like C++ RepoServer::loadKeys)
        let repo_public_key = self.cfg.load_repo_public_key()
            .unwrap_or_else(|e| panic!("Failed to load repo public key: {}", e));
        let repo_private_key = self.cfg.load_repo_private_key()
            .unwrap_or_else(|e| panic!("Failed to load repo private key: {}", e));
        
        crate::ffi::repo::server_start(&self.cfg.core_server, &repo_public_key, &repo_private_key)
            .unwrap_or_else(|e| panic!("Failed to start ZMQ proxy: {}", e));
        println!("ZMQ proxy started successfully");
        
        // C) Spawn workers (using real ZMQ INPROC messenger)
        let base = std::path::PathBuf::from(self.cfg.globus_collection_path.clone().unwrap_or("/data".into()));
        let version_info = self.version_info.clone();
        
        // Set running flag to true BEFORE spawning workers
        self.running.store(true, Ordering::SeqCst);
        println!("Running flag set to true, spawning workers...");
        
        for i in 0..self.cfg.num_req_worker_threads {
            let mess = ZMQInprocMessenger::new(i); // Real INPROC messenger
            println!("Spawning worker {}", i);
            let h = worker::spawn_worker(
                i,
                mess,
                base.clone(),
                self.running.clone(),
                version_info.clone(),
            );
            self.workers.push(h);
            println!("Worker {} spawned successfully", i);
        }

        // D) Block until shutdown (proxy blocks here in real code)
        println!("Server entering main loop, waiting for shutdown signal...");
        while self.running.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_millis(100));
        }
        println!("Server received shutdown signal, stopping workers...");

        // E) Stop proxy here (and any other IO threads)
        println!("Stopping ZMQ proxy...");
        // Stop the C++ proxy via FFI bridge
        if let Err(e) = crate::ffi::repo::server_stop() {
            eprintln!("Warning: Failed to stop ZMQ proxy: {}", e);
        }
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    pub fn join(self) {
        for h in self.workers {
            let _ = h.join();
        }
    }

    pub fn cfg(&self) -> &Config { &self.cfg }

    fn core_handshake(&self) -> Result<(), String> {
        // Use the new version management system to connect to core server
        match fetch_core_server_version(&self.cfg.core_server, self.version_info.clone(), &self.cfg) {
            Ok(()) => Ok(()),
            Err(e) => Err(format!("Failed to connect to core server: {}", e)),
        }
    }
}
