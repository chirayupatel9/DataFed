// src/version.rs

// Version constants matching cmake/Version.cmake
pub const DATAFED_RELEASE_YEAR: u32 = 2025;
pub const DATAFED_RELEASE_MONTH: u32 = 6;
pub const DATAFED_RELEASE_DAY: u32 = 11;
pub const DATAFED_RELEASE_HOUR: u32 = 14;
pub const DATAFED_RELEASE_MINUTE: u32 = 1;

pub const DATAFED_COMMON_PROTOCOL_API_MAJOR: u32 = 1;
pub const DATAFED_COMMON_PROTOCOL_API_MINOR: u32 = 1;
pub const DATAFED_COMMON_PROTOCOL_API_PATCH: u32 = 0;

pub const DATAFED_REPO_MAJOR: u32 = 1;
pub const DATAFED_REPO_MINOR: u32 = 0;
pub const DATAFED_REPO_PATCH: u32 = 0;

fn parse_env_u32(key: &str, default: u32) -> u32 {
    std::env::var(key).ok().and_then(|s| s.parse().ok()).unwrap_or(default)
}

pub fn repo_major() -> u32 { parse_env_u32("REPO_MAJOR", DATAFED_REPO_MAJOR) }
pub fn repo_minor() -> u32 { parse_env_u32("REPO_MINOR", DATAFED_REPO_MINOR) }
pub fn repo_patch() -> u32 { parse_env_u32("REPO_PATCH", DATAFED_REPO_PATCH) }
pub fn api_major()  -> u32 { parse_env_u32("API_MAJOR", DATAFED_COMMON_PROTOCOL_API_MAJOR) }
pub fn api_minor()  -> u32 { parse_env_u32("API_MINOR", DATAFED_COMMON_PROTOCOL_API_MINOR) }

// ===== VersionReply structure =====
#[derive(Debug, Clone)]
pub struct VersionReply {
    pub release_year: u32,
    pub release_month: u32,
    pub release_day: u32,
    pub release_hour: u32,
    pub release_minute: u32,
    pub api_major: u32,
    pub api_minor: u32,
    pub api_patch: u32,
    pub component_major: u32,
    pub component_minor: u32,
    pub component_patch: u32,
}

impl VersionReply {
    pub fn new() -> Self {
        Self {
            release_year: DATAFED_RELEASE_YEAR,
            release_month: DATAFED_RELEASE_MONTH,
            release_day: DATAFED_RELEASE_DAY,
            release_hour: DATAFED_RELEASE_HOUR,
            release_minute: DATAFED_RELEASE_MINUTE,
            api_major: DATAFED_COMMON_PROTOCOL_API_MAJOR,
            api_minor: DATAFED_COMMON_PROTOCOL_API_MINOR,
            api_patch: DATAFED_COMMON_PROTOCOL_API_PATCH,
            component_major: DATAFED_REPO_MAJOR,
            component_minor: DATAFED_REPO_MINOR,
            component_patch: DATAFED_REPO_PATCH,
        }
    }

    pub fn from_core_server(
        release_year: u32,
        release_month: u32,
        release_day: u32,
        release_hour: u32,
        release_minute: u32,
        api_major: u32,
        api_minor: u32,
        api_patch: u32,
        component_major: u32,
        component_minor: u32,
        component_patch: u32,
    ) -> Self {
        Self {
            release_year,
            release_month,
            release_day,
            release_hour,
            release_minute,
            api_major,
            api_minor,
            api_patch,
            component_major,
            component_minor,
            component_patch,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        // Simple binary serialization format for now
        // In a real implementation, you'd use prost/protobuf
        let mut bytes = Vec::new();
        
        // Add each field as little-endian u32
        bytes.extend_from_slice(&self.release_year.to_le_bytes());
        bytes.extend_from_slice(&self.release_month.to_le_bytes());
        bytes.extend_from_slice(&self.release_day.to_le_bytes());
        bytes.extend_from_slice(&self.release_hour.to_le_bytes());
        bytes.extend_from_slice(&self.release_minute.to_le_bytes());
        bytes.extend_from_slice(&self.api_major.to_le_bytes());
        bytes.extend_from_slice(&self.api_minor.to_le_bytes());
        bytes.extend_from_slice(&self.api_patch.to_le_bytes());
        bytes.extend_from_slice(&self.component_major.to_le_bytes());
        bytes.extend_from_slice(&self.component_minor.to_le_bytes());
        bytes.extend_from_slice(&self.component_patch.to_le_bytes());
        
        bytes
    }
}

use crate::config::Config;
use crate::proto::VersionReply as ProtoVersionReply;

// ===== Core Server Version Management =====
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone)]
pub struct CoreVersionInfo {
    pub version_reply: ProtoVersionReply,
    pub is_connected: bool,
}

impl CoreVersionInfo {
    pub fn new() -> Self {
        Self {
            version_reply: ProtoVersionReply::new(),
            is_connected: false,
        }
    }

    pub fn update_from_core(&mut self, version_reply: ProtoVersionReply) {
        self.version_reply = version_reply;
        self.is_connected = true;
    }

    pub fn mark_disconnected(&mut self) {
        self.is_connected = false;
    }
}

pub type SharedCoreVersionInfo = Arc<RwLock<CoreVersionInfo>>;

pub fn create_shared_version_info() -> SharedCoreVersionInfo {
    Arc::new(RwLock::new(CoreVersionInfo::new()))
}

/// Attempts to connect to the core server and fetch version information
/// This mimics the C++ checkServerVersion() behavior
pub fn fetch_core_server_version(
    core_server_address: &str,
    version_info: SharedCoreVersionInfo,
    config: &Config,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Checking core server connection and version at {}", core_server_address);
    
    // Parse the core server address (e.g., "tcp://localhost:9998")
    let (host, port, scheme) = parse_core_server_address(core_server_address)?;
    
    // Use the existing C++ bridge that handles secure ZMQ connection and authentication
    // This is the same approach used by the C++ version
    let core_public_key = config.load_core_public_key()
        .map_err(|e| format!("Failed to load core public key: {}", e))?;
    
    match crate::ffi::repo::send_version_request(host, port, scheme, &core_public_key, 20000) {
        Ok(version_info_cpp) => {
            // Convert the C++ VersionInfo to our ProtoVersionReply
            let core_version = ProtoVersionReply {
                release_year: version_info_cpp.release_year,
                release_month: version_info_cpp.release_month,
                release_day: version_info_cpp.release_day,
                release_hour: version_info_cpp.release_hour,
                release_minute: version_info_cpp.release_minute,
                api_major: version_info_cpp.api_major,
                api_minor: version_info_cpp.api_minor,
                api_patch: version_info_cpp.api_patch,
                component_major: version_info_cpp.component_major,
                component_minor: version_info_cpp.component_minor,
                component_patch: version_info_cpp.component_patch,
            };
            
            // Validate API compatibility
            if core_version.api_major != DATAFED_COMMON_PROTOCOL_API_MAJOR {
                return Err(format!(
                    "Incompatible messaging API detected: major backwards breaking changes detected version ({}.{}.{})",
                    core_version.api_major, core_version.api_minor, core_version.api_patch
                ).into());
            }
            
            if core_version.api_minor + 9 > DATAFED_COMMON_PROTOCOL_API_MINOR {
                println!("Warning: Significant changes in message API detected ({}.{}.{})", 
                         core_version.api_major, core_version.api_minor, core_version.api_patch);
            }
            
            // Check for newer releases
            let mut new_release_available = false;
            if core_version.release_year > DATAFED_RELEASE_YEAR {
                new_release_available = true;
            } else if core_version.release_year == DATAFED_RELEASE_YEAR {
                if core_version.release_month > DATAFED_RELEASE_MONTH {
                    new_release_available = true;
                } else if core_version.release_month == DATAFED_RELEASE_MONTH {
                    if core_version.release_day > DATAFED_RELEASE_DAY {
                        new_release_available = true;
                    } else if core_version.release_day == DATAFED_RELEASE_DAY {
                        if core_version.release_hour > DATAFED_RELEASE_HOUR {
                            new_release_available = true;
                        } else if core_version.release_hour == DATAFED_RELEASE_HOUR {
                            if core_version.release_minute > DATAFED_RELEASE_MINUTE {
                                new_release_available = true;
                            }
                        }
                    }
                }
            }
            
            if new_release_available {
                println!("Newer releases for the repo server may be available.");
            }
            
            // Update the shared version info
            {
                let mut info = version_info.write().unwrap();
                info.update_from_core(core_version);
            }
            
            println!("Core server connection OK.");
            Ok(())
        }
        Err(e) => {
            Err(format!("Failed to connect to core server via C++ bridge: {}", e).into())
        }
    }
}

/// Parse core server address in format "tcp://host:port"
fn parse_core_server_address(address: &str) -> Result<(&str, u16, &str), Box<dyn std::error::Error>> {
    let parts: Vec<&str> = address.split("://").collect();
    if parts.len() != 2 {
        return Err("Invalid core server address format. Expected 'tcp://host:port'".into());
    }
    
    let scheme = parts[0];
    if scheme != "tcp" {
        return Err("Only TCP protocol is supported".into());
    }
    
    let host_port = parts[1];
    let colon_pos = host_port.rfind(':').ok_or("No port specified")?;
    let host = &host_port[..colon_pos];
    let port = host_port[colon_pos + 1..].parse::<u16>()?;
    
    Ok((host, port, scheme))
}
