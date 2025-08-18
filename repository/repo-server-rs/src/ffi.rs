// src/ffi.rs
#[cxx::bridge(namespace = "SDMS")]
mod ffi {
    unsafe extern "C++" {
        // Make these prototypes visible to the generated C++.
        include!("bridge.hpp");

        // The generated C++ also includes ICommunicator for the type name:
        include!("common/ICommunicator.hpp");

        // Opaque C++ types (Rust never moves/owns them directly)
        type ICommunicator;
        type ICredentials;

        // Functions declared in bridge.hpp and defined in cxx/bridge.cc
        fn repo_set_log_defaults();
        fn repo_log_info(thread_name: &str, correlation_id: &str, thread_id: u64, msg: &str);

        fn make_client(
            host: &str,
            endpoint: &str,
            // mutable C++ references must be pinned for cxx
            credentials: &ICredentials,
            sid: u32,
            poll_timeout_ms: i64,
        ) -> UniquePtr<ICommunicator>;

        fn check_core_server_version(
            core_server_address: &str,
            thread_name: &str,
            correlation_id: &str,
            thread_id: u64,
        ) -> bool;
    }
}

// Rust struct that corresponds to C++ LogContext
#[repr(C)]
pub struct CxxLogContext {
    pub thread_name: String,
    pub correlation_id: String,
    pub thread_id: u64,
}

pub use ffi::*;
