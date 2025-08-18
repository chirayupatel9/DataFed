// examples/basic_client.rs
use repo_server_rs::DataFedClient;
use tracing::{info, error, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    info!("DataFed Client Example");
    
    // Create a client
    let mut client = match DataFedClient::new("localhost", 9998) {
        Ok(client) => {
            info!("Successfully created DataFed client");
            client
        }
        Err(e) => {
            error!("Failed to create DataFed client: {}", e);
            return Err(e);
        }
    };
    
    // Check server version
    match client.check_server_version("tcp://localhost:9998") {
        Ok(_) => info!("Core server version check passed"),
        Err(e) => {
            warn!("Core server version check failed: {}", e);
            warn!("This is expected if no server is running");
        }
    }
    
    // Try to get version info
    match client.get_version() {
        Ok(version_info) => {
            info!("Version info: {}", version_info);
        }
        Err(e) => {
            warn!("Failed to get version info: {}", e);
            warn!("This is expected if no server is running");
        }
    }
    
    // Try to create a repository path
    match client.create_path("/test/example/path".to_string()) {
        Ok(response) => {
            info!("Path create response: {}", response);
        }
        Err(e) => {
            warn!("Failed to create path: {}", e);
            warn!("This is expected if no server is running");
        }
    }
    
    // Try to delete some data
    match client.delete_data(
        vec!["example_record_1".to_string(), "example_record_2".to_string()],
        vec!["/path/to/example/file1".to_string(), "/path/to/example/file2".to_string()],
    ) {
        Ok(response) => {
            info!("Delete response: {}", response);
        }
        Err(e) => {
            warn!("Failed to delete data: {}", e);
            warn!("This is expected if no server is running");
        }
    }
    
    info!("Example completed successfully");
    Ok(())
}
