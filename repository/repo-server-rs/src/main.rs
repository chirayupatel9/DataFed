mod ffi;
mod config;
mod server;

use config::Config;
use server::RepoServer;

use ffi::dynalog::{self, LogCtx};
use ffi::dynalog::level;
use ffi::repo::send_version_request;
use std::sync::Arc;
use std::{env, process};

fn print_usage() {
    eprintln!(
"Usage:
  repo-server-rs version
  repo-server-rs run <config_path>"
    );
}

fn main() {
    // logging
    dynalog::sdms_add_stdout_stream();
    dynalog::sdms_add_stderr_stream();
    dynalog::sdms_set_level(level::INFO);
    dynalog::sdms_set_syslog(false);

    let ctx = LogCtx {
        thread_name: std::thread::current().name().unwrap_or("main").to_string(),
        correlation_id: "abc-123".to_string(),
        thread_id: std::process::id() as i32,
    };

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_usage();
        return;
    }

    match args[1].as_str() {
        "version" => {
            // same version check as before
            let addr = "tcp://localhost:9999";
            let parts: Vec<&str> = addr.split("://").collect();
            let scheme = parts[0];
            let host = parts[1].split(':').next().unwrap();
            let port: u16 = parts[1].split(':').nth(1).unwrap().parse().unwrap();

            let core_pub = "4X1hKiU5pdwdk.s7&=Q2(b1]p!^Nj=Dnk2&7vh@f";
            let vi = send_version_request(host, port, scheme, core_pub, 20_000)
                .expect("version query failed");

            dl_info!(ctx, "Core API {}.{}.{} | Repo component {}.{}.{} | Release {}-{:02}-{:02} {:02}:{:02}",
                vi.api_major, vi.api_minor, vi.api_patch,
                vi.component_major, vi.component_minor, vi.component_patch,
                vi.release_year, vi.release_month, vi.release_day, vi.release_hour, vi.release_minute);

            println!("Core API {}.{}.{}  | Component {}.{}.{}  | Release {}-{:02}-{:02} {:02}:{:02}",
                vi.api_major, vi.api_minor, vi.api_patch,
                vi.component_major, vi.component_minor, vi.component_patch,
                vi.release_year, vi.release_month, vi.release_day, vi.release_hour, vi.release_minute);
        }

        "run" => {
            let config_path = args.get(2).map(String::as_str).unwrap_or("repo_server.toml");
            let cfg = Config::load(config_path).expect("failed to load config");

            dl_info!(ctx, "Starting Rust repo server on port {}", cfg.port);

            let mut srv = RepoServer::new(cfg.clone());
            srv.start();

            // graceful shutdown on Ctrl-C
            let stop_flag = std::sync::Arc::new(std::sync::Mutex::new(None));
            let stop_flag_c = stop_flag.clone();
            let srv_stop = std::sync::Arc::new(srv);
            let srv_stop_c = srv_stop.clone();

            ctrlc::set_handler(move || {
                eprintln!("\nCtrl-C received: stopping…");
                srv_stop_c.stop();
                *stop_flag_c.lock().unwrap() = Some(());
            }).expect("set Ctrl-C handler");

            // busy-wait until handler sets the flag, then join workers
            loop {
                if stop_flag.lock().unwrap().is_some() { break; }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }

            // take ownership back and join
            let srv = Arc::try_unwrap(srv_stop).ok().expect("refs held");
            srv.join();

            dl_info!(ctx, "Server exited");
        }

        _ => {
            print_usage();
            process::exit(2);
        }
    }
}
