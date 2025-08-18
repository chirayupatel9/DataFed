use std::sync::Arc;
use parking_lot::Mutex;
use tracing::{info, error, debug, warn};
use uuid::Uuid;

use crate::zmq_communicator::{
    ZeroMQCommunicator, SocketOptions, MockCredentials, MessageType, 
    MessageFactory, CommunicatorFactory,
    SocketClassType, UriScheme, SocketDirectionalityType, SocketCommunicationType,
    SocketConnectionLife, SocketConnectionSecurity, ProtocolType
};
use crate::request_worker::RequestWorker;
use crate::proto::version;
use prost::Message;

/// Repository server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub port: u16,
    pub num_worker_threads: u32,
    pub core_server: String,
    pub globus_collection_path: String,
    pub cred_dir: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 10000, // Use port 10001 to avoid conflicts with mock core server (9998, 10000)
            num_worker_threads: 4,
            core_server: "tcp://localhost:9998".to_string(),
            globus_collection_path: "/mnt/datafed-repo".to_string(),
            cred_dir: "/mnt/storage/rust/Datafed/".to_string(),
        }
    }
}

/// Repository server implementation using ZeroMQ proxy pattern
pub struct RepoServer {
    config: ServerConfig,
    workers: Vec<RequestWorker>,
    proxy_server: Option<ZeroMQCommunicator>,
    proxy_client: Option<ZeroMQCommunicator>,
    running: Arc<Mutex<bool>>,
}

impl RepoServer {
    pub fn new(config: ServerConfig) -> Self {
        Self {
            config,
            workers: Vec::new(),
            proxy_server: None,
            proxy_client: None,
            running: Arc::new(Mutex::new(true)),
        }
    }
    
    pub async fn start(&mut self) -> Result<(), String> {
        info!("Starting DataFed repository server on port {}", self.config.port);
        
        // Load mock credentials (matching the C++ implementation)
        let credentials = MockCredentials::new(
            "4X1hKiU5pdwdk.s7&=Q2(b1]p!^Nj=Dnk2&7vh@f".to_string(),
            "G1DpacgVoCcRmLYQ6PA8:Q$]/w5SE*Qm?)}L!@Gv".to_string(),
        );
        
        // Create worker threads
        info!("Creating {} worker threads", self.config.num_worker_threads);
        for worker_id in 1..=self.config.num_worker_threads {
            let mut worker = RequestWorker::new(worker_id);
            worker.start().await.map_err(|e| format!("Failed to start worker {}: {}", worker_id, e))?;
            self.workers.push(worker);
        }
        
        // Create ZeroMQ proxy (matching the C++ implementation)
        self.setup_proxy(credentials).await?;
        
        info!("Repository server started successfully");
        Ok(())
    }
    
    async fn setup_proxy(&mut self, credentials: MockCredentials) -> Result<(), String> {
        info!("Setting up ZeroMQ proxy");
        
        // Create proxy server socket (external facing)
        let server_socket_options = SocketOptions {
            scheme: UriScheme::Tcp,
            class_type: SocketClassType::Server,
            direction_type: SocketDirectionalityType::Bidirectional,
            communication_type: SocketCommunicationType::Asynchronous,
            connection_life: SocketConnectionLife::Persistent,
            connection_security: SocketConnectionSecurity::Secure,
            protocol_type: ProtocolType::Zqtp,
            host: "*".to_string(),
            port: Some(self.config.port),
            local_id: Some("main_repository_server_external_facing_socket".to_string()),
        };
        
        let factory = CommunicatorFactory::new();
        let proxy_server = factory.create(
            server_socket_options,
            credentials.clone(),
            10000, // 10 second timeout
            10,    // 10ms poll timeout
        ).map_err(|e| format!("Failed to create proxy server: {}", e))?;
        
        // Create proxy client socket (internal facing)
        let client_socket_options = SocketOptions {
            scheme: UriScheme::Inproc,
            class_type: SocketClassType::Client,
            direction_type: SocketDirectionalityType::Bidirectional,
            communication_type: SocketCommunicationType::Asynchronous,
            connection_life: SocketConnectionLife::Persistent,
            connection_security: SocketConnectionSecurity::Insecure,
            protocol_type: ProtocolType::Zqtp,
            host: "workers".to_string(),
            port: None,
            local_id: Some("main_repository_server_internal_facing_socket".to_string()),
        };
        
        let proxy_client = factory.create(
            client_socket_options,
            credentials,
            10000, // 10 second timeout
            10,    // 10ms poll timeout
        ).map_err(|e| format!("Failed to create proxy client: {}", e))?;
        
        self.proxy_server = Some(proxy_server);
        self.proxy_client = Some(proxy_client);
        
        info!("ZeroMQ proxy setup complete");
        Ok(())
    }
    
    pub async fn run(&mut self) -> Result<(), String> {
        info!("Repository server running");
        
        // Start worker tasks
        let mut worker_handles = Vec::new();
        for worker in &mut self.workers {
            let worker_id = worker.worker_id;
            let _running = self.running.clone();
            
            let handle = tokio::spawn(async move {
                // Create a new worker for this task
                let mut task_worker = RequestWorker::new(worker_id);
                if let Err(e) = task_worker.start().await {
                    error!("Failed to start worker {}: {}", worker_id, e);
                    return;
                }
                
                if let Err(e) = task_worker.run().await {
                    error!("Worker {} failed: {}", worker_id, e);
                }
            });
            worker_handles.push(handle);
        }
        
        // Run proxy loop
        let proxy_server = self.proxy_server.as_ref()
            .ok_or("Proxy server not initialized")?;
        let proxy_client = self.proxy_client.as_ref()
            .ok_or("Proxy client not initialized")?;
        
        while *self.running.lock() {
            // Receive from external clients
            let response = proxy_server.receive(MessageType::GoogleProtocolBuffer);
            
            if response.error {
                error!("Proxy server received error: {}", response.error_msg);
                continue;
            }
            
            if response.time_out {
                debug!("Proxy server received timeout");
                continue;
            }
            
            if let Some(message) = response.message {
                debug!("Proxy server received message: correlation_id={}, type={}, proto_id={}", 
                       message.correlation_id, message.message_type, message.proto_id);
                
                // Forward to internal workers
                if let Err(e) = proxy_client.send(&message) {
                    error!("Failed to forward message to workers: {}", e);
                } else {
                    debug!("Forwarded message to workers: correlation_id={}", message.correlation_id);
                }
                
                // Receive response from workers
                let worker_response = proxy_client.receive(MessageType::GoogleProtocolBuffer);
                
                if worker_response.error {
                    error!("Proxy client received error: {}", worker_response.error_msg);
                    continue;
                }
                
                if worker_response.time_out {
                    debug!("Proxy client received timeout");
                    continue;
                }
                
                if let Some(reply) = worker_response.message {
                    debug!("Proxy client received reply: correlation_id={}, type={}, proto_id={}", 
                           reply.correlation_id, reply.message_type, reply.proto_id);
                    
                    // Forward response back to external client
                    if let Err(e) = proxy_server.send(&reply) {
                        error!("Failed to forward reply to client: {}", e);
                    } else {
                        debug!("Forwarded reply to client: correlation_id={}", reply.correlation_id);
                    }
                }
            }
        }
        
        // Wait for workers to finish
        for handle in worker_handles {
            if let Err(e) = handle.await {
                error!("Worker task failed: {}", e);
            }
        }
        
        info!("Repository server stopped");
        Ok(())
    }
    
    pub async fn stop(&self) {
        info!("Stopping repository server");
        *self.running.lock() = false;
        
        // Stop all workers
        for worker in &self.workers {
            worker.stop().await;
        }
    }
    
    pub async fn check_server_version(&self, server_address: &str) -> Result<bool, String> {
        info!("Checking core server version at {}", server_address);
        
        // Parse server address to extract host and port
        let (host, port) = if server_address.starts_with("tcp://") {
            let parts: Vec<&str> = server_address[6..].split(':').collect();
            if parts.len() == 2 {
                (parts[0].to_string(), parts[1].parse::<u16>().unwrap_or(9998))
            } else {
                (parts[0].to_string(), 9998)
            }
        } else {
            (server_address.to_string(), 9998)
        };
        
        // The mock core server has both secure (port) and insecure (port + 1) interfaces
        // We'll use the secure interface (port 9998) since that's the main interface
        let secure_port = port;
        
        // Create client socket options
        let socket_options = SocketOptions {
            scheme: UriScheme::Tcp,
            class_type: SocketClassType::Client,
            direction_type: SocketDirectionalityType::Bidirectional,
            communication_type: SocketCommunicationType::Asynchronous,
            connection_life: SocketConnectionLife::Persistent,
            connection_security: SocketConnectionSecurity::Insecure,
            protocol_type: ProtocolType::Zqtp,
            host,
            port: Some(secure_port), // Use secure port (9998) but insecure connection
            local_id: Some(format!("version-check-client-{}", Uuid::new_v4())),
        };
        
        // Create mock credentials for connecting to the core server
        // We need the server's public key and our own key pair
        let credentials = MockCredentials::new(
            "4X1hKiU5pdwdk.s7&=Q2(b1]p!^Nj=Dnk2&7vh@f".to_string(), // Our public key
            "G1DpacgVoCcRmLYQ6PA8:Q$]/w5SE*Qm?)}L!@Gv".to_string(), // Our private key
        );
        
        // Create communicator
        let factory = CommunicatorFactory::new();
        debug!("Creating version check communicator with options: {:?}", socket_options);
        let communicator = match factory.create(
            socket_options,
            credentials,
            5000, // 5 second timeout
            10,   // 10ms poll timeout
        ) {
            Ok(comm) => comm,
            Err(e) => {
                warn!("Failed to create version check communicator: {}", e);
                return Ok(false); // Return false instead of error to allow server to start
            }
        };
        
        // Create version request
        let message_factory = MessageFactory::new();
        let version_request = message_factory.create_version_request();
        
        // Send version request
        if let Err(e) = communicator.send(&version_request) {
            warn!("Failed to send version request: {}", e);
            return Ok(false); // Return false instead of error to allow server to start
        }
        
        // Receive response
        let response = communicator.receive(MessageType::GoogleProtocolBuffer);
        
        if response.error {
            warn!("Version check failed: {}", response.error_msg);
            return Ok(false);
        }
        
        if response.time_out {
            warn!("Version check timed out");
            return Ok(false);
        }
        
        if let Some(message) = response.message {
            // Parse version reply
            let version_reply = match version::VersionReply::decode(message.payload.as_slice()) {
                Ok(reply) => reply,
                Err(e) => {
                    warn!("Failed to decode version reply: {}", e);
                    return Ok(false);
                }
            };
            
            info!("Core server version: {}.{}.{}", 
                  version_reply.api_major, version_reply.api_minor, version_reply.api_patch);
            
            // Check compatibility (matching the C++ implementation)
            if version_reply.api_major != 1 {
                warn!("Incompatible messaging API detected: major version {} not supported", 
                      version_reply.api_major);
                return Ok(false);
            }
            
            info!("Core server connection OK");
            Ok(true)
        } else {
            warn!("No version response received");
            Ok(false)
        }
    }
}