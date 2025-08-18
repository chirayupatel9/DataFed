use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::Mutex;
use tracing::{info, error, debug};

use crate::zmq_communicator::{
    ZeroMQCommunicator, SocketOptions, MockCredentials, MessageType, MessageState, 
    ProtoMessage, MessageFactory, CommunicatorFactory,
    SocketClassType, UriScheme, SocketDirectionalityType, SocketCommunicationType,
    SocketConnectionLife, SocketConnectionSecurity, ProtocolType
};
use crate::proto::{sdms, sdms_auth, version};
use prost::Message as ProstMessage;

/// Request worker that processes messages using ZeroMQ and Protocol Buffers
pub struct RequestWorker {
    pub worker_id: u32,
    communicator: Option<ZeroMQCommunicator>,
    message_factory: MessageFactory,
    message_handlers: HashMap<u16, Box<dyn Fn(ProtoMessage) -> Result<ProtoMessage, String> + Send + Sync>>,
    running: Arc<Mutex<bool>>,
}

impl RequestWorker {
    pub fn new(worker_id: u32) -> Self {
        let mut worker = Self {
            worker_id,
            communicator: None,
            message_factory: MessageFactory::new(),
            message_handlers: HashMap::new(),
            running: Arc::new(Mutex::new(true)),
        };
        
        worker.setup_message_handlers();
        worker
    }
    
    fn setup_message_handlers(&mut self) {
        // Anonymous protocol handlers (protocol ID = 1)
        self.message_handlers.insert(1, Box::new(|msg| {
            Self::handle_version_request(msg)
        }));
        
        // Authenticated protocol handlers (protocol ID = 2)
        self.message_handlers.insert(3, Box::new(|msg| {
            Self::handle_repo_data_delete_request(msg)
        }));
        
        self.message_handlers.insert(4, Box::new(|msg| {
            Self::handle_repo_data_get_size_request(msg)
        }));
        
        self.message_handlers.insert(5, Box::new(|msg| {
            Self::handle_repo_path_create_request(msg)
        }));
        
        self.message_handlers.insert(6, Box::new(|msg| {
            Self::handle_repo_path_delete_request(msg)
        }));
    }
    
    pub async fn start(&mut self) -> Result<(), String> {
        info!("Starting request worker {}", self.worker_id);
        
        // Create socket options for worker communication
        let socket_options = SocketOptions {
            scheme: UriScheme::Inproc,
            class_type: SocketClassType::Client,
            direction_type: SocketDirectionalityType::Bidirectional,
            communication_type: SocketCommunicationType::Asynchronous,
            connection_life: SocketConnectionLife::Intermittent,
            connection_security: SocketConnectionSecurity::Insecure,
            protocol_type: ProtocolType::Zqtp,
            host: "workers".to_string(),
            port: None,
            local_id: Some(format!("repo-worker-{}", self.worker_id)),
        };
        
        // Create mock credentials (matching the C++ implementation)
        let credentials = MockCredentials::new(
            "4X1hKiU5pdwdk.s7&=Q2(b1]p!^Nj=Dnk2&7vh@f".to_string(),
            "G1DpacgVoCcRmLYQ6PA8:Q$]/w5SE*Qm?)}L!@Gv".to_string(),
        );
        
        // Create communicator
        let factory = CommunicatorFactory::new();
        let communicator = factory.create(
            socket_options,
            credentials,
            10000, // 10 second timeout
            10,    // 10ms poll timeout
        ).map_err(|e| format!("Failed to create communicator: {}", e))?;
        
        self.communicator = Some(communicator);
        
        info!("Request worker {} started, listening on {}", 
              self.worker_id, self.communicator.as_ref().unwrap().address());
        
        Ok(())
    }
    
    pub async fn run(&mut self) -> Result<(), String> {
        let communicator = self.communicator.as_ref()
            .ok_or("Communicator not initialized")?;
        
        info!("Request worker {} running", self.worker_id);
        
        while *self.running.lock() {
            // Receive message
            let response = communicator.receive(MessageType::GoogleProtocolBuffer);
            
            if response.error {
                error!("Worker {} received error: {}", self.worker_id, response.error_msg);
                continue;
            }
            
            if response.time_out {
                debug!("Worker {} received timeout", self.worker_id);
                continue;
            }
            
            if let Some(message) = response.message {
                let correlation_id = message.correlation_id.clone();
                let message_type = message.message_type;
                let proto_id = message.proto_id;
                
                debug!("Worker {} received message: correlation_id={}, type={}, proto_id={}", 
                       self.worker_id, correlation_id, message_type, proto_id);
                
                // Handle the message
                if let Some(handler) = self.message_handlers.get(&message_type) {
                    match handler(message) {
                        Ok(reply) => {
                            // Send the reply
                            if let Err(e) = communicator.send(&reply) {
                                error!("Worker {} failed to send reply: {}", self.worker_id, e);
                            } else {
                                debug!("Worker {} sent reply: correlation_id={}", 
                                       self.worker_id, reply.correlation_id);
                            }
                        }
                        Err(e) => {
                            error!("Worker {} failed to handle message: {}", self.worker_id, e);
                            // Send error reply
                            let error_reply = self.message_factory.create_nack_reply(
                                correlation_id,
                                sdms::ErrorCode::IdInternalError,
                                e,
                            );
                            if let Err(e) = communicator.send(&error_reply) {
                                error!("Worker {} failed to send error reply: {}", self.worker_id, e);
                            }
                        }
                    }
                } else {
                    error!("Worker {} received unregistered message type: {}", 
                           self.worker_id, message_type);
                    // Send error reply for unknown message type
                    let error_reply = self.message_factory.create_nack_reply(
                        correlation_id,
                        sdms::ErrorCode::IdBadRequest,
                        format!("Unknown message type: {}", message_type),
                    );
                    if let Err(e) = communicator.send(&error_reply) {
                        error!("Worker {} failed to send error reply: {}", self.worker_id, e);
                    }
                }
            }
        }
        
        info!("Request worker {} stopped", self.worker_id);
        Ok(())
    }
    
    pub async fn stop(&self) {
        info!("Stopping request worker {}", self.worker_id);
        *self.running.lock() = false;
    }
    
    // Message handlers matching the C++ implementation
    
    fn handle_version_request(message: ProtoMessage) -> Result<ProtoMessage, String> {
        debug!("Processing version request: correlation_id={}", message.correlation_id);
        
        // Parse the version request
        let _request = version::VersionRequest::decode(message.payload.as_slice())
            .map_err(|e| format!("Failed to decode version request: {}", e))?;
        
        // Create version reply (matching the C++ implementation)
        let reply = version::VersionReply {
            release_year: 2024,
            release_month: 12,
            release_day: 1,
            release_hour: 0,
            release_minute: 0,
            api_major: 1,
            api_minor: 0,
            api_patch: 0,
            component_major: 1,
            component_minor: 0,
            component_patch: 0,
        };
        
        let payload = reply.encode_to_vec();
        let response = ProtoMessage::new(
            message.correlation_id,
            MessageState::Response,
            payload,
            2, // Version reply type
            1, // Anonymous protocol ID
        );
        
        Ok(response)
    }
    
    fn handle_repo_data_delete_request(message: ProtoMessage) -> Result<ProtoMessage, String> {
        debug!("Processing repo data delete request: correlation_id={}", message.correlation_id);
        
        // Parse the request
        let request = sdms_auth::RepoDataDeleteRequest::decode(message.payload.as_slice())
            .map_err(|e| format!("Failed to decode repo data delete request: {}", e))?;
        
        // Process the request (in a real implementation, this would delete files)
        info!("Deleting {} records from repository", request.record_ids.len());
        
        // Create success reply
        let reply = sdms_auth::AckReply {
            message: Some(format!("Successfully deleted {} records", request.record_ids.len())),
        };
        
        let payload = reply.encode_to_vec();
        let response = ProtoMessage::new(
            message.correlation_id,
            MessageState::Response,
            payload,
            7, // Ack reply type
            2, // Auth protocol ID
        );
        
        Ok(response)
    }
    
    fn handle_repo_data_get_size_request(message: ProtoMessage) -> Result<ProtoMessage, String> {
        debug!("Processing repo data get size request: correlation_id={}", message.correlation_id);
        
        // Parse the request
        let request = sdms_auth::RepoDataGetSizeRequest::decode(message.payload.as_slice())
            .map_err(|e| format!("Failed to decode repo data get size request: {}", e))?;
        
        // Process the request (in a real implementation, this would get file sizes)
        let mut sizes = Vec::new();
        for _ in &request.record_ids {
            sizes.push(1024); // Mock size of 1KB per file
        }
        
        // Create size reply
        let reply = sdms_auth::RepoDataSizeReply {
            sizes,
        };
        
        let payload = reply.encode_to_vec();
        let response = ProtoMessage::new(
            message.correlation_id,
            MessageState::Response,
            payload,
            9, // Repo data size reply type
            2, // Auth protocol ID
        );
        
        Ok(response)
    }
    
    fn handle_repo_path_create_request(message: ProtoMessage) -> Result<ProtoMessage, String> {
        debug!("Processing repo path create request: correlation_id={}", message.correlation_id);
        
        // Parse the request
        let request = sdms_auth::RepoPathCreateRequest::decode(message.payload.as_slice())
            .map_err(|e| format!("Failed to decode repo path create request: {}", e))?;
        
        // Process the request (in a real implementation, this would create directories)
        info!("Creating repository path: {}", request.path);
        
        // Create success reply
        let reply = sdms_auth::AckReply {
            message: Some(format!("Successfully created path: {}", request.path)),
        };
        
        let payload = reply.encode_to_vec();
        let response = ProtoMessage::new(
            message.correlation_id,
            MessageState::Response,
            payload,
            7, // Ack reply type
            2, // Auth protocol ID
        );
        
        Ok(response)
    }
    
    fn handle_repo_path_delete_request(message: ProtoMessage) -> Result<ProtoMessage, String> {
        debug!("Processing repo path delete request: correlation_id={}", message.correlation_id);
        
        // Parse the request
        let request = sdms_auth::RepoPathDeleteRequest::decode(message.payload.as_slice())
            .map_err(|e| format!("Failed to decode repo path delete request: {}", e))?;
        
        // Process the request (in a real implementation, this would delete directories)
        info!("Deleting repository path: {}", request.path);
        
        // Create success reply
        let reply = sdms_auth::AckReply {
            message: Some(format!("Successfully deleted path: {}", request.path)),
        };
        
        let payload = reply.encode_to_vec();
        let response = ProtoMessage::new(
            message.correlation_id,
            MessageState::Response,
            payload,
            7, // Ack reply type
            2, // Auth protocol ID
        );
        
        Ok(response)
    }
}