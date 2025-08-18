// cxx/bridge.hpp
#pragma once

#include "rust/cxx.h"   // for rust::Str, UniquePtr
#include <memory>       // for std::unique_ptr

// Forward decls are enough for these function signatures.
namespace SDMS {
class ICommunicator;
class ICredentials;
class IMessage;
class MessageFactory;
class CommunicatorFactory;
class ICredentials;
class SocketOptions;

// Declarations the generated ffi.rs.cc expects to find.
void repo_set_log_defaults();
void repo_log_info(rust::Str thread_name, rust::Str correlation_id, std::uint64_t thread_id, rust::Str msg);
std::unique_ptr<ICommunicator> make_client(
    rust::Str host,
    rust::Str endpoint,
    ICredentials const& creds,
    std::uint32_t sid,
    std::int64_t poll_timeout_ms);

// Version check functions
bool check_core_server_version(rust::Str core_server_address, rust::Str thread_name, rust::Str correlation_id, std::uint64_t thread_id);
} // namespace SDMS
