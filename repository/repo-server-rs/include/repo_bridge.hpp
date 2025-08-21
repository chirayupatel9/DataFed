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

} // namespace RepoBridge

// // // include/repo_bridge.hpp
// // #pragma once
// // #include "rust/cxx.h"
// // #include <cstdint>
// // #include "repo-server-rs/src/ffi/repo.rs.h"  // cxx-generated header for VersionInfo mapping

// // struct VersionInfo;

// // namespace RepoBridge {
// // VersionInfo send_version_request(rust::Str core_addr,
// //                                  rust::Str core_pub_key,
// //                                  uint32_t  creds_mask);
// // } // namespace RepoBridge

// #pragma once

// #include "rust/cxx.h"

// // Your project headers
// #include "common/CommunicatorFactory.hpp"
// #include "common/CredentialFactory.hpp"
// #include "common/DynaLog.hpp"
// #include "common/MessageFactory.hpp"
// #include "common/SocketOptions.hpp"
// #include "common/TraceException.hpp"
// #include "common/Util.hpp"

// // Protobuf types (VersionRequest/VersionReply are defined here)
// #include "common/SDMS.pb.h"

// // 🔧 NEW: add the specific headers that declare these symbols
// #include "common/KeyGenerator.hpp"       // SDMS::KeyGenerator, SDMS::KeyType
// #include "common/CredentialFactory.hpp"    // SDMS::CredentialType
// #include "common/SocketOptions.hpp"    // SDMS::AddressSplitter
// // Struct returned to Rust
// struct VersionInfo; 
// // {
// //   uint32_t release_year, release_month, release_day, release_hour, release_minute;
// //   uint32_t api_major, api_minor, api_patch;
// //   uint32_t component_major, component_minor, component_patch;
// // };
// namespace RepoBridge {


// // Sends VersionRequest to `core_addr` using `core_public_key` (ZQTP secure).
// // Throws rust::Error on timeout/bad payload/etc.
// VersionInfo send_version_request(rust::Str core_addr,
//                                  rust::Str core_public_key,
//                                  uint32_t timeout_ms);

// } // namespace RepoBridge
