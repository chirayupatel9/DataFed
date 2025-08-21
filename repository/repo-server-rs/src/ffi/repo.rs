#[cxx::bridge]
mod ffi {
    struct VersionInfo {
        release_year: u32, release_month: u32, release_day: u32,
        release_hour: u32, release_minute: u32,
        api_major: u32, api_minor: u32, api_patch: u32,
        component_major: u32, component_minor: u32, component_patch: u32,
    }

    unsafe extern "C++" {
        include!("repo_bridge.hpp");

        #[namespace = "RepoBridge"]
        fn send_version_request(
            host: &str,
            port: u16,
            scheme: &str,
            core_public_key: &str,
            timeout_ms: u32
        ) -> Result<VersionInfo>;
    }
}
pub use ffi::*;
