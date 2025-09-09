// at the top:
mod config;
mod ffi;
mod server;
mod version;
mod worker;

use crate::config::Config;
use crate::server::RepoServer;
// keep your dynalog imports/macros if you have them

use std::env;

fn print_usage() {
    eprintln!(
        "Usage:
  repo version
  repo serve --cfg <config.toml>"
    );
}

fn main() {
    unsafe {
        // Add stderr stream via your C++ logger wrapper (exists per your headers)
        ffi::dynalog::sdms_add_stderr_stream();
    }

    let args: Vec<String> = env::args().collect();
    
    // Handle help and version commands
    if args.len() > 1 {
        match args[1].as_str() {
            "--help" | "-h" => {
                print_usage();
                return;
            }
            "version" => {
                println!(
                    "repo {}.{}.{}\napi  {}.{}",
                    version::repo_major(), version::repo_minor(), version::repo_patch(),
                    version::api_major(),  version::api_minor()
                );
                return;
            }
            _ => {}
        }
    }
    
    // Use default config from config.rs, ignore TOML file
    let mut cfg = Config::default();
    cfg.normalize();
    println!("Using default config from config.rs");
    println!("Config: core_server={}, cred_dir={}, port={}", cfg.core_server, cfg.cred_dir, cfg.port);
    let mut srv = RepoServer::new(cfg);
    println!("Server created, starting run method...");
    srv.run(); // blocking
    println!("Server run method returned, calling join...");
    srv.join();
    println!("Server join completed, main function ending...");
    // match args[1].as_str() {
    //     "version" => {
    //         println!(
    //             "repo {}.{}.{}\napi  {}.{}",
    //             version::repo_major(), version::repo_minor(), version::repo_patch(),
    //             version::api_major(),  version::api_minor()
    //         );
    //     }
    //     "serve" => {
    //         // Robust parse for --cfg
    //         let cfg_path: String = args.windows(2)
    //             .find(|w| w[0] == "--cfg")
    //             .map(|w| w[1].clone())
    //             .unwrap_or_else(|| "repo-server.toml".to_string());

    //         let cfg = Config::load(&cfg_path).expect("failed to load config");
    //         let mut srv = RepoServer::new(cfg);
    //         srv.run();   // blocking
    //         srv.join();
    //     }
    //     _ => {
    //         print_usage();
    //         process::exit(2);
    //     }
    // }
}
