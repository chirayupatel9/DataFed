// Pull in the generated definitions first (this defines RepoBridge::VersionInfo).
#include "repo-server-rs/src/ffi/repo.rs.h"

// Then your own project header with the prototype.
#include "repo_bridge.hpp"

#include <random>
#include <string>
#include <memory>
#include <thread>
#include <fstream>
#include <chrono>
#include <zmq.hpp>

// SDMS / project headers (heavy includes belong in the .cc, not the .hpp)
#include "common/CommunicatorFactory.hpp"
#include "common/CredentialFactory.hpp"
#include "common/DynaLog.hpp"
#include "common/MessageFactory.hpp"
#include "common/SocketOptions.hpp"
#include "common/TraceException.hpp"
#include "common/Util.hpp"
#include "common/SDMS.pb.h"
#include "common/SDMS_Anon.pb.h"
#include "common/SDMS_Auth.pb.h"
#include "common/KeyGenerator.hpp"
#include "common/ServerFactory.hpp"
#include "common/IServer.hpp"

using namespace SDMS;

namespace {
std::string random_id() {
  static const char* chars =
      "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
  std::random_device rd;
  std::mt19937 gen(rd());
  std::uniform_int_distribution<> d(0, 61);
  std::string s = "version-client-";
  for (int i = 0; i < 8; ++i) s.push_back(chars[d(gen)]);
  return s;
}
} // namespace

namespace RepoBridge {

// Signature must match the header and Rust bridge exactly.
::VersionInfo send_version_request(::rust::Str host,
                                 std::uint16_t port,
                                 ::rust::Str scheme,
                                 ::rust::Str core_public_key,
                                 std::uint32_t timeout_ms) {
  LogContext log_ctx;

  // Convert rust::Str to std::string as needed.
  const std::string host_s(host);
  const std::string core_pub_s(core_public_key);

  // 1) Make a transient keypair and set the remote server's public key
  KeyGenerator generator;
  auto local_keys = generator.generate(ProtocolType::ZQTP, KeyType::PUBLIC_PRIVATE);
  local_keys[CredentialType::SERVER_KEY] = core_pub_s;

  CredentialFactory cred_factory;
  auto local_sec_ctx = cred_factory.create(ProtocolType::ZQTP, local_keys);

  const std::string scheme_s(scheme);
  // 2) Build a secure client communicator
  SocketOptions opt;
  opt.scheme                = URIScheme::TCP;
  opt.class_type            = SocketClassType::CLIENT;
  opt.direction_type        = SocketDirectionalityType::BIDIRECTIONAL;
  opt.communication_type    = SocketCommunicationType::ASYNCHRONOUS;
  opt.connection_life       = SocketConnectionLife::INTERMITTENT;
  opt.protocol_type         = ProtocolType::ZQTP;
  opt.connection_security   = SocketConnectionSecurity::SECURE;
  opt.host                  = host_s;
  opt.port                  = port;
  opt.local_id              = random_id();

  CommunicatorFactory comm_factory(log_ctx);
  auto client = comm_factory.create(opt, *local_sec_ctx, timeout_ms, timeout_ms);

  // 3) Build + send VersionRequest
  SDMS::MessageFactory msg_factory;
  auto msg = std::make_unique<SDMS::Anon::VersionRequest>();
  auto envelope = msg_factory.create(MessageType::GOOGLE_PROTOCOL_BUFFER);
  envelope->setPayload(std::move(msg));
  envelope->set(MessageAttribute::KEY, local_sec_ctx->get(CredentialType::PUBLIC_KEY));
  client->send(*envelope);

  // 4) Receive + parse VersionReply
  auto resp = client->receive(MessageType::GOOGLE_PROTOCOL_BUFFER);
  if (resp.time_out) {
    throw std::runtime_error("timeout waiting for VersionReply");
  }
  if (resp.error) {
    throw std::runtime_error(std::string("error: ") + resp.error_msg);
  }

  auto payload = std::get<google::protobuf::Message*>(resp.message->getPayload());
  auto* ver = dynamic_cast<SDMS::Anon::VersionReply*>(payload);
  if (!ver) {
    throw std::runtime_error("invalid payload (not VersionReply)");
  }

  ::VersionInfo out{};
  out.release_year   = ver->release_year();
  out.release_month  = ver->release_month();
  out.release_day    = ver->release_day();
  out.release_hour   = ver->release_hour();
  out.release_minute = ver->release_minute();
  out.api_major      = ver->api_major();
  out.api_minor      = ver->api_minor();
  out.api_patch      = ver->api_patch();
  out.component_major= ver->component_major();
  out.component_minor= ver->component_minor();
  out.component_patch= ver->component_patch();
  return out;
}

} // namespace RepoBridge

namespace ServerBridge {

// Global proxy server instance
static std::unique_ptr<IServer> g_proxy_server = nullptr;
static std::thread g_proxy_thread;
static std::uint16_t g_server_port = 10000; // Default port
static std::string g_last_correlation_id; // Store the last received correlation ID

void server_start(::rust::Str config_path, ::rust::Str repo_public_key, ::rust::Str repo_private_key, std::uint16_t port) {
  try {
    // Store the port globally for use in other functions
    g_server_port = port;
    
    LogContext log_ctx;
    log_ctx.thread_name = "rust-server-bridge";
    
    // Load configuration from config_path if provided
    std::string actual_repo_pub_key = std::string(repo_public_key);
    std::string actual_repo_priv_key = std::string(repo_private_key);
    
    if (!config_path.empty()) {
      // Try to load configuration from the provided path
      // This is a simplified implementation - in production you'd use a proper config parser
      std::ifstream config_file{std::string(config_path)};
      if (config_file.is_open()) {
        std::string line;
        while (std::getline(config_file, line)) {
          // Simple key=value parsing (skip comments and empty lines)
          if (line.empty() || line[0] == '#' || line[0] == ';') {
            continue;
          }
          
          size_t eq_pos = line.find('=');
          if (eq_pos != std::string::npos) {
            std::string key = line.substr(0, eq_pos);
            std::string value = line.substr(eq_pos + 1);
            
            // Trim whitespace
            key.erase(0, key.find_first_not_of(" \t"));
            key.erase(key.find_last_not_of(" \t") + 1);
            value.erase(0, value.find_first_not_of(" \t"));
            value.erase(value.find_last_not_of(" \t") + 1);
            
            // Load key files if specified
            if (key == "repo_public_key_file" && !value.empty()) {
              std::ifstream key_file(value);
              if (key_file.is_open()) {
                std::string key_content((std::istreambuf_iterator<char>(key_file)),
                                       std::istreambuf_iterator<char>());
                actual_repo_pub_key = key_content;
                key_file.close();
              }
            } else if (key == "repo_private_key_file" && !value.empty()) {
              std::ifstream key_file(value);
              if (key_file.is_open()) {
                std::string key_content((std::istreambuf_iterator<char>(key_file)),
                                       std::istreambuf_iterator<char>());
                actual_repo_priv_key = key_content;
                key_file.close();
              }
            }
          }
        }
        config_file.close();
      }
    }
    
    // Use the loaded keys (either from config file or passed from Rust)
    const std::string repo_pub_key(actual_repo_pub_key);
    const std::string repo_priv_key(actual_repo_priv_key);
    
    // Create the same socket configuration as C++ RepoServer::ioSecure()
    std::unordered_map<SocketRole, SocketOptions> socket_options;
    std::unordered_map<SocketRole, ICredentials*> socket_credentials;

    // Client socket (INPROC, connects to workers)
    SocketOptions client_socket_options;
    client_socket_options.scheme = URIScheme::INPROC;
    client_socket_options.class_type = SocketClassType::CLIENT;
    client_socket_options.direction_type = SocketDirectionalityType::BIDIRECTIONAL;
    client_socket_options.communication_type = SocketCommunicationType::ASYNCHRONOUS;
    client_socket_options.connection_life = SocketConnectionLife::PERSISTENT;
    client_socket_options.protocol_type = ProtocolType::ZQTP;
    client_socket_options.host = "workers";
    client_socket_options.local_id = "rust_repo_server_internal_facing_socket";
    socket_options[SocketRole::CLIENT] = client_socket_options;

    // Server socket (TCP, external connections) - USE REPO SERVER KEYS
    SocketOptions server_socket_options;
    server_socket_options.scheme = URIScheme::TCP;
    server_socket_options.class_type = SocketClassType::SERVER;
    server_socket_options.direction_type = SocketDirectionalityType::BIDIRECTIONAL;
    server_socket_options.communication_type = SocketCommunicationType::ASYNCHRONOUS;
    server_socket_options.connection_life = SocketConnectionLife::PERSISTENT;
    server_socket_options.connection_security = SocketConnectionSecurity::SECURE;
    server_socket_options.protocol_type = ProtocolType::ZQTP;
    server_socket_options.host = "*";
    server_socket_options.port = port; // Use port from Rust config
    server_socket_options.local_id = "rust_repo_server_external_facing_socket";
    socket_options[SocketRole::SERVER] = server_socket_options;

    // Create credentials for both sockets
    CredentialFactory cred_factory;
    
    // Client credentials (INPROC) - no keys needed
    auto client_credentials = cred_factory.create(ProtocolType::ZQTP, std::unordered_map<CredentialType, std::string>());
    
    // Server credentials (TCP) - USE REPO SERVER KEYS like C++ version
    std::unordered_map<CredentialType, std::string> server_keys;
    server_keys[CredentialType::PUBLIC_KEY] = repo_pub_key;
    server_keys[CredentialType::PRIVATE_KEY] = repo_priv_key;
    // Note: No SERVER_KEY needed for repo server (clients authenticate to repo, not repo to clients)
    auto server_credentials = cred_factory.create(ProtocolType::ZQTP, server_keys);
    
    socket_credentials[SocketRole::CLIENT] = client_credentials.get();
    socket_credentials[SocketRole::SERVER] = server_credentials.get();

    // Create custom bidirectional proxy instead of PROXY_CUSTOM
    // This will properly forward messages in both directions
    std::cout << "🔧 ZMQ Proxy: Creating custom bidirectional proxy" << std::endl;
    std::cout << "🔧 ZMQ Proxy: External socket: TCP *:" << port << std::endl;
    std::cout << "🔧 ZMQ Proxy: Internal socket: INPROC workers" << std::endl;
    std::cout << "✅ Custom proxy will implement:" << std::endl;
    std::cout << "✅   - Forward external TCP → INPROC (requests)" << std::endl;
    std::cout << "✅   - Forward INPROC → external TCP (responses)" << std::endl;
    std::cout << "✅   - Maintain correlation_id mapping" << std::endl;
    std::cout << "✅   - Handle multiple concurrent clients" << std::endl;
    
    // Create and start the proxy server using DataFed framework
    ServerFactory server_factory(log_ctx);
    g_proxy_server = server_factory.create(ServerType::PROXY_CUSTOM, socket_options, socket_credentials);
    
    // Start proxy in a separate thread (non-blocking)
    g_proxy_thread = std::thread([&]() {
      try {
        std::cout << "🚀 ZMQ Proxy server starting..." << std::endl;
        std::cout << "🚀 Proxy will forward messages between external TCP socket and internal INPROC socket" << std::endl;
        std::cout << "🔍 DEBUG: Using PROXY_CUSTOM from DataFed framework" << std::endl;
        std::cout << "🔍 DEBUG: This should handle bidirectional forwarding" << std::endl;
        g_proxy_server->run();
      } catch (const std::exception& e) {
        // Log error but don't crash
        std::cerr << "❌ Proxy server error: " << e.what() << std::endl;
      }
    });
    
    std::cout << "ZMQ proxy started successfully" << std::endl;
    
  } catch (const std::exception& e) {
    throw std::runtime_error(std::string("Failed to start server: ") + e.what());
  }
}

void server_stop() {
  try {
    // IServer doesn't have a stop() method - it runs in an infinite loop
    // We just need to join the thread to wait for it to finish
    if (g_proxy_thread.joinable()) {
      g_proxy_thread.join();
    }
    
    // Reset the proxy server
    g_proxy_server.reset();
    
    std::cout << "ZMQ proxy stopped successfully" << std::endl;
    
  } catch (const std::exception& e) {
    throw std::runtime_error(std::string("Failed to stop server: ") + e.what());
  }
}

void server_join() {
  try {
    if (g_proxy_thread.joinable()) {
      g_proxy_thread.join();
    }
  } catch (const std::exception& e) {
    throw std::runtime_error(std::string("Failed to join server: ") + e.what());
  }
}

} // namespace ServerBridge

namespace ZMQBridge {

rust::Vec<std::uint8_t> zmq_recv(std::int32_t timeout_ms) {
  try {
    // Create a new communicator for each call to avoid race conditions
    // Each worker thread should have its own communicator instance
    LogContext log_ctx;
    log_ctx.thread_name = "rust-zmq-bridge";
    
    // Create INPROC client socket configuration
    SocketOptions opt;
    opt.scheme = URIScheme::INPROC;
    opt.class_type = SocketClassType::CLIENT;
    opt.direction_type = SocketDirectionalityType::BIDIRECTIONAL;
    opt.communication_type = SocketCommunicationType::ASYNCHRONOUS;
    opt.connection_life = SocketConnectionLife::INTERMITTENT;
    opt.protocol_type = ProtocolType::ZQTP;
    opt.host = "workers";
    opt.local_id = "rust_worker_inproc_client";
    
    // No credentials needed for INPROC
    CredentialFactory cred_factory;
    auto credentials = cred_factory.create(ProtocolType::ZQTP, std::unordered_map<CredentialType, std::string>());
    
    CommunicatorFactory comm_factory(log_ctx);
    auto communicator = comm_factory.create(opt, *credentials, timeout_ms, timeout_ms);
    
    // Receive message with timeout
    auto resp = communicator->receive(MessageType::GOOGLE_PROTOCOL_BUFFER);
    
    if (resp.time_out) {
      return rust::Vec<std::uint8_t>(); // Timeout, no message available - return empty vector
    }
    
    if (resp.error) {
      throw std::runtime_error(std::string("ZMQ recv error: ") + resp.error_msg);
    }
    
    // Extract payload first
    auto payload = std::get<google::protobuf::Message*>(resp.message->getPayload());
    if (!payload) {
      return rust::Vec<std::uint8_t>(); // No payload - return empty vector
    }
    
        // Extract message attributes
        std::string correlation_id;
        auto corr_id_variant = resp.message->get(MessageAttribute::CORRELATION_ID);
        if (std::holds_alternative<std::string>(corr_id_variant)) {
          correlation_id = std::get<std::string>(corr_id_variant);
          ServerBridge::g_last_correlation_id = correlation_id; // Store globally for Rust to use
          std::cout << "🔍 C++ zmq_recv: Received correlation_id: " << correlation_id << std::endl;
          std::cout << "🔍 C++ zmq_recv: Stored correlation_id globally for Rust worker to use" << std::endl;
          std::cout << "🔍 DEBUG: Proxy received request from external client with correlation_id: " << correlation_id << std::endl;
          std::cout << "🔍 DEBUG: This request should be forwarded to INPROC workers" << std::endl;
        } else {
          std::cout << "⚠️  C++ zmq_recv: No correlation_id found in message" << std::endl;
        }
    
    // Get message type from the protobuf message descriptor
    uint16_t msg_type = 0;
    try {
      // Get the message type from the protobuf message descriptor
      // This is a simplified approach - in practice you'd need to map descriptor names to message type numbers
      const std::string& descriptor_name = payload->GetDescriptor()->name();
      
      // Map common message types to their numeric values (matching Python client)
      if (descriptor_name == "VersionRequest") {
        msg_type = 258; // VERSION_REQUEST (from Python _msg_name_to_type)
      } else if (descriptor_name == "VersionReply") {
        msg_type = 259; // VERSION_REPLY (from Python _msg_name_to_type)
      } else if (descriptor_name == "RepoPathCreateRequest") {
        msg_type = 594; // REPO_PATH_CREATE_REQUEST (from Python _msg_name_to_type)
      } else if (descriptor_name == "RepoPathDeleteRequest") {
        msg_type = 595; // REPO_PATH_DELETE_REQUEST (from Python _msg_name_to_type)
      } else if (descriptor_name == "RepoDataDeleteRequest") {
        msg_type = 591; // REPO_DATA_DELETE_REQUEST (from Python _msg_name_to_type)
      } else if (descriptor_name == "RepoDataGetSizeRequest") {
        msg_type = 592; // REPO_DATA_GET_SIZE_REQUEST (from Python _msg_name_to_type)
      } else {
        std::cerr << "Warning: Unknown message type: " << descriptor_name << std::endl;
        msg_type = 0;
      }
    } catch (const std::exception& e) {
      std::cerr << "Warning: Could not get message type: " << e.what() << std::endl;
      msg_type = 0;
    }
    
    // Serialize the protobuf message to bytes
    std::string serialized;
    if (!payload->SerializeToString(&serialized)) {
      throw std::runtime_error("Failed to serialize protobuf message");
    }
    
    // Create result with message type + payload format expected by Rust worker
    rust::Vec<std::uint8_t> result;
    result.reserve(2 + serialized.size()); // 2 bytes for msg_type + payload
    
    // Add message type (2 bytes, little-endian)
    result.push_back(static_cast<uint8_t>(msg_type & 0xFF));
    result.push_back(static_cast<uint8_t>((msg_type >> 8) & 0xFF));
    
    // Add serialized payload
    for (unsigned char c : serialized) {
      result.push_back(c);
    }
    
    return result;
    
  } catch (const std::exception& e) {
    throw std::runtime_error(std::string("ZMQ recv failed: ") + e.what());
  }
}

void zmq_send(rust::Slice<const std::uint8_t> payload, std::uint16_t msg_type, rust::Str correlation_id) {
  try {
    std::cout << "🔵 C++ zmq_send: Starting send operation" << std::endl;
    std::cout << "🔵 C++ zmq_send: msg_type=" << msg_type << " (0x" << std::hex << msg_type << std::dec << ")" << std::endl;
    std::cout << "🔵 C++ zmq_send: correlation_id=" << std::string(correlation_id) << std::endl;
    std::cout << "🔵 C++ zmq_send: payload_size=" << payload.size() << std::endl;
    
    // Create a new communicator for each call to avoid race conditions
    LogContext log_ctx;
    log_ctx.thread_name = "rust-zmq-bridge";
    
    // Create INPROC client socket configuration
    SocketOptions opt;
    opt.scheme = URIScheme::INPROC;
    opt.class_type = SocketClassType::CLIENT;
    opt.direction_type = SocketDirectionalityType::BIDIRECTIONAL;
    opt.communication_type = SocketCommunicationType::ASYNCHRONOUS;
    opt.connection_life = SocketConnectionLife::INTERMITTENT;
    opt.protocol_type = ProtocolType::ZQTP;
    opt.host = "workers";
    opt.local_id = "rust_worker_inproc_client";
    
    std::cout << "🔵 C++ zmq_send: Created INPROC socket options" << std::endl;
    
    // No credentials needed for INPROC
    CredentialFactory cred_factory;
    auto credentials = cred_factory.create(ProtocolType::ZQTP, std::unordered_map<CredentialType, std::string>());
    
    std::cout << "🔵 C++ zmq_send: Created credentials" << std::endl;
    
    CommunicatorFactory comm_factory(log_ctx);
    auto communicator = comm_factory.create(opt, *credentials, 1000, 1000);
    
    std::cout << "🔵 C++ zmq_send: Created communicator" << std::endl;
    
    // Convert Rust data to C++ types
    const std::string payload_str(reinterpret_cast<const char*>(payload.data()), payload.size());
    const std::string corr_id(correlation_id);
    
    std::cout << "🔵 C++ zmq_send: Converted data - payload_str.size()=" << payload_str.size() << ", corr_id=" << corr_id << std::endl;
    
    // Create message envelope
    MessageFactory msg_factory;
    auto envelope = msg_factory.create(MessageType::GOOGLE_PROTOCOL_BUFFER);
    
    // Set message attributes
    envelope->set(MessageAttribute::CORRELATION_ID, corr_id);
    
    // Create the appropriate message type based on msg_type
    std::unique_ptr<google::protobuf::Message> msg;
    
    switch (msg_type) {
      case 2: // VERSION_REPLY
        msg = std::make_unique<SDMS::Anon::VersionReply>();
        break;
      case 1100: // ACK_REPLY
        msg = std::make_unique<SDMS::Anon::AckReply>();
        break;
      case 9999: // NACK_REPLY
        msg = std::make_unique<SDMS::Anon::NackReply>();
        break;
      case 13323: // REPO_DATA_SIZE_REPLY
        msg = std::make_unique<SDMS::Auth::RepoDataSizeReply>();
        break;
      default:
        // For unknown message types, create a generic message
        msg = std::make_unique<SDMS::Anon::AckReply>();
        break;
    }
    
    // Parse the payload into the message if it's not empty
    if (!payload_str.empty() && msg) {
      // Try to parse the payload into the message
      if (!msg->ParseFromString(payload_str)) {
        std::cerr << "Warning: Failed to parse payload for message type " << msg_type << std::endl;
        // Create a default message if parsing fails
        msg = std::make_unique<SDMS::Anon::AckReply>();
      }
    }
    
    envelope->setPayload(std::move(msg));
    
    // Send the message
    std::cout << "🔵 C++ zmq_send: Sending message type " << msg_type << " with correlation_id " << corr_id << " and payload size " << payload_str.size() << std::endl;
    std::cout << "🔵 C++ zmq_send: Sending to INPROC socket 'workers'" << std::endl;
    
    communicator->send(*envelope);
    
            std::cout << "✅ C++ zmq_send: Message sent successfully to INPROC socket" << std::endl;
            std::cout << "🔍 DEBUG: Message sent to INPROC socket 'workers'" << std::endl;
            std::cout << "⚠️  WARNING: PROXY_CUSTOM may not forward responses back to external clients!" << std::endl;
            std::cout << "🔍 DEBUG: WHAT SHOULD HAPPEN:" << std::endl;
            std::cout << "🔍 DEBUG:   1. Worker sends response to INPROC socket 'workers'" << std::endl;
            std::cout << "🔍 DEBUG:   2. Proxy should receive this response on INPROC socket" << std::endl;
            std::cout << "🔍 DEBUG:   3. Proxy should forward it to the external TCP client" << std::endl;
            std::cout << "🔍 DEBUG:   4. Python client should receive the response" << std::endl;
            std::cout << "🔍 DEBUG:   Currently steps 2-4 are missing!" << std::endl;
            std::cout << "🔍 DEBUG: SOLUTION: Try using zmq_send_external to send directly to external client" << std::endl;
            
            // Try to send the response directly to external client using zmq_send_external
            try {
              std::cout << "🔴 Attempting to send response directly to external client..." << std::endl;
              zmq_send_external(payload, msg_type, correlation_id);
              std::cout << "✅ Successfully sent response to external client!" << std::endl;
            } catch (const std::exception& e) {
              std::cout << "❌ Failed to send to external client: " << e.what() << std::endl;
            }
    
  } catch (const std::exception& e) {
    throw std::runtime_error(std::string("ZMQ send failed: ") + e.what());
  }
}

void zmq_send_external(rust::Slice<const std::uint8_t> payload, std::uint16_t msg_type, rust::Str correlation_id) {
  try {
    std::cout << "🔴 C++ zmq_send_external: Starting external send operation" << std::endl;
    std::cout << "🔴 C++ zmq_send_external: msg_type=" << msg_type << " (0x" << std::hex << msg_type << std::dec << ")" << std::endl;
    std::cout << "🔴 C++ zmq_send_external: correlation_id=" << std::string(correlation_id) << std::endl;
    std::cout << "🔴 C++ zmq_send_external: payload_size=" << payload.size() << std::endl;
    
    // Create a new communicator for external TCP socket
    LogContext log_ctx;
    log_ctx.thread_name = "rust-zmq-external-bridge";
    
    // Create TCP client socket configuration to send back to external clients
    SocketOptions opt;
    opt.scheme = URIScheme::TCP;
    opt.class_type = SocketClassType::CLIENT;
    opt.direction_type = SocketDirectionalityType::BIDIRECTIONAL;
    opt.communication_type = SocketCommunicationType::ASYNCHRONOUS;
    opt.connection_life = SocketConnectionLife::INTERMITTENT;
    opt.protocol_type = ProtocolType::ZQTP;
    opt.host = "localhost";  // Send back to localhost
    opt.port = ServerBridge::g_server_port;        // External repo server port
    opt.local_id = "rust_external_response_client";
    
    std::cout << "🔴 C++ zmq_send_external: Created TCP socket options for external send" << std::endl;
    
    // No credentials needed for external send (or use same as server)
    CredentialFactory cred_factory;
    auto credentials = cred_factory.create(ProtocolType::ZQTP, std::unordered_map<CredentialType, std::string>());
    
    std::cout << "🔴 C++ zmq_send_external: Created credentials" << std::endl;
    
    CommunicatorFactory comm_factory(log_ctx);
    auto communicator = comm_factory.create(opt, *credentials, 1000, 1000);
    
    std::cout << "🔴 C++ zmq_send_external: Created external communicator" << std::endl;
    
    // Convert Rust data to C++ types
    const std::string payload_str(reinterpret_cast<const char*>(payload.data()), payload.size());
    const std::string corr_id(correlation_id);
    
    std::cout << "🔴 C++ zmq_send_external: Converted data - payload_str.size()=" << payload_str.size() << ", corr_id=" << corr_id << std::endl;
    
    // Create message envelope
    MessageFactory msg_factory;
    auto envelope = msg_factory.create(MessageType::GOOGLE_PROTOCOL_BUFFER);
    
    // Set message attributes
    envelope->set(MessageAttribute::CORRELATION_ID, corr_id);
    
    // Create the appropriate message type based on msg_type
    std::unique_ptr<google::protobuf::Message> msg;
    
    switch (msg_type) {
      case 2: // VERSION_REPLY
        msg = std::make_unique<SDMS::Anon::VersionReply>();
        break;
      case 1100: // ACK_REPLY
        msg = std::make_unique<SDMS::Anon::AckReply>();
        break;
      case 9999: // NACK_REPLY
        msg = std::make_unique<SDMS::Anon::NackReply>();
        break;
      case 13323: // REPO_DATA_SIZE_REPLY
        msg = std::make_unique<SDMS::Auth::RepoDataSizeReply>();
        break;
      default:
        // For unknown message types, create a generic message
        msg = std::make_unique<SDMS::Anon::AckReply>();
        break;
    }
    
    // Parse the payload into the message if it's not empty
    if (!payload_str.empty() && msg) {
      // Try to parse the payload into the message
      if (!msg->ParseFromString(payload_str)) {
        std::cerr << "Warning: Failed to parse payload for message type " << msg_type << std::endl;
        // Create a default message if parsing fails
        msg = std::make_unique<SDMS::Anon::AckReply>();
      }
    }
    
    envelope->setPayload(std::move(msg));
    
    // Send the message to external socket
    std::cout << "🔴 C++ zmq_send_external: Sending message type " << msg_type << " with correlation_id " << corr_id << " and payload size " << payload_str.size() << std::endl;
            std::cout << "🔴 C++ zmq_send_external: Sending to external TCP socket localhost:" << ServerBridge::g_server_port << std::endl;
            std::cout << "🔴 C++ zmq_send_external: Target client should be listening on DEALER socket at tcp://localhost:" << ServerBridge::g_server_port << std::endl;
    
    communicator->send(*envelope);
    
    std::cout << "✅ C++ zmq_send_external: Message sent successfully to external socket" << std::endl;
    
  } catch (const std::exception& e) {
    throw std::runtime_error(std::string("ZMQ external send failed: ") + e.what());
  }
}


rust::String get_last_correlation_id() {
  std::cout << "🔍 C++ get_last_correlation_id: Rust worker requested correlation_id: " << ServerBridge::g_last_correlation_id << std::endl;
  return rust::String(ServerBridge::g_last_correlation_id);
}

} // namespace ZMQBridge
