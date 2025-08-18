use std::sync::Arc;
use zmq::{Context, Socket, SocketType, Error as ZmqError};
use uuid::Uuid;
use tracing::{info, debug, error};


use crate::proto::{sdms_auth, version, sdms};
use prost::Message as ProstMessage;

/// Socket communication types matching the C++ implementation
#[derive(Debug, Clone, Copy)]
pub enum SocketCommunicationType {
    Asynchronous,
    Synchronous,
}

/// Socket connection life types
#[derive(Debug, Clone, Copy)]
pub enum SocketConnectionLife {
    Intermittent,
    Persistent,
}

/// Socket class types
#[derive(Debug, Clone, Copy)]
pub enum SocketClassType {
    Client,
    Server,
}

/// URI schemes
#[derive(Debug, Clone, Copy)]
pub enum UriScheme {
    Tcp,
    Inproc,
}

/// Socket directionality types
#[derive(Debug, Clone, Copy)]
pub enum SocketDirectionalityType {
    Unidirectional,
    Bidirectional,
}

/// Socket connection security
#[derive(Debug, Clone, Copy)]
pub enum SocketConnectionSecurity {
    Secure,
    Insecure,
}

/// Protocol types
#[derive(Debug, Clone, Copy)]
pub enum ProtocolType {
    Http,
    Zqtp, // ZeroMQ Transport Protocol
}

/// Socket options matching the C++ implementation
#[derive(Debug, Clone)]
pub struct SocketOptions {
    pub scheme: UriScheme,
    pub class_type: SocketClassType,
    pub direction_type: SocketDirectionalityType,
    pub communication_type: SocketCommunicationType,
    pub connection_life: SocketConnectionLife,
    pub connection_security: SocketConnectionSecurity,
    pub protocol_type: ProtocolType,
    pub host: String,
    pub port: Option<u16>,
    pub local_id: Option<String>,
}

impl Default for SocketOptions {
    fn default() -> Self {
        Self {
            scheme: UriScheme::Inproc,
            class_type: SocketClassType::Server,
            direction_type: SocketDirectionalityType::Bidirectional,
            communication_type: SocketCommunicationType::Asynchronous,
            connection_life: SocketConnectionLife::Persistent,
            connection_security: SocketConnectionSecurity::Insecure,
            protocol_type: ProtocolType::Zqtp,
            host: String::new(),
            port: None,
            local_id: None,
        }
    }
}

/// Credential types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CredentialType {
    PublicKey,
    PrivateKey,
    ServerKey,
}

/// Mock credentials implementation matching the C++ version
#[derive(Clone)]
pub struct MockCredentials {
    public_key: String,
    private_key: String,
}

impl MockCredentials {
    pub fn new(public_key: String, private_key: String) -> Self {
        Self {
            public_key,
            private_key,
        }
    }

    pub fn get(&self, cred_type: CredentialType) -> Option<String> {
        match cred_type {
            CredentialType::PublicKey => Some(self.public_key.clone()),
            CredentialType::PrivateKey => Some(self.private_key.clone()),
            CredentialType::ServerKey => Some(self.public_key.clone()), // For testing, use public key as server key
        }
    }

    pub fn has(&self, cred_type: CredentialType) -> bool {
        matches!(
            cred_type,
            CredentialType::PublicKey | CredentialType::PrivateKey | CredentialType::ServerKey
        )
    }
}

/// Message types
#[derive(Debug, Clone, Copy)]
pub enum MessageType {
    GoogleProtocolBuffer,
    String,
}

/// Message states
#[derive(Debug, Clone, Copy)]
pub enum MessageState {
    Request,
    Response,
}

/// Message attributes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MessageAttribute {
    Id,
    Key,
    State,
    CorrelationId,
}

/// Protocol Buffer message wrapper
pub struct ProtoMessage {
    pub correlation_id: String,
    pub state: MessageState,
    pub payload: Vec<u8>,
    pub message_type: u16,
    pub proto_id: u8,
}

impl ProtoMessage {
    pub fn new(correlation_id: String, state: MessageState, payload: Vec<u8>, message_type: u16, proto_id: u8) -> Self {
        Self {
            correlation_id,
            state,
            payload,
            message_type,
            proto_id,
        }
    }

    pub fn get(&self, attr: MessageAttribute) -> Option<String> {
        match attr {
            MessageAttribute::CorrelationId => Some(self.correlation_id.clone()),
            MessageAttribute::State => Some(format!("{:?}", self.state)),
            _ => None,
        }
    }

    pub fn exists(&self, attr: MessageAttribute) -> bool {
        self.get(attr).is_some()
    }
}

/// Response structure matching the C++ implementation
pub struct Response {
    pub events: i32,
    pub time_out: bool,
    pub error: bool,
    pub error_msg: String,
    pub message: Option<ProtoMessage>,
}

impl Default for Response {
    fn default() -> Self {
        Self {
            events: 0,
            time_out: false,
            error: false,
            error_msg: String::new(),
            message: None,
        }
    }
}

/// ZeroMQ communicator implementation
pub struct ZeroMQCommunicator {
    socket: Socket,
    context: Arc<Context>,
    options: SocketOptions,
    credentials: MockCredentials,
    timeout_receive_ms: u32,
    timeout_poll_ms: i64,
    local_id: String,
}

impl ZeroMQCommunicator {
    pub fn new(
        options: SocketOptions,
        credentials: MockCredentials,
        timeout_receive_ms: u32,
        timeout_poll_ms: i64,
    ) -> Result<Self, ZmqError> {
        let context = Arc::new(Context::new());
        let socket_type = Self::get_socket_type(&options)?;
        let socket = context.socket(socket_type)?;
        
        // Set socket options
        if let Some(local_id) = &options.local_id {
            socket.set_identity(local_id.as_bytes())?;
        }
        
        // Set timeouts
        socket.set_rcvtimeo(timeout_receive_ms as i32)?;
        socket.set_sndtimeo(timeout_receive_ms as i32)?;
        
        // Set up Curve security if needed
        if let SocketConnectionSecurity::Secure = options.connection_security {
            // For client connections, we need the server's public key
            if let SocketClassType::Client = options.class_type {
                debug!("Setting up Curve security for client connection");
                
                let server_public_key = credentials.get(CredentialType::ServerKey)
                    .ok_or_else(|| {
                        error!("Failed to get server key from credentials");
                        ZmqError::EINVAL
                    })?;
                debug!("Server public key: {}", server_public_key);
                
                // The keys are in ZeroMQ's Z85 format, not base64
                // We can pass them directly to the socket methods
                debug!("Setting server public key (Z85 format): {}", server_public_key);
                
                socket.set_curve_serverkey(server_public_key.as_bytes())
                    .map_err(|e| {
                        error!("Failed to set curve server key: {}", e);
                        e
                    })?;
                
                // Set our own key pair
                let public_key = credentials.get(CredentialType::PublicKey)
                    .ok_or_else(|| {
                        error!("Failed to get public key from credentials");
                        ZmqError::EINVAL
                    })?;
                let private_key = credentials.get(CredentialType::PrivateKey)
                    .ok_or_else(|| {
                        error!("Failed to get private key from credentials");
                        ZmqError::EINVAL
                    })?;
                
                debug!("Setting our public key (Z85 format): {}", public_key);
                debug!("Setting our private key (Z85 format): {}", private_key);
                
                socket.set_curve_publickey(public_key.as_bytes())
                    .map_err(|e| {
                        error!("Failed to set curve public key: {}", e);
                        e
                    })?;
                socket.set_curve_secretkey(private_key.as_bytes())
                    .map_err(|e| {
                        error!("Failed to set curve secret key: {}", e);
                        e
                    })?;
                
                debug!("Curve security setup complete");
            }
        }
        
        // Bind or connect based on socket type
        let address = Self::build_address(&options)?;
        match options.class_type {
            SocketClassType::Server => {
                socket.bind(&address)?;
                info!("Bound server socket to: {}", address);
            }
            SocketClassType::Client => {
                socket.connect(&address)?;
                info!("Connected client socket to: {}", address);
            }
        }
        
        let local_id = options.local_id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());
        
        Ok(Self {
            socket,
            context,
            options,
            credentials,
            timeout_receive_ms,
            timeout_poll_ms,
            local_id,
        })
    }
    
    fn get_socket_type(options: &SocketOptions) -> Result<SocketType, ZmqError> {
        match options.class_type {
            SocketClassType::Client => Ok(SocketType::DEALER),
            SocketClassType::Server => Ok(SocketType::ROUTER),
        }
    }
    
    fn build_address(options: &SocketOptions) -> Result<String, ZmqError> {
        match options.scheme {
            UriScheme::Tcp => {
                let port = options.port.ok_or(ZmqError::EINVAL)?;
                Ok(format!("tcp://{}:{}", options.host, port))
            }
            UriScheme::Inproc => {
                Ok(format!("inproc://{}", options.host))
            }
        }
    }
    
    pub fn send(&self, message: &ProtoMessage) -> Result<(), ZmqError> {
        // For ROUTER sockets, we need to send the identity first
        if let SocketClassType::Server = self.options.class_type {
            // Send correlation ID as routing frame
            self.socket.send(&message.correlation_id, zmq::SNDMORE)?;
        }
        
        // Send message type
        let msg_type_bytes = message.message_type.to_le_bytes();
        self.socket.send(&msg_type_bytes[..], zmq::SNDMORE)?;
        
        // Send protocol ID
        let proto_id_bytes = [message.proto_id];
        self.socket.send(&proto_id_bytes[..], zmq::SNDMORE)?;
        
        // Send payload
        self.socket.send(&message.payload, 0)?;
        
        debug!("Sent message: correlation_id={}, type={}, proto_id={}", 
               message.correlation_id, message.message_type, message.proto_id);
        
        Ok(())
    }
    
    pub fn receive(&self, message_type: MessageType) -> Response {
        let mut response = Response::default();
        
        match message_type {
            MessageType::GoogleProtocolBuffer => {
                // Receive correlation ID (for ROUTER sockets)
                let correlation_id = if let SocketClassType::Server = self.options.class_type {
                    match self.socket.recv_string(0) {
                        Ok(Ok(id)) => id,
                        Ok(Err(_)) => {
                            response.error = true;
                            response.error_msg = "Failed to receive correlation ID".to_string();
                            return response;
                        }
                        Err(e) => {
                            if e == ZmqError::EAGAIN {
                                response.time_out = true;
                            } else {
                                response.error = true;
                                response.error_msg = format!("Error receiving correlation ID: {}", e);
                            }
                            return response;
                        }
                    }
                } else {
                    Uuid::new_v4().to_string()
                };
                
                // Receive message type
                let msg_type_msg = match self.socket.recv_msg(0) {
                    Ok(msg) => msg,
                    Err(e) => {
                        if e == ZmqError::EAGAIN {
                            response.time_out = true;
                        } else {
                            response.error = true;
                            response.error_msg = format!("Error receiving message type: {}", e);
                        }
                        return response;
                    }
                };
                
                if msg_type_msg.len() != 2 {
                    response.error = true;
                    response.error_msg = "Invalid message type frame size".to_string();
                    return response;
                }
                
                let message_type = u16::from_le_bytes([msg_type_msg[0], msg_type_msg[1]]);
                
                // Receive protocol ID
                let proto_id_msg = match self.socket.recv_msg(0) {
                    Ok(msg) => msg,
                    Err(e) => {
                        if e == ZmqError::EAGAIN {
                            response.time_out = true;
                        } else {
                            response.error = true;
                            response.error_msg = format!("Error receiving protocol ID: {}", e);
                        }
                        return response;
                    }
                };
                
                if proto_id_msg.len() != 1 {
                    response.error = true;
                    response.error_msg = "Invalid protocol ID frame size".to_string();
                    return response;
                }
                
                let proto_id = proto_id_msg[0];
                
                // Receive payload
                let payload_msg = match self.socket.recv_msg(0) {
                    Ok(msg) => msg,
                    Err(e) => {
                        if e == ZmqError::EAGAIN {
                            response.time_out = true;
                        } else {
                            response.error = true;
                            response.error_msg = format!("Error receiving payload: {}", e);
                        }
                        return response;
                    }
                };
                
                let payload = payload_msg.to_vec();
                
                debug!("Received message: correlation_id={}, type={}, proto_id={}", 
                       correlation_id, message_type, proto_id);
                
                // Create message
                let message = ProtoMessage::new(
                    correlation_id,
                    MessageState::Request, // Assume request for now
                    payload,
                    message_type,
                    proto_id,
                );
                
                response.message = Some(message);
            }
            MessageType::String => {
                // For string messages, just receive the string
                match self.socket.recv_string(0) {
                    Ok(Ok(msg)) => {
                        let message = ProtoMessage::new(
                            Uuid::new_v4().to_string(),
                            MessageState::Request,
                            msg.into_bytes(),
                            0, // No message type for strings
                            0, // No protocol ID for strings
                        );
                        response.message = Some(message);
                    }
                    Ok(Err(_)) => {
                        response.error = true;
                        response.error_msg = "Failed to receive string message".to_string();
                    }
                    Err(e) => {
                        if e == ZmqError::EAGAIN {
                            response.time_out = true;
                        } else {
                            response.error = true;
                            response.error_msg = format!("Error receiving string message: {}", e);
                        }
                    }
                }
            }
        }
        
        response
    }
    
    pub fn poll(&self, message_type: MessageType) -> Response {
        // For now, just call receive directly
        // In a full implementation, this would use zmq_poll
        self.receive(message_type)
    }
    
    pub fn id(&self) -> &str {
        &self.local_id
    }
    
    pub fn address(&self) -> String {
        Self::build_address(&self.options).unwrap_or_else(|_| "unknown".to_string())
    }
}

/// Message factory for creating Protocol Buffer messages
pub struct MessageFactory;

impl MessageFactory {
    pub fn new() -> Self {
        Self
    }
    
    pub fn create_version_request(&self) -> ProtoMessage {
        let request = version::VersionRequest {};
        let payload = request.encode_to_vec();
        
        ProtoMessage::new(
            Uuid::new_v4().to_string(),
            MessageState::Request,
            payload,
            1, // Version request type
            1, // Anonymous protocol ID
        )
    }
    
    pub fn create_version_reply(&self, correlation_id: String) -> ProtoMessage {
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
        
        ProtoMessage::new(
            correlation_id,
            MessageState::Response,
            payload,
            2, // Version reply type
            1, // Anonymous protocol ID
        )
    }
    
    pub fn create_repo_data_delete_request(&self, record_ids: Vec<String>, file_paths: Vec<String>) -> ProtoMessage {
        let request = sdms_auth::RepoDataDeleteRequest {
            record_ids,
            file_paths,
        };
        let payload = request.encode_to_vec();
        
        ProtoMessage::new(
            Uuid::new_v4().to_string(),
            MessageState::Request,
            payload,
            3, // Repo data delete request type
            2, // Auth protocol ID
        )
    }
    
    pub fn create_repo_data_get_size_request(&self, record_ids: Vec<String>, file_paths: Vec<String>) -> ProtoMessage {
        let request = sdms_auth::RepoDataGetSizeRequest {
            record_ids,
            file_paths,
        };
        let payload = request.encode_to_vec();
        
        ProtoMessage::new(
            Uuid::new_v4().to_string(),
            MessageState::Request,
            payload,
            4, // Repo data get size request type
            2, // Auth protocol ID
        )
    }
    
    pub fn create_repo_path_create_request(&self, path: String) -> ProtoMessage {
        let request = sdms_auth::RepoPathCreateRequest {
            path,
        };
        let payload = request.encode_to_vec();
        
        ProtoMessage::new(
            Uuid::new_v4().to_string(),
            MessageState::Request,
            payload,
            5, // Repo path create request type
            2, // Auth protocol ID
        )
    }
    
    pub fn create_repo_path_delete_request(&self, path: String) -> ProtoMessage {
        let request = sdms_auth::RepoPathDeleteRequest {
            path,
        };
        let payload = request.encode_to_vec();
        
        ProtoMessage::new(
            Uuid::new_v4().to_string(),
            MessageState::Request,
            payload,
            6, // Repo path delete request type
            2, // Auth protocol ID
        )
    }
    
    pub fn create_ack_reply(&self, correlation_id: String, message: Option<String>) -> ProtoMessage {
        let reply = sdms_auth::AckReply {
            message,
        };
        let payload = reply.encode_to_vec();
        
        ProtoMessage::new(
            correlation_id,
            MessageState::Response,
            payload,
            7, // Ack reply type
            2, // Auth protocol ID
        )
    }
    
    pub fn create_nack_reply(&self, correlation_id: String, err_code: sdms::ErrorCode, err_msg: String) -> ProtoMessage {
        let reply = sdms_auth::NackReply {
            err_code: err_code as i32,
            err_msg,
        };
        let payload = reply.encode_to_vec();
        
        ProtoMessage::new(
            correlation_id,
            MessageState::Response,
            payload,
            8, // Nack reply type
            2, // Auth protocol ID
        )
    }
}

/// Communicator factory for creating ZeroMQ communicators
pub struct CommunicatorFactory;

impl CommunicatorFactory {
    pub fn new() -> Self {
        Self
    }
    
    pub fn create(
        &self,
        options: SocketOptions,
        credentials: MockCredentials,
        timeout_receive_ms: u32,
        timeout_poll_ms: i64,
    ) -> Result<ZeroMQCommunicator, ZmqError> {
        ZeroMQCommunicator::new(options, credentials, timeout_receive_ms, timeout_poll_ms)
    }
}
