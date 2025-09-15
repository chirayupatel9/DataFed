use std::{
    io,
    path::PathBuf,
    sync::{Arc, atomic::{AtomicBool, Ordering}},
    thread,
    time::Duration,
};

use crate::version::{VersionReply, SharedCoreVersionInfo};
use crate::ffi::repo::*;

// ===== Envelope + Transport (kept minimal so it works today) =====
#[derive(Debug, Clone)]
pub struct Envelope {
    pub correlation_id: String,
    pub msg_type: u16,
    pub payload: Vec<u8>,
    pub key: Option<String>,
}

pub trait Messenger: Send + Sync + 'static {
    fn recv(&self, _timeout_ms: i32) -> io::Result<Option<Envelope>> { Ok(None) }
    fn send(&self, _env: Envelope) -> io::Result<()> { Ok(()) }
}

/// Real ZMQ INPROC messenger that connects to the C++ messaging system
/// This mimics exactly how the C++ RequestWorker connects to the proxy
#[derive(Clone)]
pub struct ZMQInprocMessenger {
    worker_id: usize,
}

impl ZMQInprocMessenger {
    pub fn new(worker_id: usize) -> Self {
        Self { worker_id }
    }
}

impl Messenger for ZMQInprocMessenger {
    fn recv(&self, timeout_ms: i32) -> io::Result<Option<Envelope>> {
        // Use the C++ FFI bridge to receive messages from the INPROC socket
        match zmq_recv(timeout_ms) {
            Ok(payload) => {
                // Check if we got an empty payload (timeout or no message)
                if payload.is_empty() {
                    return Ok(None); // Timeout, no message available
                }
                
                // Parse the received payload into an Envelope
                // For now, we'll create a simple envelope structure
                // In a real implementation, you'd deserialize the protobuf message
                // and extract the correlation_id, msg_type, etc.
                
                // This is a simplified parsing - in practice you'd use proper protobuf deserialization
                if payload.len() < 4 {
                    return Ok(None); // Invalid message format
                }
                
                // Extract message type from first 2 bytes (little-endian)
                let msg_type = u16::from_le_bytes([payload[0], payload[1]]);
                
                // For now, use a default correlation ID - in practice this would come from the message
                let correlation_id = format!("worker_{}_{}", self.worker_id, std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis());
                
                // The rest is the actual payload
                let message_payload = payload[2..].to_vec();
                
                Ok(Some(Envelope {
                    correlation_id,
                    msg_type,
                    payload: message_payload,
                    key: None,
                }))
            }
            Err(e) => {
                eprintln!("ZMQ recv error: {}", e);
                Err(io::Error::new(io::ErrorKind::Other, format!("ZMQ recv failed: {}", e)))
            }
        }
    }

    fn send(&self, env: Envelope) -> io::Result<()> {
        // Use the C++ FFI bridge to send messages through the INPROC socket
        // Create a simple binary format: [msg_type:2][payload:...]
        let mut payload = Vec::new();
        payload.extend_from_slice(&env.msg_type.to_le_bytes());
        payload.extend_from_slice(&env.payload);
        
        match zmq_send(&payload, env.msg_type, &env.correlation_id) {
            Ok(()) => Ok(()),
            Err(e) => {
                eprintln!("ZMQ send error: {}", e);
                Err(io::Error::new(io::ErrorKind::Other, format!("ZMQ send failed: {}", e)))
            }
        }
    }
}

impl Default for ZMQInprocMessenger {
    fn default() -> Self {
        Self::new(0)
    }
}

// ===== Message type IDs (adjust to your real mapper when you wire proto) =====
const MT_VERSION_REQUEST: u16            = 1;
const MT_VERSION_REPLY: u16              = 2;
const MT_REPO_DATA_DELETE_REQUEST: u16   = 1001;
const MT_REPO_DATA_GET_SIZE_REQUEST: u16 = 1002;
const MT_REPO_PATH_CREATE_REQUEST: u16   = 1003;
const MT_REPO_PATH_DELETE_REQUEST: u16   = 1004;
// simple acks/nacks for now
const MT_ACK_REPLY: u16                  = 1100;
const MT_NACK_REPLY: u16                 = 9999;

/// Spawn one worker thread. Replace `NoopMessenger` with your real inproc client later.
pub fn spawn_worker<M: Messenger + Clone + 'static>(
    id: usize,
    messenger: M,
    base_path: impl Into<PathBuf>,
    running: Arc<AtomicBool>,
    version_info: SharedCoreVersionInfo,
) -> std::thread::JoinHandle<()> {
    let base_root = base_path.into();

    thread::spawn(move || {
        println!("worker[{id}] starting, running flag: {}", running.load(Ordering::Relaxed));
        while running.load(Ordering::Relaxed) {
            match messenger.recv(1000) { // Increased timeout to 100ms
                Ok(Some(env)) => {
                    println!("worker[{id}] received message: {:?}", env.msg_type);
                    // --- decode, dispatch, reply (DONE) ---
                    let correlation_id = env.correlation_id.clone();

                    // Route by message type. For now:
                    // - VersionRequest -> VersionReply (empty payload stub)
                    // - Repo* requests  -> AckReply (empty payload stub)
                    // - Unknown         -> NackReply (json payload with error)
                    let (reply_type, payload) = match env.msg_type {
                        MT_VERSION_REQUEST => {
                            // Use version info from core server if available, otherwise fall back to local
                            let version_reply = {
                                let info = version_info.read().unwrap();
                                if info.is_connected {
                                    info.version_reply.clone()
                                } else {
                                    VersionReply::new() // fallback to local version
                                }
                            };
                            (MT_VERSION_REPLY, version_reply.to_bytes())
                        }
                        MT_REPO_DATA_DELETE_REQUEST => {
                            // Parse payload to get paths and delete under base_root
                            match parse_delete_request(&env.payload) {
                                Ok(paths) => {
                                    let mut success_count = 0;
                                    let mut error_count = 0;
                                    
                                    for path in paths {
                                        let full_path = base_root.join(&path);
                                        
                                        // Security check: ensure path is within base_root
                                        if !full_path.starts_with(&base_root) {
                                            eprintln!("Security violation: path {} is outside base root", full_path.display());
                                            error_count += 1;
                                            continue;
                                        }
                                        
                                        match std::fs::remove_file(&full_path) {
                                            Ok(()) => {
                                                println!("Deleted file: {}", full_path.display());
                                                success_count += 1;
                                            }
                                            Err(e) => {
                                                eprintln!("Failed to delete {}: {}", full_path.display(), e);
                                                error_count += 1;
                                            }
                                        }
                                    }
                                    
                                    if error_count == 0 {
                                        (MT_ACK_REPLY, Vec::new())
                                    } else {
                                        let error_msg = format!("Deleted {} files, {} errors", success_count, error_count);
                                        let body = encode_nack(-1, error_msg);
                                        (MT_NACK_REPLY, body)
                                    }
                                }
                                Err(e) => {
                                    let body = encode_nack(-3, format!("Failed to parse delete request: {}", e));
                                    (MT_NACK_REPLY, body)
                                }
                            }
                        }
                        MT_REPO_DATA_GET_SIZE_REQUEST => {
                            // Parse payload to get paths and compute sizes under base_root
                            match parse_size_request(&env.payload) {
                                Ok(paths) => {
                                    let mut results = Vec::new();
                                    let mut total_size = 0u64;
                                    let mut error_count = 0;
                                    
                                    for path in paths {
                                        let full_path = base_root.join(&path);
                                        
                                        // Security check: ensure path is within base_root
                                        if !full_path.starts_with(&base_root) {
                                            eprintln!("Security violation: path {} is outside base root", full_path.display());
                                            error_count += 1;
                                            continue;
                                        }
                                        
                                        match std::fs::metadata(&full_path) {
                                            Ok(metadata) => {
                                                let size = metadata.len();
                                                total_size += size;
                                                results.push((path, size));
                                                println!("File: {}, Size: {} bytes", full_path.display(), size);
                                            }
                                            Err(e) => {
                                                eprintln!("Failed to get metadata for {}: {}", full_path.display(), e);
                                                error_count += 1;
                                            }
                                        }
                                    }
                                    
                                    if error_count == 0 {
                                        // Create size reply with all file sizes
                                        let reply = create_size_reply(&results, total_size);
                                        (MT_ACK_REPLY, reply)
                                    } else {
                                        let error_msg = format!("Processed {} files, {} errors", results.len(), error_count);
                                        let body = encode_nack(-1, error_msg);
                                        (MT_NACK_REPLY, body)
                                    }
                                }
                                Err(e) => {
                                    let body = encode_nack(-3, format!("Failed to parse size request: {}", e));
                                    (MT_NACK_REPLY, body)
                                }
                            }
                        }
                        MT_REPO_PATH_CREATE_REQUEST => {
                            // Parse payload to get path and create directory under base_root
                            match parse_path_request(&env.payload) {
                                Ok(path) => {
                                    let full_path = base_root.join(&path);
                                    
                                    // Security check: ensure path is within base_root
                                    if !full_path.starts_with(&base_root) {
                                        let body = encode_nack(-4, format!("Security violation: path {} is outside base root", full_path.display()));
                                        (MT_NACK_REPLY, body)
                                    } else {
                                        // Create directory with -p equivalent (create parent directories)
                                        match std::fs::create_dir_all(&full_path) {
                                            Ok(()) => {
                                                println!("Created directory: {}", full_path.display());
                                                (MT_ACK_REPLY, Vec::new())
                                            }
                                            Err(e) => {
                                                let body = encode_nack(-5, format!("Failed to create directory {}: {}", full_path.display(), e));
                                                (MT_NACK_REPLY, body)
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    let body = encode_nack(-3, format!("Failed to parse path create request: {}", e));
                                    (MT_NACK_REPLY, body)
                                }
                            }
                        }
                        MT_REPO_PATH_DELETE_REQUEST => {
                            // Parse payload to get path and recursively delete directory under base_root
                            match parse_path_request(&env.payload) {
                                Ok(path) => {
                                    let full_path = base_root.join(&path);
                                    
                                    // Security check: ensure path is within base_root
                                    if !full_path.starts_with(&base_root) {
                                        let body = encode_nack(-4, format!("Security violation: path {} is outside base root", full_path.display()));
                                        (MT_NACK_REPLY, body)
                                    } else if full_path == base_root {
                                        // Additional security check: prevent deletion of base_root itself
                                        let body = encode_nack(-6, "Cannot delete base root directory".to_string());
                                        (MT_NACK_REPLY, body)
                                    } else {
                                        // Recursively delete directory (rm -r equivalent)
                                        match std::fs::remove_dir_all(&full_path) {
                                            Ok(()) => {
                                                println!("Deleted directory: {}", full_path.display());
                                                (MT_ACK_REPLY, Vec::new())
                                            }
                                            Err(e) => {
                                                let body = encode_nack(-7, format!("Failed to delete directory {}: {}", full_path.display(), e));
                                                (MT_NACK_REPLY, body)
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    let body = encode_nack(-3, format!("Failed to parse path delete request: {}", e));
                                    (MT_NACK_REPLY, body)
                                }
                            }
                        }
                        _ => {
                            let body = encode_nack(-2, format!("unsupported msg type {}", env.msg_type));
                            (MT_NACK_REPLY, body)
                        }
                    };

                    // Send reply (preserve correlation id)
                    let _ = messenger.send(Envelope {
                        correlation_id,
                        msg_type: reply_type,
                        payload,
                        key: None,
                    });
                }
                Ok(None) => { 
                    // poll timeout - add small delay to prevent busy waiting
                    thread::sleep(Duration::from_millis(10));
                }
                Err(e) => {
                    eprintln!("worker[{id}] recv error: {e}");
                    thread::sleep(Duration::from_millis(5));
                }
            }
        }
        println!("worker[{id}] exiting, running flag: {}", running.load(Ordering::Relaxed));
    })
}

// simple JSON NACK payload for now (so you can inspect in tests/logs)
fn encode_nack(code: i32, msg: String) -> Vec<u8> {
    format!(r#"{{"err_code":{},"err_msg":"{}"}}"#, code, msg.replace('"', "'")).into_bytes()
}

/// Parse delete request payload to extract file paths
/// Expected format: JSON array of strings representing relative paths
/// Example: ["file1.txt", "subdir/file2.txt"]
fn parse_delete_request(payload: &[u8]) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    if payload.is_empty() {
        return Err("Empty payload".into());
    }
    
    // Try to parse as JSON array of strings
    let payload_str = std::str::from_utf8(payload)
        .map_err(|e| format!("Invalid UTF-8 in payload: {}", e))?;
    
    // Simple JSON parsing for array of strings
    // In a real implementation, you'd use serde_json
    let trimmed = payload_str.trim();
    if !trimmed.starts_with('[') || !trimmed.ends_with(']') {
        return Err("Payload is not a JSON array".into());
    }
    
    let content = &trimmed[1..trimmed.len()-1]; // Remove [ and ]
    if content.trim().is_empty() {
        return Ok(Vec::new()); // Empty array
    }
    
    let mut paths = Vec::new();
    let mut current_path = String::new();
    let mut in_quotes = false;
    let mut escape_next = false;
    
    for ch in content.chars() {
        if escape_next {
            current_path.push(ch);
            escape_next = false;
        } else if ch == '\\' {
            escape_next = true;
        } else if ch == '"' {
            in_quotes = !in_quotes;
        } else if ch == ',' && !in_quotes {
            // End of current path
            let trimmed_path = current_path.trim();
            if !trimmed_path.is_empty() {
                paths.push(trimmed_path.to_string());
            }
            current_path.clear();
        } else {
            current_path.push(ch);
        }
    }
    
    // Don't forget the last path
    let trimmed_path = current_path.trim();
    if !trimmed_path.is_empty() {
        paths.push(trimmed_path.to_string());
    }
    
    if paths.is_empty() {
        return Err("No valid paths found in payload".into());
    }
    
    Ok(paths)
}

/// Parse size request payload to extract file paths
/// Expected format: JSON array of strings representing relative paths
/// Example: ["file1.txt", "subdir/file2.txt"]
fn parse_size_request(payload: &[u8]) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    // Reuse the same parsing logic as delete request
    parse_delete_request(payload)
}

/// Parse path request payload to extract a single path
/// Expected format: JSON string representing a relative path
/// Example: "subdir/newdir"
fn parse_path_request(payload: &[u8]) -> Result<String, Box<dyn std::error::Error>> {
    if payload.is_empty() {
        return Err("Empty payload".into());
    }
    
    // Try to parse as JSON string
    let payload_str = std::str::from_utf8(payload)
        .map_err(|e| format!("Invalid UTF-8 in payload: {}", e))?;
    
    let trimmed = payload_str.trim();
    if !trimmed.starts_with('"') || !trimmed.ends_with('"') {
        return Err("Payload is not a JSON string".into());
    }
    
    // Extract content between quotes, handling escape sequences
    let content = &trimmed[1..trimmed.len()-1]; // Remove " and "
    let mut result = String::new();
    let mut escape_next = false;
    
    for ch in content.chars() {
        if escape_next {
            result.push(ch);
            escape_next = false;
        } else if ch == '\\' {
            escape_next = true;
        } else {
            result.push(ch);
        }
    }
    
    if result.is_empty() {
        return Err("Empty path".into());
    }
    
    Ok(result)
}

/// Create a size reply with file sizes and total
/// Format: JSON object with file sizes and total
/// Example: {"files": {"file1.txt": 1024, "subdir/file2.txt": 2048}, "total": 3072}
fn create_size_reply(results: &[(String, u64)], total_size: u64) -> Vec<u8> {
    let mut json = String::from("{\"files\":{");
    
    // Add individual file sizes
    for (i, (path, size)) in results.iter().enumerate() {
        if i > 0 {
            json.push(',');
        }
        json.push_str(&format!("\"{}\":{}", path.replace('"', "\\\""), size));
    }
    
    // Add total size
    json.push_str(&format!("}},\"total\":{}}}", total_size));
    
    json.into_bytes()
}
