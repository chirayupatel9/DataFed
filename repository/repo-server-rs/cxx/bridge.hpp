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
class SocketOptions;
class ProtoBufMap;
class ProtoBufFactory;

// Declarations the generated ffi.rs.cc expects to find.
void repo_set_log_defaults();
void repo_log_info(rust::Str thread_name, rust::Str correlation_id, std::uint64_t thread_id, rust::Str msg);

// Client creation with proper ZeroMQ and protobuf support
std::unique_ptr<ICommunicator> make_client(
    rust::Str host,
    rust::Str endpoint,
    std::uint32_t sid,
    std::int64_t poll_timeout_ms);

// Version check with proper protobuf message handling
bool check_core_server_version(rust::Str core_server_address, rust::Str thread_name, rust::Str correlation_id, std::uint64_t thread_id);

// Protobuf message creation and handling
std::unique_ptr<IMessage> create_version_request();
std::unique_ptr<IMessage> create_version_reply();
rust::String serialize_message_to_json(IMessage const& message);
std::unique_ptr<IMessage> deserialize_message_from_json(rust::Str json_str);

// ZeroMQ utility functions
bool send_message(std::unique_ptr<ICommunicator>& communicator, std::unique_ptr<IMessage>& message);
std::unique_ptr<IMessage> receive_message(std::unique_ptr<ICommunicator>& communicator);

} // namespace SDMS
