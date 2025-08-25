use crate::config::Config;
use crate::version::{self, SharedCoreVersionInfo, fetch_core_server_version, create_shared_version_info};
use crate::worker::{self, NoopMessenger};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub struct RepoServer {
    cfg: Config,
    shutdown: Arc<AtomicBool>,
    workers: Vec<JoinHandle<()>>,
    version_info: SharedCoreVersionInfo,
}

impl RepoServer {
    pub fn new(cfg: Config) -> Self {
        Self {
            cfg,
            shutdown: Arc::new(AtomicBool::new(false)),
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

        // B) TODO: Start your secure TCP ↔ inproc proxy via C++ bridge here.
        //    Keep a handle so you can stop it on shutdown.

        // C) Spawn workers (using a NoopMessenger so we compile/runs idle)
        let base = std::path::PathBuf::from(self.cfg.globus_collection_path.clone().unwrap_or("/data".into()));
        let version_info = self.version_info.clone();
        
        for i in 0..self.cfg.num_req_worker_threads {
            let mess = NoopMessenger::default(); // TODO: replace with your inproc client
            let h = worker::spawn_worker(
                i,
                mess,
                base.clone(),
                self.shutdown.clone(),
                version_info.clone(),
            );
            self.workers.push(h);
        }

        // D) Block until shutdown (your proxy may block here in real code)
        while !self.shutdown.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_millis(100));
        }

        // E) TODO: Stop proxy here (and any other IO threads)
    }

    pub fn stop(&self) {
        self.shutdown.store(true, Ordering::SeqCst);
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
