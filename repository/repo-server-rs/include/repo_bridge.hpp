#pragma once
#include "rust/cxx.h"
#include <cstdint>

struct VersionInfo;
namespace RepoBridge {

// This is defined by the generated repo.rs.h.
// Forward-declare it *inside* the RepoBridge namespace so names match.

// Must exactly match src/ffi/repo.rs
::VersionInfo send_version_request(::rust::Str host,
    std::uint16_t port,
    ::rust::Str scheme,
    ::rust::Str core_public_key,
    std::uint32_t timeout_ms);
}
namespace ServerBridge {
    
    void server_start(rust::Str config_path, rust::Str repo_public_key, rust::Str repo_private_key);  // throws on error
    void server_stop();
    void server_join();
} // namespace ServerBridge

namespace ZMQBridge {
    
    rust::Vec<std::uint8_t> zmq_recv(std::int32_t timeout_ms);
    void zmq_send(rust::Slice<const std::uint8_t> payload, std::uint16_t msg_type, rust::Str correlation_id);
} // namespace ZMQBridge