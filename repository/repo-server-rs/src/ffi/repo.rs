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

        #[namespace = "ServerBridge"]
        fn server_start(
            config_path: &str,
            repo_public_key: &str,
            repo_private_key: &str,
            port: u16
        ) -> Result<()>;

        #[namespace = "ServerBridge"]
        fn server_stop() -> Result<()>;

        #[namespace = "ServerBridge"]
        fn server_join() -> Result<()>;

        #[namespace = "ZMQBridge"]
        fn zmq_recv(timeout_ms: i32) -> Result<Vec<u8>>;

        #[namespace = "ZMQBridge"]
        fn zmq_send(payload: &[u8], msg_type: u16, correlation_id: &str) -> Result<()>;

        #[namespace = "ZMQBridge"]
        fn zmq_send_external(payload: &[u8], msg_type: u16, correlation_id: &str) -> Result<()>;
        
        #[namespace = "ZMQBridge"]
        fn get_last_correlation_id() -> String;
        
        #[namespace = "ZMQBridge"]
        fn get_last_context() -> u16;
        
        #[namespace = "ZMQBridge"]
        fn create_response_envelope(request_payload: &[u8], request_msg_type: u16, request_correlation_id: &str, request_route: &str) -> Result<Vec<u8>>;
        
        #[namespace = "ZMQBridge"]
        fn create_response_envelope_from_request(original_request: &[u8], response_payload: &[u8], response_msg_type: u16, context: u16) -> Result<Vec<u8>>;
        
        #[namespace = "ZMQBridge"]
        fn send_response_through_proxy(response_payload: &[u8], response_msg_type: u16, correlation_id: &str, context: u16);
    }
}
pub use ffi::*;
