use crate::config::Config;
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

    pub fn start(&mut self) {
        // Spawn placeholder workers to simulate RequestWorker pool.
        // Replace bodies with real logic as you port it from C++.
        for i in 0..self.cfg.num_req_worker_threads {
            let flag = self.shutdown.clone();
            let timeout_ms = self.cfg.timeout;
            self.workers.push(thread::Builder::new()
                .name(format!("req-worker-{i}"))
                .spawn(move || {
                    while !flag.load(Ordering::SeqCst) {
                        // TODO: real request handling (ZeroMQ, protobuf, etc.)
                        std::thread::sleep(Duration::from_millis((timeout_ms/10).max(50) as u64));
                    }
                })
                .expect("spawn worker"));
        }

        // TODO: spawn listener(s) here (e.g., ZeroMQ REP socket) when you port networking
        // Example skeleton loop for a listener thread:
        // let flag = self.shutdown.clone();
        // self.workers.push(thread::spawn(move || {
        //     while !flag.load(Ordering::SeqCst) {
        //         // recv -> route -> send
        //     }
        // }));
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
}
