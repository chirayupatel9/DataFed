use std::{
    io,
    path::PathBuf,
    sync::{Arc, atomic::{AtomicBool, Ordering}},
    thread,
    time::Duration,
};

use crate::version::SharedCoreVersionInfo;
use crate::proto::VersionReply;
use crate::ffi::repo::*;
use crate::message::*;
use crate::path_utils::*;
use crate::ffi::dynalog::{LogCtx, level};
use crate::{dl_info, dl_log};
use crate::proto::{RepoDataSizeReply, RecordDataSize};
use crate::message::{NackReply, AckReply, ErrorCode};

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
                
        // Get the correlation ID from the C++ side
        let correlation_id = get_last_correlation_id();
        println!("🔍 ZMQInprocMessenger::recv: Retrieved correlation_id from C++: {}", correlation_id);
        println!("🔍 ZMQInprocMessenger::recv: This correlation_id will be used for the response");
                
                // The rest is the actual payload
                let message_payload = payload[2..].to_vec();
                
                // Debug logging
                println!("Worker {} received message: type={} (0x{:x}), payload_len={}", 
                         self.worker_id, msg_type, msg_type, message_payload.len());
                
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
        // Send the payload directly - the C++ side will handle message type routing
        println!("🔵 ZMQInprocMessenger::send called: msg_type={} (0x{:x}), corr_id={}, payload_len={}", 
                 env.msg_type, env.msg_type, env.correlation_id, env.payload.len());
        
        // Debug: print first few bytes of payload
        let debug_len = std::cmp::min(20, env.payload.len());
        let debug_bytes = &env.payload[..debug_len];
        println!("🔵 Payload preview (first {} bytes): {:?}", debug_len, debug_bytes);
        
        // Send through INPROC socket - the ZMQ proxy should forward this back to external clients
        println!("🔵 ZMQInprocMessenger::send: Sending to INPROC socket 'workers'");
        println!("🔵 ZMQInprocMessenger::send: C++ proxy should forward this to external TCP socket localhost:10000");
        println!("🔵 ZMQInprocMessenger::send: Target external client should receive this on their DEALER socket");
        
        match zmq_send(&env.payload, env.msg_type, &env.correlation_id) {
            Ok(()) => {
                println!("✅ ZMQInprocMessenger::send successful - message sent to INPROC socket (proxy should forward)");
                println!("✅ ZMQInprocMessenger::send: C++ zmq_send() completed successfully");
                Ok(())
            },
            Err(e) => {
                eprintln!("❌ ZMQ send error: {}", e);
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

// Use message type constants from the message module
use crate::message::message_types::*;

/// Spawn one worker thread. Replace `NoopMessenger` with your real inproc client later.
pub fn spawn_worker<M: Messenger + Clone + 'static>(
    id: usize,
    messenger: M,
    base_path: impl Into<PathBuf>,
    running: Arc<AtomicBool>,
    version_info: SharedCoreVersionInfo,
) -> std::thread::JoinHandle<()> {
    let base_root = base_path.into();
    let path_sanitizer = PathSanitizer::new(&base_root);

    thread::spawn(move || {
        let log_ctx = LogCtx {
            thread_name: format!("worker-{}", id),
            correlation_id: String::new(),
            thread_id: id as i32,
        };
        
        dl_info!(log_ctx, "Worker {} starting, running flag: {}", id, running.load(Ordering::Relaxed));
        
        while running.load(Ordering::Relaxed) {
            match messenger.recv(1000) { // Increased timeout to 100ms
                Ok(Some(env)) => {
                    let mut msg_ctx = log_ctx.clone();
                    msg_ctx.correlation_id = env.correlation_id.clone();
                    
                    println!("🟢 Worker[{}] received message type: {} (0x{:x}), corr_id: {}, payload_len: {}", 
                             id, env.msg_type, env.msg_type, env.correlation_id, env.payload.len());
                    
                    // Debug: print first few bytes of payload
                    let debug_len = std::cmp::min(20, env.payload.len());
                    let debug_bytes = &env.payload[..debug_len];
                    println!("🟢 Payload preview (first {} bytes): {:?}", debug_len, debug_bytes);
                    
                    dl_info!(msg_ctx, "Received message type: {} (0x{:x})", env.msg_type, env.msg_type);
                    // --- decode, dispatch, reply (DONE) ---
                    let correlation_id = env.correlation_id.clone();

                    // Route by message type with proper error handling
                    let (reply_type, payload) =                     match env.msg_type {
                        VERSION_REQUEST => {
                            dl_info!(msg_ctx, "Processing version request");
                            // Use version info from core server if available, otherwise fall back to local
                            let version_reply = {
                                let info = version_info.read().unwrap();
                                if info.is_connected {
                                    dl_info!(msg_ctx, "Using core server version info");
                                    info.version_reply.clone()
                                } else {
                                    dl_info!(msg_ctx, "Using local fallback version info");
                                    VersionReply::new() // fallback to local version
                                }
                            };
                            dl_info!(msg_ctx, "Version reply: {}.{}.{}", version_reply.component_major, version_reply.component_minor, version_reply.component_patch);
                            // Use protobuf serialization instead of custom binary format
                            match version_reply.serialize() {
                                Ok(payload) => (VERSION_REPLY, payload),
                                Err(e) => {
                                    dl_log!(level::ERROR, msg_ctx, "Failed to serialize version reply: {}", e);
                                    let nack = NackReply {
                                        err_code: ErrorCode::InternalError as i32,
                                        err_msg: format!("Serialization error: {}", e),
                                    };
                                    (NACK_REPLY, nack.serialize().unwrap_or_default())
                                }
                            }
                        }
                        REPO_DATA_DELETE_REQUEST => {
                            dl_info!(msg_ctx, "Processing data delete request");
                            // Parse payload to get paths and delete under base_root
                            match parse_delete_request(&env.payload) {
                                Ok(paths) => {
                                    dl_info!(msg_ctx, "Parsed {} paths for deletion", paths.len());
                                    let mut success_count = 0;
                                    let mut error_count = 0;
                                    
                                    for path in paths {
                                        dl_info!(msg_ctx, "Processing delete for path: {}", path);
                                        // Use path sanitizer for security
                                        match path_sanitizer.sanitize_path(&path) {
                                            Ok(sanitized) => {
                                                dl_info!(msg_ctx, "Sanitized path: {}", sanitized.local_path.display());
                                                // Additional security validation
                                                if let Err(e) = path_sanitizer.validate_path_security(&sanitized.local_path) {
                                                    dl_log!(level::ERROR, msg_ctx, "Security violation: {}", e);
                                                    error_count += 1;
                                                    continue;
                                                }
                                                
                                                match std::fs::remove_file(&sanitized.local_path) {
                                                    Ok(()) => {
                                                        dl_info!(msg_ctx, "Successfully deleted file: {}", sanitized.local_path.display());
                                                        success_count += 1;
                                                    }
                                                    Err(e) => {
                                                        dl_log!(level::ERROR, msg_ctx, "Failed to delete {}: {}", sanitized.local_path.display(), e);
                                                        error_count += 1;
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                dl_log!(level::ERROR, msg_ctx, "Path sanitization failed for {}: {}", path, e);
                                                error_count += 1;
                                            }
                                        }
                                    }
                                    
                                    if error_count == 0 {
                                        // Create empty AckReply
                                        let ack = AckReply;
                                        match ack.serialize() {
                                            Ok(payload) => (ACK_REPLY, payload),
                                            Err(e) => {
                                                dl_log!(level::ERROR, msg_ctx, "Failed to serialize AckReply: {}", e);
                                                (ACK_REPLY, Vec::new()) // Fallback to empty payload
                                            }
                                        }
                                    } else {
                                        let nack = NackReply {
                                            err_code: ErrorCode::InternalError as i32,
                                            err_msg: format!("Deleted {} files, {} errors", success_count, error_count),
                                        };
                                        match nack.serialize() {
                                            Ok(payload) => (NACK_REPLY, payload),
                                            Err(e) => {
                                                dl_log!(level::ERROR, msg_ctx, "Failed to serialize NackReply: {}", e);
                                                (NACK_REPLY, Vec::new()) // Fallback to empty payload
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    let nack = NackReply {
                                        err_code: ErrorCode::ParseError as i32,
                                        err_msg: format!("Failed to parse delete request: {}", e),
                                    };
                                    match nack.serialize() {
                                        Ok(payload) => (NACK_REPLY, payload),
                                        Err(ser_err) => {
                                            dl_log!(level::ERROR, msg_ctx, "Failed to serialize NackReply: {}", ser_err);
                                            (NACK_REPLY, Vec::new())
                                        }
                                    }
                                }
                            }
                        }
                        REPO_DATA_GET_SIZE_REQUEST => {
                            // Parse payload to get paths and compute sizes under base_root
                            match parse_size_request(&env.payload) {
                                Ok(paths) => {
                                    let mut results = Vec::new();
                                    let mut total_size = 0u64;
                                    let mut error_count = 0;
                                    
                                    for path in paths {
                                        // Use path sanitizer for security
                                        match path_sanitizer.sanitize_path(&path) {
                                            Ok(sanitized) => {
                                                // Additional security validation
                                                if let Err(e) = path_sanitizer.validate_path_security(&sanitized.local_path) {
                                                    eprintln!("Security violation: {}", e);
                                                    error_count += 1;
                                                    continue;
                                                }
                                                
                                                match std::fs::metadata(&sanitized.local_path) {
                                                    Ok(metadata) => {
                                                        let size = metadata.len();
                                                        total_size += size;
                                                        results.push((path, size));
                                                        println!("File: {}, Size: {} bytes", sanitized.local_path.display(), size);
                                                    }
                                                    Err(e) => {
                                                        eprintln!("Failed to get metadata for {}: {}", sanitized.local_path.display(), e);
                                                        error_count += 1;
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                eprintln!("Path sanitization failed for {}: {}", path, e);
                                                error_count += 1;
                                            }
                                        }
                                    }
                                    
                                    if error_count == 0 {
                                        // Create proper RepoDataSizeReply
                                        let size_reply = RepoDataSizeReply {
                                            size: results.into_iter().map(|(path, size)| {
                                                RecordDataSize { id: path, size }
                                            }).collect(),
                                        };
                                        match size_reply.serialize() {
                                            Ok(payload) => (REPO_DATA_SIZE_REPLY, payload),
                                            Err(e) => {
                                                dl_log!(level::ERROR, msg_ctx, "Failed to serialize RepoDataSizeReply: {}", e);
                                                let nack = NackReply {
                                                    err_code: ErrorCode::InternalError as i32,
                                                    err_msg: format!("Serialization error: {}", e),
                                                };
                                                match nack.serialize() {
                                                    Ok(payload) => (NACK_REPLY, payload),
                                                    Err(_) => (NACK_REPLY, Vec::new()),
                                                }
                                            }
                                        }
                                    } else {
                                        let nack = NackReply {
                                            err_code: ErrorCode::InternalError as i32,
                                            err_msg: format!("Processed {} files, {} errors", results.len(), error_count),
                                        };
                                        match nack.serialize() {
                                            Ok(payload) => (NACK_REPLY, payload),
                                            Err(e) => {
                                                dl_log!(level::ERROR, msg_ctx, "Failed to serialize NackReply: {}", e);
                                                (NACK_REPLY, Vec::new())
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    let nack = NackReply {
                                        err_code: ErrorCode::ParseError as i32,
                                        err_msg: format!("Failed to parse size request: {}", e),
                                    };
                                    match nack.serialize() {
                                        Ok(payload) => (NACK_REPLY, payload),
                                        Err(ser_err) => {
                                            dl_log!(level::ERROR, msg_ctx, "Failed to serialize NackReply: {}", ser_err);
                                            (NACK_REPLY, Vec::new())
                                        }
                                    }
                                }
                            }
                        }
                        REPO_PATH_CREATE_REQUEST => {
                            println!("worker[{id}] processing RepoPathCreateRequest, payload {:?} len: {}", env.payload ,env.payload.len());
                            // Parse payload to get path and create directory under base_root
                            match parse_path_request(&env.payload) {
                                Ok(path) => {
                                    // Use path sanitizer for security
                                    match path_sanitizer.sanitize_path(&path) {
                                        Ok(sanitized) => {
                                            // Additional security validation
                                            if let Err(e) = path_sanitizer.validate_path_security(&sanitized.local_path) {
                                                let nack = NackReply {
                                                    err_code: ErrorCode::SecurityViolation as i32,
                                                    err_msg: format!("Security violation: {}", e),
                                                };
                                                match nack.serialize() {
                                                    Ok(payload) => (NACK_REPLY, payload),
                                                    Err(ser_err) => {
                                                        dl_log!(level::ERROR, msg_ctx, "Failed to serialize NackReply: {}", ser_err);
                                                        (NACK_REPLY, Vec::new())
                                                    }
                                                }
                                            } else {
                                                // Create directory with -p equivalent (create parent directories)
                                                match std::fs::create_dir_all(&sanitized.local_path) {
                                                    Ok(()) => {
                                                        println!("Created directory: {}", sanitized.local_path.display());
                                                        let ack = AckReply;
                                                        match ack.serialize() {
                                                            Ok(payload) => (ACK_REPLY, payload),
                                                            Err(e) => {
                                                                dl_log!(level::ERROR, msg_ctx, "Failed to serialize AckReply: {}", e);
                                                                (ACK_REPLY, Vec::new())
                                                            }
                                                        }
                                                    }
                                                    Err(e) => {
                                                        let nack = NackReply {
                                                            err_code: ErrorCode::DirectoryCreationError as i32,
                                                            err_msg: format!("Failed to create directory {}: {}", sanitized.local_path.display(), e),
                                                        };
                                                        match nack.serialize() {
                                                            Ok(payload) => (NACK_REPLY, payload),
                                                            Err(ser_err) => {
                                                                dl_log!(level::ERROR, msg_ctx, "Failed to serialize NackReply: {}", ser_err);
                                                                (NACK_REPLY, Vec::new())
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            let nack = NackReply {
                                                err_code: ErrorCode::SecurityViolation as i32,
                                                err_msg: format!("Path sanitization failed: {}", e),
                                            };
                                            match nack.serialize() {
                                                Ok(payload) => (NACK_REPLY, payload),
                                                Err(ser_err) => {
                                                    dl_log!(level::ERROR, msg_ctx, "Failed to serialize NackReply: {}", ser_err);
                                                    (NACK_REPLY, Vec::new())
                                                }
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    let nack = NackReply {
                                        err_code: ErrorCode::ParseError as i32,
                                        err_msg: format!("Failed to parse path create request: {}", e),
                                    };
                                    match nack.serialize() {
                                        Ok(payload) => (NACK_REPLY, payload),
                                        Err(ser_err) => {
                                            dl_log!(level::ERROR, msg_ctx, "Failed to serialize NackReply: {}", ser_err);
                                            (NACK_REPLY, Vec::new())
                                        }
                                    }
                                }
                            }
                        }
                        REPO_PATH_DELETE_REQUEST => {
                            // Parse payload to get path and recursively delete directory under base_root
                            match parse_path_request(&env.payload) {
                                Ok(path) => {
                                    // Use path sanitizer for security
                                    match path_sanitizer.sanitize_path(&path) {
                                        Ok(sanitized) => {
                                            // Additional security validation
                                            if let Err(e) = path_sanitizer.validate_path_security(&sanitized.local_path) {
                                                let nack = NackReply {
                                                    err_code: ErrorCode::SecurityViolation as i32,
                                                    err_msg: format!("Security violation: {}", e),
                                                };
                                                match nack.serialize() {
                                                    Ok(payload) => (NACK_REPLY, payload),
                                                    Err(ser_err) => {
                                                        dl_log!(level::ERROR, msg_ctx, "Failed to serialize NackReply: {}", ser_err);
                                                        (NACK_REPLY, Vec::new())
                                                    }
                                                }
                                            } else if sanitized.local_path == base_root {
                                                // Additional security check: prevent deletion of base_root itself
                                                let nack = NackReply {
                                                    err_code: ErrorCode::BaseRootDeletionError as i32,
                                                    err_msg: "Cannot delete base root directory".to_string(),
                                                };
                                                match nack.serialize() {
                                                    Ok(payload) => (NACK_REPLY, payload),
                                                    Err(ser_err) => {
                                                        dl_log!(level::ERROR, msg_ctx, "Failed to serialize NackReply: {}", ser_err);
                                                        (NACK_REPLY, Vec::new())
                                                    }
                                                }
                                            } else {
                                                // Recursively delete directory (rm -r equivalent)
                                                match std::fs::remove_dir_all(&sanitized.local_path) {
                                                    Ok(()) => {
                                                        println!("Deleted directory: {}", sanitized.local_path.display());
                                                        let ack = AckReply;
                                                        match ack.serialize() {
                                                            Ok(payload) => (ACK_REPLY, payload),
                                                            Err(e) => {
                                                                dl_log!(level::ERROR, msg_ctx, "Failed to serialize AckReply: {}", e);
                                                                (ACK_REPLY, Vec::new())
                                                            }
                                                        }
                                                    }
                                                    Err(e) => {
                                                        let nack = NackReply {
                                                            err_code: ErrorCode::DirectoryDeletionError as i32,
                                                            err_msg: format!("Failed to delete directory {}: {}", sanitized.local_path.display(), e),
                                                        };
                                                        match nack.serialize() {
                                                            Ok(payload) => (NACK_REPLY, payload),
                                                            Err(ser_err) => {
                                                                dl_log!(level::ERROR, msg_ctx, "Failed to serialize NackReply: {}", ser_err);
                                                                (NACK_REPLY, Vec::new())
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            let nack = NackReply {
                                                err_code: ErrorCode::SecurityViolation as i32,
                                                err_msg: format!("Path sanitization failed: {}", e),
                                            };
                                            match nack.serialize() {
                                                Ok(payload) => (NACK_REPLY, payload),
                                                Err(ser_err) => {
                                                    dl_log!(level::ERROR, msg_ctx, "Failed to serialize NackReply: {}", ser_err);
                                                    (NACK_REPLY, Vec::new())
                                                }
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    let nack = NackReply {
                                        err_code: ErrorCode::ParseError as i32,
                                        err_msg: format!("Failed to parse path delete request: {}", e),
                                    };
                                    match nack.serialize() {
                                        Ok(payload) => (NACK_REPLY, payload),
                                        Err(ser_err) => {
                                            dl_log!(level::ERROR, msg_ctx, "Failed to serialize NackReply: {}", ser_err);
                                            (NACK_REPLY, Vec::new())
                                        }
                                    }
                                }
                            }
                        }
                        _ => {
                            let nack = NackReply {
                                err_code: ErrorCode::BadRequest as i32,
                                err_msg: format!("Unsupported message type: {}", env.msg_type),
                            };
                            match nack.serialize() {
                                Ok(payload) => (NACK_REPLY, payload),
                                Err(e) => {
                                    dl_log!(level::ERROR, msg_ctx, "Failed to serialize NackReply: {}", e);
                                    (NACK_REPLY, Vec::new())
                                }
                            }
                        }
                    };

                    // Send reply (preserve correlation id)
                    println!("🟡 Worker[{}] sending reply type: {} (0x{:x}), corr_id: {}, payload_len: {}", 
                             id, reply_type, reply_type, correlation_id, payload.len());
                    
                    // Debug: print first few bytes of reply payload
                    let debug_len = std::cmp::min(20, payload.len());
                    let debug_bytes = &payload[..debug_len];
                    println!("🟡 Reply payload preview (first {} bytes): {:?}", debug_len, debug_bytes);
                    
                    let result = messenger.send(Envelope {
                        correlation_id,
                        msg_type: reply_type,
                        payload,
                        key: None,
                    });
                    
                    match result {
                        Ok(()) => println!("✅ Worker[{}] successfully sent reply", id),
                        Err(e) => eprintln!("❌ Worker[{}] failed to send reply: {}", id, e),
                    }
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

// Old encode_nack function removed - now using NackReply struct

/// Parse delete request payload to extract file paths
/// Expected format: JSON array of strings representing relative paths
/// Example: ["file1.txt", "subdir/file2.txt"]
fn parse_delete_request(payload: &[u8]) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    if payload.is_empty() {
        return Err("Empty payload".into());
    }
    
    // Debug: print first few bytes of payload
    let debug_len = std::cmp::min(20, payload.len());
    let debug_bytes = &payload[..debug_len];
    println!("parse_delete_request: payload {:?} first {} bytes: {:?}", payload, debug_len, debug_bytes);
    
    // Parse as protobuf RepoDataDeleteRequest
    let delete_request = crate::proto::RepoDataDeleteRequest::deserialize(payload)?;
    
    // Extract paths from the locations
    let paths: Vec<String> = delete_request.loc.into_iter()
        .map(|location| location.path)
        .collect();
    
    Ok(paths)
}

/// Parse size request payload to extract file paths
/// Expected format: JSON array of strings representing relative paths
/// Example: ["file1.txt", "subdir/file2.txt"]
fn parse_size_request(payload: &[u8]) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    if payload.is_empty() {
        return Err("Empty payload".into());
    }
    
    // Debug: print first few bytes of payload
    let debug_len = std::cmp::min(20, payload.len());
    let debug_bytes = &payload[..debug_len];
    println!("parse_size_request: payload first {} bytes: {:?}", debug_len, debug_bytes);
    
    // Parse as protobuf RepoDataGetSizeRequest
    let size_request = crate::proto::RepoDataGetSizeRequest::deserialize(payload)?;
    
    // Extract paths from the locations
    let paths: Vec<String> = size_request.loc.into_iter()
        .map(|location| location.path)
        .collect();
    
    Ok(paths)
}

/// Parse path request payload to extract a single path
/// Parse the protobuf field data to extract the path
fn parse_path_request(payload: &[u8]) -> Result<String, Box<dyn std::error::Error>> {
    if payload.is_empty() {
        return Err("Empty payload".into());
    }
    
    // Debug: print first few bytes of payload
    let debug_len = std::cmp::min(20, payload.len());
    let debug_bytes = &payload[..debug_len];
    println!("parse_path_request: payload {:?} first {} bytes: {:?}", payload, debug_len, debug_bytes);
    
    // The payload is a raw protobuf string field, not a complete message
    // Format: [field_tag, length, string_bytes...]
    if payload.len() >= 2 && payload[0] == 0x0A { // Field 1, wire type 2 (string)
        let length = payload[1] as usize;
        if payload.len() >= 2 + length {
            let path_bytes = &payload[2..2 + length];
            if let Ok(path_str) = std::str::from_utf8(path_bytes) {
                println!("parse_path_request: extracted path from protobuf field: '{}'", path_str);
                return Ok(path_str.to_string());
            }
        }
    }
    
    // Fallback: try to parse as complete protobuf message
    match crate::proto::RepoPathCreateRequest::deserialize(payload) {
        Ok(request) => {
            let path = request.path;
            println!("parse_path_request: extracted path from complete protobuf: '{}'", path);
            Ok(path)
        }
        Err(e) => {
            println!("parse_path_request: failed to parse protobuf: {}", e);
            Err(format!("Failed to parse RepoPathCreateRequest: {}", e).into())
        }
    }
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
