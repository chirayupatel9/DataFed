use clap::Parser;
use tracing::{info, error, warn, Level};
use tracing_subscriber;

use repo_server_rs::server::{RepoServer, ServerConfig};
use repo_server_rs::config::Config as AppConfig;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Server port (overrides config file)
    #[arg(short, long)]
    port: Option<u16>,
    
    /// Number of worker threads (overrides config file)
    #[arg(short, long)]
    threads: Option<u32>,
    
    /// Core server address (overrides config file)
    #[arg(long)]
    core_server: Option<String>,
    
    /// Globus collection path (overrides config file)
    #[arg(short, long)]
    globus_path: Option<String>,
    
    /// Credentials directory (overrides config file)
    #[arg(long)]
    cred_dir: Option<String>,
    
    /// Log level (overrides config file)
    #[arg(short, long)]
    log_level: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    // Load configuration from file or use defaults
    let app_config = AppConfig::load();
    
    // Validate configuration
    if let Err(errors) = app_config.validate() {
        for error in errors {
            error!("Configuration error: {}", error);
        }
        return Err("Configuration validation failed".into());
    }
    
    // Initialize logging based on configuration or command line
    let log_level = match args.log_level.as_deref().unwrap_or(&app_config.logging.level) {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };
    
    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .init();
    
    info!("DataFed Repository Server starting");
    info!("Version: 1.0.0");
    
    // Use configuration file values, with command line args as overrides
    let port = args.port.or(Some(app_config.server.port)).unwrap_or(10000);
    let threads = args.threads.or(Some(app_config.server.num_worker_threads)).unwrap_or(4);
    let core_server = args.core_server.as_deref().unwrap_or(&app_config.server.core_server);
    let globus_path = args.globus_path.as_deref().unwrap_or(&app_config.server.globus_collection_path);
    let cred_dir = args.cred_dir.as_deref().unwrap_or(&app_config.server.cred_dir);
    
    info!("Port: {}", port);
    info!("Worker threads: {}", threads);
    info!("Core server: {}", core_server);
    info!("Globus path: {}", globus_path);
    
    // Create server configuration
    let config = ServerConfig {
        port,
        num_worker_threads: threads,
        core_server: core_server.to_string(),
        globus_collection_path: globus_path.to_string(),
        cred_dir: cred_dir.to_string(),
    };
    
    // Create and start server
    let mut server = RepoServer::new(config);
    
    // Check core server version
    match server.check_server_version(core_server).await {
        Ok(true) => info!("Core server version check passed"),
        Ok(false) => {
            warn!("Core server version check failed - no compatible server found");
            warn!("Continuing anyway...");
        }
        Err(e) => {
            warn!("Core server version check error: {}", e);
            warn!("Continuing anyway...");
        }
    }
    
    // Start the server
    if let Err(e) = server.start().await {
        error!("Failed to start server: {}", e);
        return Err(e.into());
    }
    
    // Run the server
    if let Err(e) = server.run().await {
        error!("Server failed: {}", e);
        return Err(e.into());
    }
    
    info!("DataFed Repository Server stopped");
    Ok(())
}