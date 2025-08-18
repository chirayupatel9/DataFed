// Protocol Buffer definitions matching the C++ implementation
pub mod sdms {
    include!(concat!(env!("OUT_DIR"), "/sdms.rs"));
}

pub mod sdms_auth {
    include!(concat!(env!("OUT_DIR"), "/sdms.auth.rs"));
}

pub mod sdms_anon {
    include!(concat!(env!("OUT_DIR"), "/sdms.anon.rs"));
}

pub mod version {
    include!(concat!(env!("OUT_DIR"), "/sdms.repository.version.rs"));
}

// Re-export commonly used types
pub use sdms::*;
pub use sdms_auth::*;
// Note: sdms_anon and version have conflicting names, so we don't re-export them globally
