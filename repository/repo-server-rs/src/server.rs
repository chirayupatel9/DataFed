use crate::{config::Config, ffi::CxxLogContext};
use crate::request_worker::RequestWorker;
use crate::ffi::{repo_log_info, check_core_server_version};

pub struct Server {
    cfg: &'static Config,
    log: CxxLogContext,
    workers: Vec<RequestWorker>,
}

impl Server {
    pub fn new(log: CxxLogContext) -> Self {
        Self { cfg: Config::get(), log, workers: Vec::new() }
    }

    pub fn run(mut self) {
        self.check_server_version();
        repo_log_info(&self.log.thread_name, &self.log.correlation_id, self.log.thread_id, &format!("Public/private MAPI starting on port {}", self.cfg.port));

        for t in 0..self.cfg.num_req_worker_threads {
            let lw = CxxLogContext { thread_name: "repo_server-worker_thread".into(), thread_id: (t as u64)+1, correlation_id: String::new() };
            self.workers.push(RequestWorker::spawn(lw));
        }

        self.io_secure();

        for w in self.workers { w.stop_and_join(); }
    }

    fn check_server_version(&self) {
        repo_log_info(&self.log.thread_name, &self.log.correlation_id, self.log.thread_id, &format!("Checking core server connection and version at {}", self.cfg.core_server));
        
        // Use the FFI function to check the core server version
        let version_check_success = check_core_server_version(
            &self.cfg.core_server,
            &self.log.thread_name,
            &self.log.correlation_id,
            self.log.thread_id,
        );
        
        if version_check_success {
            repo_log_info(&self.log.thread_name, &self.log.correlation_id, self.log.thread_id, "Core server version check completed successfully");
        } else {
            repo_log_info(&self.log.thread_name, &self.log.correlation_id, self.log.thread_id, "Core server version check failed - continuing anyway");
        }
    }

    fn io_secure(&mut self) {
        // TODO: Expose your C++ proxy via FFI and run it here so it blocks.
        std::thread::park();
    }
}