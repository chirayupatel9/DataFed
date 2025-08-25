use crate::config::Config;
use crate::version;
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
}

impl RepoServer {
    pub fn new(cfg: Config) -> Self {
        Self {
            cfg,
            shutdown: Arc::new(AtomicBool::new(false)),
            workers: Vec::new(),
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
        for i in 0..self.cfg.num_req_worker_threads {
            let mess = NoopMessenger::default(); // TODO: replace with your inproc client
            let h = worker::spawn_worker(
                i,
                mess,
                base.clone(),
                self.shutdown.clone(),
                (version::repo_major(), version::repo_minor(), version::repo_patch()),
                (version::api_major(),  version::api_minor()),
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
        // Parse tcp://host:port
        let parts: Vec<&str> = self.cfg.core_server.split("://").collect();
        if parts.len() != 2 { return Err("invalid core_server URI".into()); }
        let scheme = parts[0];
        let after = parts[1];
        let mut split = after.split(':');
        let host = split.next().unwrap_or("localhost");
        let port: u16 = split.next().unwrap_or("9998").parse().map_err(|_| "bad core_server port")?;

        // Core public key: if you have one, load it here (else empty)
        let core_pub_key = "";

        // Call your C++ bridge to send an anonymous VersionRequest
        let v = unsafe {
            crate::ffi::repo::send_version_request(host, port, scheme, core_pub_key, self.cfg.timeout)
        };

        // Perform compatibility checks if VersionInfo exposes fields.
        // NOTE: If your VersionInfo has getters, adapt this accordingly.
        let api_major_local = version::api_major();
        let api_minor_local = version::api_minor();

        // If your VersionInfo exposes api_major/api_minor, uncomment and use:
        // if v.api_major != api_major_local {
        //     return Err(format!("API major mismatch (core {} != local {})", v.api_major, api_major_local));
        // }
        // if v.api_minor < api_minor_local.saturating_sub(9) {
        //     eprintln!("warning: core API minor {} is far behind local {}", v.api_minor, api_minor_local);
        // }

        Ok(())
    }
}
