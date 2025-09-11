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
    let mut config_file = None;
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
            "serve" => {
                // Handle serve command - look for --cfg argument
                i += 1;
                while i < args.len() {
                    match args[i].as_str() {
                        "--cfg" => {
                            if i + 1 < args.len() {
                                config_file = Some(args[i + 1].clone());
                                i += 1; // Skip the config file path
                            } else {
                                eprintln!("Error: --cfg requires a config file path");
                                return;
                            }
                        }
                        _ => {
                            eprintln!("Unknown argument for serve: {}", args[i]);
                            print_usage();
                            return;
                        }
                    }
                    i += 1;
                }
                break; // Exit the main loop since we handled serve
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
    
    // Load configuration from TOML file and environment variables
    let cfg_path = config_file.unwrap_or_else(|| "repo-server.toml".to_string());
    let cfg = match Config::load(&cfg_path) {
        Ok(config) => {
            if std::path::Path::new(&cfg_path).exists() {
                println!("Successfully loaded configuration from {}", cfg_path);
            } else {
                println!("No TOML file found at {}, using defaults", cfg_path);
            }
            config
        }
        Err(e) => {
            eprintln!("Failed to load configuration from {}: {}", cfg_path, e);
            eprintln!("Falling back to default configuration");
            let mut default_cfg = Config::default();
            // default_cfg.load_from_env(); // Commented out - only use defaults
            default_cfg.normalize();
            default_cfg
        }
    };
    
    println!("Config: core_server={}, cred_dir={}, port={}", cfg.core_server, cfg.cred_dir, cfg.port);
    let mut srv = RepoServer::new(cfg);
    println!("Server created, starting run method...");
    srv.run(); // blocking
    println!("Server run method returned, calling join...");
    srv.join();
    println!("Server join completed, main function ending...");
}
