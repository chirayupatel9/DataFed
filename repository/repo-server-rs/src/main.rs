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
  repo serve --cfg <config.toml>
  repo --gen-keys [--cred-dir <dir>]"
    );
}

fn main() {
    unsafe {
        // Add stderr stream via your C++ logger wrapper (exists per your headers)
        ffi::dynalog::sdms_add_stderr_stream();
    }

    let args: Vec<String> = env::args().collect();
    
    // Parse command line arguments
    let mut gen_keys = false;
    let mut cred_dir = None;
    let mut i = 1;
    
    while i < args.len() {
        match args[i].as_str() {
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
            "--gen-keys" => {
                gen_keys = true;
            }
            "--cred-dir" => {
                if i + 1 < args.len() {
                    cred_dir = Some(args[i + 1].clone());
                    i += 1; // Skip the next argument since we consumed it
                } else {
                    eprintln!("Error: --cred-dir requires a directory path");
                    return;
                }
            }
            _ => {
                eprintln!("Unknown argument: {}", args[i]);
                print_usage();
                return;
            }
        }
        i += 1;
    }
    
    // Handle key generation
    if gen_keys {
        let mut cfg = Config::default();
        if let Some(dir) = cred_dir {
            cfg.cred_dir = if dir.ends_with('/') { dir } else { format!("{}/", dir) };
        }
        cfg.normalize();
        
        match cfg.generate_and_save_keys() {
            Ok(()) => {
                println!("Key generation completed successfully");
                return;
            }
            Err(e) => {
                eprintln!("Key generation failed: {}", e);
                return;
            }
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
