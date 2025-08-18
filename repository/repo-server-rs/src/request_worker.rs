use crate::ffi::CxxLogContext;
use crate::ffi::repo_log_info;
use crate::config::Config;
use std::thread::{self, JoinHandle};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

pub struct RequestWorker {
    run: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl RequestWorker {
    pub fn spawn(log: CxxLogContext) -> Self {
        let run = Arc::new(AtomicBool::new(true));
        let run_cloned = run.clone();
        let _cfg = Config::get().clone();
        let handle = thread::spawn(move || {
            repo_log_info(&log.thread_name, &log.correlation_id, log.thread_id, &format!("Thread started (tid={})", log.thread_id));
            while run_cloned.load(Ordering::Relaxed) {
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            repo_log_info(&log.thread_name, &log.correlation_id, log.thread_id, "Thread exiting.");
        });
        Self { run, handle: Some(handle) }
    }

    pub fn stop_and_join(mut self) {
        self.run.store(false, Ordering::Relaxed);
        if let Some(h) = self.handle.take() { let _ = h.join(); }
    }
}