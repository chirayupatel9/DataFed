// Generated protobuf message definitions
// This module contains the Rust equivalents of the C++ protobuf messages

// Check if protobuf files were generated
#[cfg(any())] // Disabled for now - will enable when protobuf files are available
mod generated {
    pub mod sdms {
        include!(concat!(env!("OUT_DIR"), "/sdms.rs"));
    }

    pub mod sdms_anon {
        include!(concat!(env!("OUT_DIR"), "/sdms_anon.rs"));
    }

    pub mod sdms_auth {
        include!(concat!(env!("OUT_DIR"), "/sdms_auth.rs"));
    }

    pub mod version {
        include!(concat!(env!("OUT_DIR"), "/version.rs"));
    }
}

// Fallback definitions when protobuf files don't exist
mod fallback {
    // Placeholder structures for when protobuf is not available
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct VersionRequest;

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct RepoDataDeleteRequest {
        pub loc: Vec<RecordDataLocation>,
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct RepoDataGetSizeRequest {
        pub loc: Vec<RecordDataLocation>,
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct RepoPathCreateRequest {
        pub path: String,
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct RepoPathDeleteRequest {
        pub path: String,
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct RecordDataLocation {
        pub id: String,
        pub path: String,
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct RecordDataSize {
        pub id: String,
        pub size: u64,
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct RepoDataSizeReply {
        pub size: Vec<RecordDataSize>,
    }

    // Add serialization/deserialization methods for all message types
    impl VersionRequest {
        pub fn serialize(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
            Ok(Vec::new()) // Empty for now
        }

        pub fn deserialize(_data: &[u8]) -> Result<VersionRequest, Box<dyn std::error::Error>> {
            Ok(VersionRequest)
        }
    }

    impl VersionReply {
        pub fn new() -> Self {
            Self {
                release_year: 2025,
                release_month: 6,
                release_day: 11,
                release_hour: 14,
                release_minute: 1,
                api_major: 1,
                api_minor: 1,
                api_patch: 0,
                component_major: 1,
                component_minor: 0,
                component_patch: 0,
            }
        }

        pub fn serialize(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
            let mut data = Vec::new();
            data.extend_from_slice(&self.release_year.to_le_bytes());
            data.extend_from_slice(&self.release_month.to_le_bytes());
            data.extend_from_slice(&self.release_day.to_le_bytes());
            data.extend_from_slice(&self.release_hour.to_le_bytes());
            data.extend_from_slice(&self.release_minute.to_le_bytes());
            data.extend_from_slice(&self.api_major.to_le_bytes());
            data.extend_from_slice(&self.api_minor.to_le_bytes());
            data.extend_from_slice(&self.api_patch.to_le_bytes());
            data.extend_from_slice(&self.component_major.to_le_bytes());
            data.extend_from_slice(&self.component_minor.to_le_bytes());
            data.extend_from_slice(&self.component_patch.to_le_bytes());
            Ok(data)
        }

        pub fn deserialize(data: &[u8]) -> Result<VersionReply, Box<dyn std::error::Error>> {
            if data.len() < 44 {
                return Err("VersionReply data too short".into());
            }
            
            let mut offset = 0;
            let release_year = u32::from_le_bytes([data[offset], data[offset+1], data[offset+2], data[offset+3]]);
            offset += 4;
            let release_month = u32::from_le_bytes([data[offset], data[offset+1], data[offset+2], data[offset+3]]);
            offset += 4;
            let release_day = u32::from_le_bytes([data[offset], data[offset+1], data[offset+2], data[offset+3]]);
            offset += 4;
            let release_hour = u32::from_le_bytes([data[offset], data[offset+1], data[offset+2], data[offset+3]]);
            offset += 4;
            let release_minute = u32::from_le_bytes([data[offset], data[offset+1], data[offset+2], data[offset+3]]);
            offset += 4;
            let api_major = u32::from_le_bytes([data[offset], data[offset+1], data[offset+2], data[offset+3]]);
            offset += 4;
            let api_minor = u32::from_le_bytes([data[offset], data[offset+1], data[offset+2], data[offset+3]]);
            offset += 4;
            let api_patch = u32::from_le_bytes([data[offset], data[offset+1], data[offset+2], data[offset+3]]);
            offset += 4;
            let component_major = u32::from_le_bytes([data[offset], data[offset+1], data[offset+2], data[offset+3]]);
            offset += 4;
            let component_minor = u32::from_le_bytes([data[offset], data[offset+1], data[offset+2], data[offset+3]]);
            offset += 4;
            let component_patch = u32::from_le_bytes([data[offset], data[offset+1], data[offset+2], data[offset+3]]);
            
            Ok(VersionReply {
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
            })
        }
    }

    impl RepoDataDeleteRequest {
        pub fn serialize(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
            let mut data = Vec::new();
            data.extend_from_slice(&(self.loc.len() as u32).to_le_bytes());
            for location in &self.loc {
                data.extend_from_slice(&(location.id.len() as u32).to_le_bytes());
                data.extend_from_slice(location.id.as_bytes());
                data.extend_from_slice(&(location.path.len() as u32).to_le_bytes());
                data.extend_from_slice(location.path.as_bytes());
            }
            Ok(data)
        }

        pub fn deserialize(data: &[u8]) -> Result<RepoDataDeleteRequest, Box<dyn std::error::Error>> {
            // Parse protobuf format for RepoDataDeleteRequest
            // Field 1: repeated RecordDataLocation loc
            let mut loc = Vec::new();
            let mut offset = 0;
            
            while offset < data.len() {
                if offset >= data.len() {
                    break;
                }
                
                // Read field tag and wire type
                let tag_byte = data[offset];
                offset += 1;
                
                if tag_byte == 0x0A { // Field 1, wire type 2 (length-delimited)
                    // Read length
                    let mut length = 0u32;
                    let mut shift = 0;
                    loop {
                        if offset >= data.len() {
                            return Err("Invalid protobuf data".into());
                        }
                        let byte = data[offset];
                        offset += 1;
                        length |= ((byte & 0x7F) as u32) << shift;
                        if (byte & 0x80) == 0 {
                            break;
                        }
                        shift += 7;
                    }
                    
                    // Parse RecordDataLocation
                    if offset + length as usize > data.len() {
                        return Err("Invalid protobuf data".into());
                    }
                    
                    let location_data = &data[offset..offset + length as usize];
                    offset += length as usize;
                    
                    // Parse RecordDataLocation fields
                    let mut id = String::new();
                    let mut path = String::new();
                    let mut loc_offset = 0;
                    
                    while loc_offset < location_data.len() {
                        if loc_offset >= location_data.len() {
                            break;
                        }
                        
                        let loc_tag_byte = location_data[loc_offset];
                        loc_offset += 1;
                        
                        if loc_tag_byte == 0x0A { // Field 1: id
                            let mut id_len = 0u32;
                            let mut id_shift = 0;
                            loop {
                                if loc_offset >= location_data.len() {
                                    return Err("Invalid protobuf data".into());
                                }
                                let byte = location_data[loc_offset];
                                loc_offset += 1;
                                id_len |= ((byte & 0x7F) as u32) << id_shift;
                                if (byte & 0x80) == 0 {
                                    break;
                                }
                                id_shift += 7;
                            }
                            
                            if loc_offset + id_len as usize > location_data.len() {
                                return Err("Invalid protobuf data".into());
                            }
                            id = String::from_utf8(location_data[loc_offset..loc_offset + id_len as usize].to_vec())?;
                            loc_offset += id_len as usize;
                        } else if loc_tag_byte == 0x12 { // Field 2: path
                            let mut path_len = 0u32;
                            let mut path_shift = 0;
                            loop {
                                if loc_offset >= location_data.len() {
                                    return Err("Invalid protobuf data".into());
                                }
                                let byte = location_data[loc_offset];
                                loc_offset += 1;
                                path_len |= ((byte & 0x7F) as u32) << path_shift;
                                if (byte & 0x80) == 0 {
                                    break;
                                }
                                path_shift += 7;
                            }
                            
                            if loc_offset + path_len as usize > location_data.len() {
                                return Err("Invalid protobuf data".into());
                            }
                            path = String::from_utf8(location_data[loc_offset..loc_offset + path_len as usize].to_vec())?;
                            loc_offset += path_len as usize;
                        } else {
                            // Skip unknown field
                            let mut skip_len = 0u32;
                            let mut skip_shift = 0;
                            loop {
                                if loc_offset >= location_data.len() {
                                    return Err("Invalid protobuf data".into());
                                }
                                let byte = location_data[loc_offset];
                                loc_offset += 1;
                                skip_len |= ((byte & 0x7F) as u32) << skip_shift;
                                if (byte & 0x80) == 0 {
                                    break;
                                }
                                skip_shift += 7;
                            }
                            loc_offset += skip_len as usize;
                        }
                    }
                    
                    loc.push(RecordDataLocation { id, path });
                } else {
                    // Skip unknown field
                    let mut skip_len = 0u32;
                    let mut skip_shift = 0;
                    loop {
                        if offset >= data.len() {
                            return Err("Invalid protobuf data".into());
                        }
                        let byte = data[offset];
                        offset += 1;
                        skip_len |= ((byte & 0x7F) as u32) << skip_shift;
                        if (byte & 0x80) == 0 {
                            break;
                        }
                        skip_shift += 7;
                    }
                    offset += skip_len as usize;
                }
            }
            
            Ok(RepoDataDeleteRequest { loc })
        }
    }

    impl RepoDataGetSizeRequest {
        pub fn serialize(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
            let mut data = Vec::new();
            data.extend_from_slice(&(self.loc.len() as u32).to_le_bytes());
            for location in &self.loc {
                data.extend_from_slice(&(location.id.len() as u32).to_le_bytes());
                data.extend_from_slice(location.id.as_bytes());
                data.extend_from_slice(&(location.path.len() as u32).to_le_bytes());
                data.extend_from_slice(location.path.as_bytes());
            }
            Ok(data)
        }

        pub fn deserialize(data: &[u8]) -> Result<RepoDataGetSizeRequest, Box<dyn std::error::Error>> {
            // Parse protobuf format for RepoDataGetSizeRequest
            // Field 1: repeated RecordDataLocation loc
            let mut loc = Vec::new();
            let mut offset = 0;
            
            while offset < data.len() {
                if offset >= data.len() {
                    break;
                }
                
                // Read field tag and wire type
                let tag_byte = data[offset];
                offset += 1;
                
                if tag_byte == 0x0A { // Field 1, wire type 2 (length-delimited)
                    // Read length
                    let mut length = 0u32;
                    let mut shift = 0;
                    loop {
                        if offset >= data.len() {
                            return Err("Invalid protobuf data".into());
                        }
                        let byte = data[offset];
                        offset += 1;
                        length |= ((byte & 0x7F) as u32) << shift;
                        if (byte & 0x80) == 0 {
                            break;
                        }
                        shift += 7;
                    }
                    
                    // Parse RecordDataLocation
                    if offset + length as usize > data.len() {
                        return Err("Invalid protobuf data".into());
                    }
                    
                    let location_data = &data[offset..offset + length as usize];
                    offset += length as usize;
                    
                    // Parse RecordDataLocation fields
                    let mut id = String::new();
                    let mut path = String::new();
                    let mut loc_offset = 0;
                    
                    while loc_offset < location_data.len() {
                        if loc_offset >= location_data.len() {
                            break;
                        }
                        
                        let loc_tag_byte = location_data[loc_offset];
                        loc_offset += 1;
                        
                        if loc_tag_byte == 0x0A { // Field 1: id
                            let mut id_len = 0u32;
                            let mut id_shift = 0;
                            loop {
                                if loc_offset >= location_data.len() {
                                    return Err("Invalid protobuf data".into());
                                }
                                let byte = location_data[loc_offset];
                                loc_offset += 1;
                                id_len |= ((byte & 0x7F) as u32) << id_shift;
                                if (byte & 0x80) == 0 {
                                    break;
                                }
                                id_shift += 7;
                            }
                            
                            if loc_offset + id_len as usize > location_data.len() {
                                return Err("Invalid protobuf data".into());
                            }
                            id = String::from_utf8(location_data[loc_offset..loc_offset + id_len as usize].to_vec())?;
                            loc_offset += id_len as usize;
                        } else if loc_tag_byte == 0x12 { // Field 2: path
                            let mut path_len = 0u32;
                            let mut path_shift = 0;
                            loop {
                                if loc_offset >= location_data.len() {
                                    return Err("Invalid protobuf data".into());
                                }
                                let byte = location_data[loc_offset];
                                loc_offset += 1;
                                path_len |= ((byte & 0x7F) as u32) << path_shift;
                                if (byte & 0x80) == 0 {
                                    break;
                                }
                                path_shift += 7;
                            }
                            
                            if loc_offset + path_len as usize > location_data.len() {
                                return Err("Invalid protobuf data".into());
                            }
                            path = String::from_utf8(location_data[loc_offset..loc_offset + path_len as usize].to_vec())?;
                            loc_offset += path_len as usize;
                        } else {
                            // Skip unknown field
                            let mut skip_len = 0u32;
                            let mut skip_shift = 0;
                            loop {
                                if loc_offset >= location_data.len() {
                                    return Err("Invalid protobuf data".into());
                                }
                                let byte = location_data[loc_offset];
                                loc_offset += 1;
                                skip_len |= ((byte & 0x7F) as u32) << skip_shift;
                                if (byte & 0x80) == 0 {
                                    break;
                                }
                                skip_shift += 7;
                            }
                            loc_offset += skip_len as usize;
                        }
                    }
                    
                    loc.push(RecordDataLocation { id, path });
                } else {
                    // Skip unknown field
                    let mut skip_len = 0u32;
                    let mut skip_shift = 0;
                    loop {
                        if offset >= data.len() {
                            return Err("Invalid protobuf data".into());
                        }
                        let byte = data[offset];
                        offset += 1;
                        skip_len |= ((byte & 0x7F) as u32) << skip_shift;
                        if (byte & 0x80) == 0 {
                            break;
                        }
                        skip_shift += 7;
                    }
                    offset += skip_len as usize;
                }
            }
            
            Ok(RepoDataGetSizeRequest { loc })
        }
    }

    impl RepoPathCreateRequest {
        pub fn serialize(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
            let mut data = Vec::new();
            data.extend_from_slice(&(self.path.len() as u32).to_le_bytes());
            data.extend_from_slice(self.path.as_bytes());
            Ok(data)
        }

        pub fn deserialize(data: &[u8]) -> Result<RepoPathCreateRequest, Box<dyn std::error::Error>> {
            if data.len() < 4 {
                return Err("RepoPathCreateRequest data too short".into());
            }
            
            let path_len = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
            if 4 + path_len > data.len() {
                return Err("Invalid RepoPathCreateRequest data".into());
            }
            
            let path = String::from_utf8(data[4..4+path_len].to_vec())?;
            Ok(RepoPathCreateRequest { path })
        }
    }

    impl RepoPathDeleteRequest {
        pub fn serialize(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
            let mut data = Vec::new();
            data.extend_from_slice(&(self.path.len() as u32).to_le_bytes());
            data.extend_from_slice(self.path.as_bytes());
            Ok(data)
        }

        pub fn deserialize(data: &[u8]) -> Result<RepoPathDeleteRequest, Box<dyn std::error::Error>> {
            if data.len() < 4 {
                return Err("RepoPathDeleteRequest data too short".into());
            }
            
            let path_len = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
            if 4 + path_len > data.len() {
                return Err("Invalid RepoPathDeleteRequest data".into());
            }
            
            let path = String::from_utf8(data[4..4+path_len].to_vec())?;
            Ok(RepoPathDeleteRequest { path })
        }
    }

    impl RepoDataSizeReply {
        pub fn serialize(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
            let mut data = Vec::new();
            data.extend_from_slice(&(self.size.len() as u32).to_le_bytes());
            for size_info in &self.size {
                data.extend_from_slice(&(size_info.id.len() as u32).to_le_bytes());
                data.extend_from_slice(size_info.id.as_bytes());
                data.extend_from_slice(&size_info.size.to_le_bytes());
            }
            Ok(data)
        }

        pub fn deserialize(data: &[u8]) -> Result<RepoDataSizeReply, Box<dyn std::error::Error>> {
            if data.len() < 4 {
                return Err("RepoDataSizeReply data too short".into());
            }
            
            let mut offset = 0;
            let count = u32::from_le_bytes([data[offset], data[offset+1], data[offset+2], data[offset+3]]) as usize;
            offset += 4;
            
            let mut size = Vec::new();
            for _ in 0..count {
                if offset + 4 > data.len() {
                    return Err("Invalid RepoDataSizeReply data".into());
                }
                let id_len = u32::from_le_bytes([data[offset], data[offset+1], data[offset+2], data[offset+3]]) as usize;
                offset += 4;
                
                if offset + id_len > data.len() {
                    return Err("Invalid RepoDataSizeReply data".into());
                }
                let id = String::from_utf8(data[offset..offset+id_len].to_vec())?;
                offset += id_len;
                
                if offset + 8 > data.len() {
                    return Err("Invalid RepoDataSizeReply data".into());
                }
                let size_val = u64::from_le_bytes([
                    data[offset], data[offset+1], data[offset+2], data[offset+3],
                    data[offset+4], data[offset+5], data[offset+6], data[offset+7]
                ]);
                offset += 8;
                
                size.push(RecordDataSize { id, size: size_val });
            }
            
            Ok(RepoDataSizeReply { size })
        }
    }
}

// Re-export based on feature flags
// #[cfg(feature = "protobuf")]
// pub use generated::*;

// Use fallback definitions for now
pub use fallback::*;
