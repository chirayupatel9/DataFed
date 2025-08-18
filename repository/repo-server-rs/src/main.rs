use clap::Parser;
use tracing::{info, error, warn, Level};
use tracing_subscriber;

use repo_server_rs::server::{RepoServer, ServerConfig};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Server port
    #[arg(short, long, default_value = "9998")]
    port: u16,
    
    /// Number of worker threads
    #[arg(short, long, default_value = "4")]
    threads: u32,
    
    /// Core server address
    #[arg(long, default_value = "tcp://localhost:9998")]
    core_server: String,
    
    /// Globus collection path
    #[arg(short, long, default_value = "/mnt/datafed-repo")]
    globus_path: String,
    
    /// Credentials directory
    #[arg(long, default_value = "./")]
    cred_dir: String,
    
    /// Log level
    #[arg(short, long, default_value = "info")]
    log_level: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    // Initialize logging
    let log_level = match args.log_level.as_str() {
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
    info!("Port: {}", args.port);
    info!("Worker threads: {}", args.threads);
    info!("Core server: {}", args.core_server);
    info!("Globus path: {}", args.globus_path);
    
    // Create server configuration
    let config = ServerConfig {
        port: args.port,
        num_worker_threads: args.threads,
        core_server: args.core_server.clone(),
        globus_collection_path: args.globus_path,
        cred_dir: args.cred_dir,
    };
    
    // Create and start server
    let mut server = RepoServer::new(config);
    
    // Check core server version
    match server.check_server_version(&args.core_server).await {
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