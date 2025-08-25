// src/version.rs
fn parse_env_u32(key: &str, default: u32) -> u32 {
    std::env::var(key).ok().and_then(|s| s.parse().ok()).unwrap_or(default)
}

pub fn repo_major() -> u32 { parse_env_u32("REPO_MAJOR", 0) }
pub fn repo_minor() -> u32 { parse_env_u32("REPO_MINOR", 0) }
pub fn repo_patch() -> u32 { parse_env_u32("REPO_PATCH", 0) }
pub fn api_major()  -> u32 { parse_env_u32("API_MAJOR",  0) }
pub fn api_minor()  -> u32 { parse_env_u32("API_MINOR",  0) }
