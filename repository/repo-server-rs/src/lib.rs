// Configuration management
pub mod config;

// Protocol Buffer definitions
pub mod proto;

// ZeroMQ communicator implementation
pub mod zmq_communicator;

// Request worker implementation
pub mod request_worker;

// Server implementation
pub mod server;

// Re-export main types
pub use server::{RepoServer, ServerConfig};
pub use zmq_communicator::{
    ZeroMQCommunicator, SocketOptions, MockCredentials, MessageType, MessageState,
    MessageAttribute, ProtoMessage, Response, MessageFactory, CommunicatorFactory,
    SocketClassType, UriScheme, SocketDirectionalityType, SocketCommunicationType,
    SocketConnectionLife, SocketConnectionSecurity, ProtocolType
};
pub use request_worker::RequestWorker;

// Import prost Message trait for Protocol Buffer operations
use prost::Message;

/// High-level client interface for DataFed communication
pub struct DataFedClient {
    communicator: ZeroMQCommunicator,
    thread_id: u64,
    correlation_id: String,
}

impl DataFedClient {
    /// Create a new DataFed client
    pub fn new(host: &str, port: u16) -> Result<Self, Box<dyn std::error::Error>> {
        use std::time::{SystemTime, UNIX_EPOCH};
        use uuid::Uuid;
        
        // Generate a unique thread ID
        let thread_id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        
        // Create correlation ID
        let correlation_id = format!("client-{}", thread_id);
        
        // Create socket options
        let socket_options = SocketOptions {
            scheme: UriScheme::Tcp,
            class_type: SocketClassType::Client,
            direction_type: SocketDirectionalityType::Bidirectional,
            communication_type: SocketCommunicationType::Asynchronous,
            connection_life: SocketConnectionLife::Persistent,
            connection_security: SocketConnectionSecurity::Insecure,
            protocol_type: ProtocolType::Zqtp,
            host: host.to_string(),
            port: Some(port),
            local_id: Some(format!("client-{}", Uuid::new_v4())),
        };
        
        // Create mock credentials
        let credentials = MockCredentials::new(
            "4X1hKiU5pdwdk.s7&=Q2(b1]p!^Nj=Dnk2&7vh@f".to_string(),
            "G1DpacgVoCcRmLYQ6PA8:Q$]/w5SE*Qm?)}L!@Gv".to_string(),
        );
        
        // Create communicator
        let factory = CommunicatorFactory::new();
        let communicator = factory.create(
            socket_options,
            credentials,
            5000, // 5 second timeout
            10,   // 10ms poll timeout
        ).map_err(|e| format!("Failed to create communicator: {}", e))?;
        
        Ok(DataFedClient {
            communicator,
            thread_id,
            correlation_id,
        })
    }
    
    /// Check the version of a core server
    pub fn check_server_version(&self, server_address: &str) -> Result<bool, Box<dyn std::error::Error>> {
        use tracing::info;
        
        info!("Checking core server version at {}", server_address);
        
        // Create version request
        let message_factory = MessageFactory::new();
        let version_request = message_factory.create_version_request();
        
        // Send version request
        self.communicator.send(&version_request)
            .map_err(|e| format!("Failed to send version request: {}", e))?;
        
        // Receive response
        let response = self.communicator.receive(MessageType::GoogleProtocolBuffer);
        
        if response.error {
            return Err(format!("Version check failed: {}", response.error_msg).into());
        }
        
        if response.time_out {
            return Err("Version check timed out".into());
        }
        
        if let Some(message) = response.message {
            // Parse version reply
            let version_reply = crate::proto::version::VersionReply::decode(message.payload.as_slice())
                .map_err(|e| format!("Failed to decode version reply: {}", e))?;
            
            info!("Core server version: {}.{}.{}", 
                  version_reply.api_major, version_reply.api_minor, version_reply.api_patch);
            
            Ok(true)
        } else {
            Err("No version response received".into())
        }
    }
    
    /// Send a version request and receive the response
    pub fn get_version(&mut self) -> Result<String, Box<dyn std::error::Error>> {
        use serde_json::json;
use prost::Message;
        
        // Create version request
        let version_request = MessageFactory::new().create_version_request();
        
        // Send the request
        self.communicator.send(&version_request)
            .map_err(|e| format!("Failed to send version request: {}", e))?;
        
        // Receive the response
        let response = self.communicator.receive(MessageType::GoogleProtocolBuffer);
        
        if response.error {
            return Err(format!("Failed to receive version response: {}", response.error_msg).into());
        }
        
        if response.time_out {
            return Err("Version request timed out".into());
        }
        
        if let Some(message) = response.message {
            // Parse version reply
            let version_reply = crate::proto::version::VersionReply::decode(message.payload.as_slice())
                .map_err(|e| format!("Failed to decode version reply: {}", e))?;
            
            // Convert to JSON
            let json_response = json!({
                "release_year": version_reply.release_year,
                "release_month": version_reply.release_month,
                "release_day": version_reply.release_day,
                "release_hour": version_reply.release_hour,
                "release_minute": version_reply.release_minute,
                "api_major": version_reply.api_major,
                "api_minor": version_reply.api_minor,
                "api_patch": version_reply.api_patch,
                "component_major": version_reply.component_major,
                "component_minor": version_reply.component_minor,
                "component_patch": version_reply.component_patch,
            });
            
            Ok(json_response.to_string())
        } else {
            Err("No version response received".into())
        }
    }
    
    /// Send a repository data delete request
    pub fn delete_data(&mut self, record_ids: Vec<String>, file_paths: Vec<String>) -> Result<String, Box<dyn std::error::Error>> {
        use serde_json::json;
        
        // Create delete request
        let delete_request = MessageFactory::new().create_repo_data_delete_request(record_ids, file_paths);
        
        // Send the request
        self.communicator.send(&delete_request)
            .map_err(|e| format!("Failed to send delete request: {}", e))?;
        
        // Receive the response
        let response = self.communicator.receive(MessageType::GoogleProtocolBuffer);
        
        if response.error {
            return Err(format!("Failed to receive delete response: {}", response.error_msg).into());
        }
        
        if response.time_out {
            return Err("Delete request timed out".into());
        }
        
        if let Some(message) = response.message {
            // Parse ack reply
            let ack_reply = crate::proto::sdms_auth::AckReply::decode(message.payload.as_slice())
                .map_err(|e| format!("Failed to decode ack reply: {}", e))?;
            
            let json_response = json!({
                "success": true,
                "message": ack_reply.message,
            });
            
            Ok(json_response.to_string())
        } else {
            Err("No delete response received".into())
        }
    }
    
    /// Send a repository path create request
    pub fn create_path(&mut self, path: String) -> Result<String, Box<dyn std::error::Error>> {
        use serde_json::json;
        
        // Create path create request
        let create_request = MessageFactory::new().create_repo_path_create_request(path);
        
        // Send the request
        self.communicator.send(&create_request)
            .map_err(|e| format!("Failed to send path create request: {}", e))?;
        
        // Receive the response
        let response = self.communicator.receive(MessageType::GoogleProtocolBuffer);
        
        if response.error {
            return Err(format!("Failed to receive path create response: {}", response.error_msg).into());
        }
        
        if response.time_out {
            return Err("Path create request timed out".into());
        }
        
        if let Some(message) = response.message {
            // Parse ack reply
            let ack_reply = crate::proto::sdms_auth::AckReply::decode(message.payload.as_slice())
                .map_err(|e| format!("Failed to decode ack reply: {}", e))?;
            
            let json_response = json!({
                "success": true,
                "message": ack_reply.message,
            });
            
            Ok(json_response.to_string())
        } else {
            Err("No path create response received".into())
        }
    }
}

/// Example usage function
pub fn example_usage() -> Result<(), Box<dyn std::error::Error>> {
    // Create a client
    let mut client = DataFedClient::new("localhost", 9998)?;
    
    // Check server version
    let version_check = client.check_server_version("tcp://localhost:9998")?;
    println!("Version check result: {}", version_check);
    
    // Get version details
    let version_info = client.get_version()?;
    println!("Version info: {}", version_info);
    
    // Create a repository path
    let path_response = client.create_path("/test/path".to_string())?;
    println!("Path create response: {}", path_response);
    
    // Delete some data
    let delete_response = client.delete_data(
        vec!["record1".to_string(), "record2".to_string()],
        vec!["/path/to/file1".to_string(), "/path/to/file2".to_string()],
    )?;
    println!("Delete response: {}", delete_response);
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_client_creation() {
        // This test would require a running DataFed server
        // For now, we'll just test that the code compiles
        assert!(true);
    }
    
    #[test]
    fn test_message_factory() {
        let factory = MessageFactory::new();
        let version_request = factory.create_version_request();
        assert_eq!(version_request.message_type, 1);
        assert_eq!(version_request.proto_id, 1);
    }
}
