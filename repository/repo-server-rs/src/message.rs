// Message handling system - equivalent to C++ IMessage interface
use std::collections::HashMap;
use crate::proto::*;

/// Message attributes - equivalent to C++ MessageAttribute enum
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MessageAttribute {
    CorrelationId,
    Key,
    MessageType,
}

/// Message types - equivalent to C++ MessageType enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    GoogleProtocolBuffer,
}

/// Error codes - equivalent to C++ ErrorCode enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    BadRequest = -1,
    InternalError = -2,
    ParseError = -3,
    SecurityViolation = -4,
    DirectoryCreationError = -5,
    BaseRootDeletionError = -6,
    DirectoryDeletionError = -7,
}

/// Message payload enum - simpler approach than trait objects
#[derive(Debug, Clone)]
pub enum MessagePayload {
    Empty,
    Nack(NackReply),
    Ack(AckReply),
    VersionRequest(VersionRequest),
    VersionReply(VersionReply),
    RepoDataDeleteRequest(RepoDataDeleteRequest),
    RepoDataGetSizeRequest(RepoDataGetSizeRequest),
    RepoPathCreateRequest(RepoPathCreateRequest),
    RepoPathDeleteRequest(RepoPathDeleteRequest),
    RepoDataSizeReply(RepoDataSizeReply),
}

/// IMessage equivalent - represents a message envelope
#[derive(Debug, Clone)]
pub struct Message {
    pub correlation_id: Option<String>,
    pub key: Option<String>,
    pub message_type: u16,
    pub payload: MessagePayload,
    pub attributes: HashMap<MessageAttribute, String>,
}

impl Message {
    pub fn new(message_type: u16, payload: MessagePayload) -> Self {
        Self {
            correlation_id: None,
            key: None,
            message_type,
            payload,
            attributes: HashMap::new(),
        }
    }

    pub fn set_correlation_id(&mut self, id: String) {
        self.correlation_id = Some(id.clone());
        self.attributes.insert(MessageAttribute::CorrelationId, id);
    }

    pub fn set_key(&mut self, key: String) {
        self.key = Some(key.clone());
        self.attributes.insert(MessageAttribute::Key, key);
    }

    pub fn get_correlation_id(&self) -> Option<&String> {
        self.correlation_id.as_ref()
    }

    pub fn get_key(&self) -> Option<&String> {
        self.key.as_ref()
    }

    pub fn get_message_type(&self) -> u16 {
        self.message_type
    }

    pub fn get_payload(&self) -> &MessagePayload {
        &self.payload
    }

    pub fn get_payload_mut(&mut self) -> &mut MessagePayload {
        &mut self.payload
    }

    /// Deserialize a message from raw bytes
    pub fn deserialize(data: &[u8]) -> Result<Message, Box<dyn std::error::Error>> {
        if data.len() < 2 {
            return Err("Message too short".into());
        }

        // Extract message type (first 2 bytes, little-endian)
        let msg_type = u16::from_le_bytes([data[0], data[1]]);
        let payload_data = &data[2..];

        let payload = match msg_type {
            message_types::VERSION_REQUEST => {
                MessagePayload::VersionRequest(VersionRequest::deserialize(payload_data)?)
            }
            message_types::VERSION_REPLY => {
                MessagePayload::VersionReply(VersionReply::deserialize(payload_data)?)
            }
            message_types::REPO_DATA_DELETE_REQUEST => {
                MessagePayload::RepoDataDeleteRequest(RepoDataDeleteRequest::deserialize(payload_data)?)
            }
            message_types::REPO_DATA_GET_SIZE_REQUEST => {
                MessagePayload::RepoDataGetSizeRequest(RepoDataGetSizeRequest::deserialize(payload_data)?)
            }
            message_types::REPO_PATH_CREATE_REQUEST => {
                MessagePayload::RepoPathCreateRequest(RepoPathCreateRequest::deserialize(payload_data)?)
            }
            message_types::REPO_PATH_DELETE_REQUEST => {
                MessagePayload::RepoPathDeleteRequest(RepoPathDeleteRequest::deserialize(payload_data)?)
            }
            message_types::REPO_DATA_SIZE_REPLY => {
                MessagePayload::RepoDataSizeReply(RepoDataSizeReply::deserialize(payload_data)?)
            }
            message_types::ACK_REPLY => {
                MessagePayload::Ack(AckReply::deserialize(payload_data)?)
            }
            message_types::NACK_REPLY => {
                MessagePayload::Nack(NackReply::deserialize(payload_data)?)
            }
            _ => {
                return Err(format!("Unknown message type: {}", msg_type).into());
            }
        };

        Ok(Message::new(msg_type, payload))
    }

    /// Serialize a message to raw bytes
    pub fn serialize(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let mut data = Vec::new();
        
        // Add message type (2 bytes, little-endian)
        data.extend_from_slice(&self.message_type.to_le_bytes());
        
        // Add payload data
        let payload_data = match &self.payload {
            MessagePayload::Empty => Vec::new(),
            MessagePayload::Nack(nack) => nack.serialize()?,
            MessagePayload::Ack(ack) => ack.serialize()?,
            MessagePayload::VersionRequest(req) => req.serialize()?,
            MessagePayload::VersionReply(rep) => rep.serialize()?,
            MessagePayload::RepoDataDeleteRequest(req) => req.serialize()?,
            MessagePayload::RepoDataGetSizeRequest(req) => req.serialize()?,
            MessagePayload::RepoPathCreateRequest(req) => req.serialize()?,
            MessagePayload::RepoPathDeleteRequest(req) => req.serialize()?,
            MessagePayload::RepoDataSizeReply(rep) => rep.serialize()?,
        };
        
        data.extend_from_slice(&payload_data);
        Ok(data)
    }
}

/// Message factory - equivalent to C++ MessageFactory
pub struct MessageFactory;

impl MessageFactory {
    pub fn new() -> Self {
        Self
    }

    pub fn create(&self, _message_type: MessageType) -> Message {
        // Create a placeholder message - in practice this would create the appropriate message type
        Message::new(0, MessagePayload::Empty)
    }

    pub fn create_response_envelope(&self, request: &Message) -> Message {
        let mut response = Message::new(0, MessagePayload::Empty);
        if let Some(correlation_id) = request.get_correlation_id() {
            response.set_correlation_id(correlation_id.clone());
        }
        response
    }
}

/// Message type constants - matching Python client message types
pub mod message_types {
    // Anonymous protocol messages (from Python _msg_name_to_type)
    pub const VERSION_REQUEST: u16 = 258;
    pub const VERSION_REPLY: u16 = 2;
    pub const ACK_REPLY: u16 = 1100;
    pub const NACK_REPLY: u16 = 9999;

    // Authorized protocol messages (from Python _msg_name_to_type)
    pub const REPO_DATA_DELETE_REQUEST: u16 = 591;
    pub const REPO_DATA_GET_SIZE_REQUEST: u16 = 592;
    pub const REPO_PATH_CREATE_REQUEST: u16 = 594;
    pub const REPO_PATH_DELETE_REQUEST: u16 = 595;
    pub const REPO_DATA_SIZE_REPLY: u16 = 13323; // Match C++ bridge expectations
}

/// NackReply message - equivalent to C++ NackReply
#[derive(Debug, Clone)]
pub struct NackReply {
    pub err_code: i32,
    pub err_msg: String,
}

impl NackReply {
    pub fn get_message_type(&self) -> u16 {
        message_types::NACK_REPLY
    }

    pub fn serialize(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        // Protobuf serialization for NackReply
        // Field 1: err_code (varint) - tag 0x08 (field 1, wire type 0)
        // Field 2: err_msg (string) - tag 0x12 (field 2, wire type 2)
        let mut data = Vec::new();
        
        // Field 1: err_code (varint encoding with zigzag for negative numbers)
        data.push(0x08); // Field 1, wire type 0 (varint)
        
        // Convert to zigzag encoding for negative numbers
        let zigzag_code = ((self.err_code << 1) ^ (self.err_code >> 31)) as u32;
        let mut code = zigzag_code as u64;
        while code >= 0x80 {
            data.push((code as u8) | 0x80);
            code >>= 7;
        }
        data.push(code as u8);
        
        // Field 2: err_msg (string encoding)
        if !self.err_msg.is_empty() {
            let msg_bytes = self.err_msg.as_bytes();
            data.push(0x12); // Field 2, wire type 2 (string)
            
            // Encode length as varint
            let mut len = msg_bytes.len() as u64;
            while len >= 0x80 {
                data.push((len as u8) | 0x80);
                len >>= 7;
            }
            data.push(len as u8);
            
            data.extend_from_slice(msg_bytes);
        }
        
        // Debug: print the serialized data
        println!("NackReply::serialize: err_code={}, err_msg='{}', serialized_len={}, data={:?}", 
                 self.err_code, self.err_msg, data.len(), data);
        Ok(data)
    }

    pub fn deserialize(data: &[u8]) -> Result<NackReply, Box<dyn std::error::Error>> {
        let json_str = std::str::from_utf8(data)?;
        // Simple JSON parsing - in practice you'd use serde_json
        let json_str = json_str.trim();
        if !json_str.starts_with('{') || !json_str.ends_with('}') {
            return Err("Invalid JSON format".into());
        }
        
        // Extract err_code and err_msg (simplified parsing)
        let mut err_code = 0;
        let mut err_msg = String::new();
        
        if let Some(code_start) = json_str.find("\"err_code\":") {
            let code_part = &json_str[code_start + 11..];
            if let Some(code_end) = code_part.find(',') {
                if let Ok(code) = code_part[..code_end].trim().parse::<i32>() {
                    err_code = code;
                }
            }
        }
        
        if let Some(msg_start) = json_str.find("\"err_msg\":\"") {
            let msg_part = &json_str[msg_start + 11..];
            if let Some(msg_end) = msg_part.find('"') {
                err_msg = msg_part[..msg_end].to_string();
            }
        }
        
        Ok(NackReply { err_code, err_msg })
    }
}

/// AckReply message - equivalent to C++ AckReply
#[derive(Debug, Clone)]
pub struct AckReply;

impl AckReply {
    pub fn get_message_type(&self) -> u16 {
        message_types::ACK_REPLY
    }

    pub fn serialize(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        Ok(Vec::new()) // Empty payload
    }

    pub fn deserialize(_data: &[u8]) -> Result<AckReply, Box<dyn std::error::Error>> {
        Ok(AckReply)
    }
}
