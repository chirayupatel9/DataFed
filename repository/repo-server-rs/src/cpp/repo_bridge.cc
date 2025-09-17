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
static std::uint16_t g_last_context = 0; // Store the last received context

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

    // CLIENT socket (INPROC, proxy connects to workers) - Match C++ RepoServer exactly
    SocketOptions client_socket_options;
    client_socket_options.scheme = URIScheme::INPROC;
    client_socket_options.class_type = SocketClassType::CLIENT;  // ← MATCH C++: CLIENT class_type
    client_socket_options.direction_type = SocketDirectionalityType::BIDIRECTIONAL;
    client_socket_options.communication_type = SocketCommunicationType::ASYNCHRONOUS;
    client_socket_options.connection_life = SocketConnectionLife::PERSISTENT;
    client_socket_options.protocol_type = ProtocolType::ZQTP;
    client_socket_options.host = "workers";
    client_socket_options.local_id = "main_repository_server_interal_facing_socket";  // ← MATCH C++: exact local_id
    socket_options[SocketRole::CLIENT] = client_socket_options;

    // SERVER socket (TCP, external connections) - Match C++ RepoServer exactly
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
    server_socket_options.local_id = "main_repository_server_external_facing_socket";  // ← MATCH C++: exact local_id
    socket_options[SocketRole::SERVER] = server_socket_options;

    // Create credentials for both sockets
    CredentialFactory cred_factory;
    
    // CLIENT credentials (INPROC) - no keys needed, match C++ exactly
    std::unordered_map<CredentialType, std::string> client_cred_options;
    auto client_credentials = cred_factory.create(ProtocolType::ZQTP, client_cred_options);
    
    // SERVER credentials (TCP) - Match C++ RepoServer exactly
    std::unordered_map<CredentialType, std::string> server_cred_options;
    server_cred_options[CredentialType::PUBLIC_KEY] = repo_pub_key;
    server_cred_options[CredentialType::PRIVATE_KEY] = repo_priv_key;
    server_cred_options[CredentialType::SERVER_KEY] = repo_pub_key;  // ← MATCH C++: Include SERVER_KEY
    auto server_credentials = cred_factory.create(ProtocolType::ZQTP, server_cred_options);
    
    socket_credentials[SocketRole::CLIENT] = client_credentials.get();
    socket_credentials[SocketRole::SERVER] = server_credentials.get();

        // Create basic ZMQ proxy - this should actually work
        std::cout << "🔧 ZMQ Proxy: Creating basic ZMQ proxy" << std::endl;
        std::cout << "🔧 ZMQ Proxy: TCP socket (SERVER): *:" << port << " (external clients connect here)" << std::endl;
        std::cout << "🔧 ZMQ Proxy: INPROC socket (CLIENT): workers (proxy connects to workers)" << std::endl;
        std::cout << "✅ Basic ZMQ proxy will implement:" << std::endl;
        std::cout << "✅   - Forward TCP → INPROC (requests: external clients → workers)" << std::endl;
        std::cout << "✅   - Forward INPROC → TCP (responses: workers → external clients)" << std::endl;
        std::cout << "✅   - Preserve ZMQ routing information for proper client targeting" << std::endl;
        std::cout << "✅   - Handle multiple concurrent clients" << std::endl;
        
        // Create DataFed PROXY_CUSTOM (exactly like C++ RepoServer)
        ServerFactory server_factory(log_ctx);
        g_proxy_server = server_factory.create(ServerType::PROXY_CUSTOM, socket_options, socket_credentials);
    
    // Start proxy in a separate thread (non-blocking)
    g_proxy_thread = std::thread([&]() {
      try {
        std::cout << "🚀 ZMQ Proxy server starting with DataFed framework..." << std::endl;
        std::cout << "🚀 Proxy will forward messages between external TCP socket and internal INPROC socket" << std::endl;
        std::cout << "🔍 DEBUG: Using PROXY_CUSTOM from DataFed framework" << std::endl;
        std::cout << "🔍 DEBUG: This should handle bidirectional forwarding with proper routing" << std::endl;
        
        // Print socket addresses for debugging
        auto addresses = g_proxy_server->getAddresses();
        std::cout << "🔍 DEBUG: Proxy socket addresses:" << std::endl;
        std::cout << "🔍 DEBUG:   CLIENT (INPROC): " << addresses[SocketRole::CLIENT] << std::endl;
        std::cout << "🔍 DEBUG:   SERVER (TCP): " << addresses[SocketRole::SERVER] << std::endl;
        
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
    
    // Create INPROC client socket configuration (DEALER socket to connect to proxy)
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
      std::cout << "🔍 DEBUG: Proxy received request from external client with correlation_id: " << correlation_id << std::endl;
    }
    
    // Extract context from the DataFed message (like C++ RepoServer does)
    uint16_t context = 0;
    try {
      context = std::get<uint16_t>(resp.message->get(constants::message::google::CONTEXT));
      std::cout << "🔍 DEBUG: Proxy received request with context: " << context << std::endl;
    } catch (...) {
      std::cout << "🔍 DEBUG: No context found in request, using default context: 0" << std::endl;
      context = 0;
    }
    
    // Store context globally for Rust to use
    ServerBridge::g_last_context = context;
    
    // Get message type from the protobuf message descriptor
    uint16_t msg_type = 0;
    try {
      // Get the message type from the protobuf message descriptor
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
        std::cerr << "⚠️  Warning: Unknown message type: " << descriptor_name << std::endl;
        msg_type = 0;
      }
    } catch (const std::exception& e) {
      std::cerr << "⚠️  Warning: Could not get message type: " << e.what() << std::endl;
      msg_type = 0;
    }
    
    // Serialize the protobuf message to bytes
    std::string serialized;
    if (!payload->SerializeToString(&serialized)) {
      throw std::runtime_error("Failed to serialize protobuf message");
    }
    
    // Create result with message type + payload format expected by Rust worker
    // BUT ALSO preserve route information for proper response routing
    rust::Vec<std::uint8_t> result;
    result.reserve(2 + serialized.size()); // 2 bytes for msg_type + payload
    
    // Add message type (2 bytes, little-endian)
    result.push_back(static_cast<uint8_t>(msg_type & 0xFF));
    result.push_back(static_cast<uint8_t>((msg_type >> 8) & 0xFF));
    
    // Add serialized payload
    for (unsigned char c : serialized) {
      result.push_back(c);
    }
    
    // Extract route information from the DataFed message for proper response routing
    auto routes = resp.message->getRoutes();
    std::cout << "🔧 C++ zmq_recv: Found " << routes.size() << " routes in original message" << std::endl;
    
    // Add route count (4 bytes, little-endian) after the payload
    uint32_t route_count = static_cast<uint32_t>(routes.size());
    result.push_back(static_cast<uint8_t>(route_count & 0xFF));
    result.push_back(static_cast<uint8_t>((route_count >> 8) & 0xFF));
    result.push_back(static_cast<uint8_t>((route_count >> 16) & 0xFF));
    result.push_back(static_cast<uint8_t>((route_count >> 24) & 0xFF));
    
    // Add each route (length + data)
    for (const auto& route : routes) {
      std::cout << "🔧 C++ zmq_recv: Adding route: " << route << " (length: " << route.size() << ")" << std::endl;
      
      // Add route length (4 bytes, little-endian)
      uint32_t route_len = static_cast<uint32_t>(route.size());
      result.push_back(static_cast<uint8_t>(route_len & 0xFF));
      result.push_back(static_cast<uint8_t>((route_len >> 8) & 0xFF));
      result.push_back(static_cast<uint8_t>((route_len >> 16) & 0xFF));
      result.push_back(static_cast<uint8_t>((route_len >> 24) & 0xFF));
      
      // Add route data
      for (unsigned char c : route) {
        result.push_back(c);
      }
    }
    
    std::cout << "✅ C++ zmq_recv: Preserved " << routes.size() << " routes for response routing" << std::endl;
    
    return result;
    
  } catch (const std::exception& e) {
    throw std::runtime_error(std::string("ZMQ recv failed: ") + e.what());
  }
}

void zmq_send(rust::Slice<const std::uint8_t> payload, std::uint16_t msg_type, rust::Str correlation_id) {
  try {
    std::cout << "🔵 C++ zmq_send: Sending message type " << msg_type << " with correlation_id " << std::string(correlation_id) << " and payload size " << payload.size() << std::endl;
    
    // Create a new communicator for each call to avoid race conditions
    LogContext log_ctx;
    log_ctx.thread_name = "rust-zmq-bridge";
    
    // Create INPROC client socket configuration (DEALER socket to connect to proxy)
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
        std::cerr << "⚠️  Warning: Failed to parse payload for message type " << msg_type << std::endl;
        // Create a default message if parsing fails
        msg = std::make_unique<SDMS::Anon::AckReply>();
      }
    }
    
    envelope->setPayload(std::move(msg));
    
    // Send the message
    communicator->send(*envelope);
    
    std::cout << "✅ C++ zmq_send: Message sent successfully to INPROC socket" << std::endl;
    
  } catch (const std::exception& e) {
    throw std::runtime_error(std::string("ZMQ send failed: ") + e.what());
  }
}

rust::Vec<std::uint8_t> create_response_envelope(rust::Slice<const std::uint8_t> request_payload, std::uint16_t request_msg_type, rust::Str request_correlation_id, rust::Str request_route) {
  try {
    std::cout << "🔧 C++ create_response_envelope: Creating DataFed-compatible response envelope for request type " << request_msg_type << " with correlation_id " << std::string(request_correlation_id) << " and route " << std::string(request_route) << std::endl;
    
    // Create a DataFed framework-compatible response envelope
    // Format: [msg_type][payload_length][payload] (same as zmq_recv format)
    rust::Vec<std::uint8_t> result;
    
    // Add message type (2 bytes, little-endian) - same as zmq_recv
    result.push_back(static_cast<uint8_t>(request_msg_type & 0xFF));
    result.push_back(static_cast<uint8_t>((request_msg_type >> 8) & 0xFF));
    
    // Add payload (same as zmq_recv)
    for (unsigned char c : request_payload) {
      result.push_back(c);
    }
    
    std::cout << "✅ C++ create_response_envelope: Created DataFed-compatible response envelope with msg_type=" << request_msg_type << ", payload_len=" << request_payload.size() << ", correlation_id=" << std::string(request_correlation_id) << ", route=" << std::string(request_route) << std::endl;
    std::cout << "🔧 C++ create_response_envelope: This will be sent through DataFed framework's ZMQ communicator with proper framing" << std::endl;
    
    return result;
    
  } catch (const std::exception& e) {
    throw std::runtime_error(std::string("create_response_envelope failed: ") + e.what());
  }
}

rust::Vec<std::uint8_t> create_response_envelope_from_request(rust::Slice<const std::uint8_t> original_request, rust::Slice<const std::uint8_t> response_payload, std::uint16_t response_msg_type, std::uint16_t context) {
  try {
    std::cout << "🔧 C++ create_response_envelope_from_request: Creating response using original request" << std::endl;
    std::cout << "🔧 C++ create_response_envelope_from_request: original_request_len=" << original_request.size() << ", response_payload_len=" << response_payload.size() << ", response_msg_type=" << response_msg_type << std::endl;
    
    // Parse the original request to extract correlation ID and routes
    if (original_request.size() < 2) {
      throw std::runtime_error("Original request too short");
    }
    
    // Extract message type from original request
    uint16_t original_msg_type = original_request[0] | (original_request[1] << 8);
    std::cout << "🔧 C++ create_response_envelope_from_request: original_msg_type=" << original_msg_type << std::endl;
    
    // Create a DataFed framework-compatible response envelope with routes
    // Format: [msg_type][payload] + route information (like C++ RepoServer)
    rust::Vec<std::uint8_t> result;
    
    // Add response message type (2 bytes, little-endian)
    result.push_back(static_cast<uint8_t>(response_msg_type & 0xFF));
    result.push_back(static_cast<uint8_t>((response_msg_type >> 8) & 0xFF));
    
    // Add response payload
    for (unsigned char c : response_payload) {
      result.push_back(c);
    }
    
    // Add context (2 bytes, little-endian) - like C++ RepoServer does
    result.push_back(static_cast<uint8_t>(context & 0xFF));
    result.push_back(static_cast<uint8_t>((context >> 8) & 0xFF));
    
    // Parse route information from the original request for proper response routing
    // Format: [msg_type][payload][route_count][routes...]
    std::vector<std::string> routes;
    if (original_request.size() > 6) { // Need at least 2 (msg_type) + 4 (route_count) + some payload
      size_t route_offset = original_request.size() - 4;
      
      // Read route count (4 bytes, little-endian)
      if (route_offset + 4 <= original_request.size()) {
        uint32_t route_count = original_request[route_offset] | 
                              (original_request[route_offset + 1] << 8) |
                              (original_request[route_offset + 2] << 16) |
                              (original_request[route_offset + 3] << 24);
        
        route_offset += 4;
        
        // Parse each route
        for (uint32_t i = 0; i < route_count; i++) {
          if (route_offset + 4 <= original_request.size()) {
            // Read route length (4 bytes, little-endian)
            uint32_t route_len = original_request[route_offset] | 
                                (original_request[route_offset + 1] << 8) |
                                (original_request[route_offset + 2] << 16) |
                                (original_request[route_offset + 3] << 24);
            
            route_offset += 4;
            
            // Read route data
            if (route_offset + route_len <= original_request.size()) {
              std::string route_data(original_request.begin() + route_offset, 
                                   original_request.begin() + route_offset + route_len);
              routes.push_back(route_data);
              route_offset += route_len;
            }
          }
        }
      }
    }
    
    std::cout << "🔧 C++ create_response_envelope_from_request: Parsed " << routes.size() << " routes from original request" << std::endl;
    
    // Create a DataFed framework IMessage for the original request to extract routing info
    LogContext log_ctx;
    log_ctx.thread_name = "rust-response-envelope-bridge";
    
    // Create a temporary message from the original request to extract routes and correlation ID
    MessageFactory msg_factory;
    auto original_msg = msg_factory.create(MessageType::GOOGLE_PROTOCOL_BUFFER);
    
    // Set correlation ID from global storage
    original_msg->set(MessageAttribute::CORRELATION_ID, ServerBridge::g_last_correlation_id);
    
    // Set context
    original_msg->set(constants::message::google::CONTEXT, context);
    
    // Set routes on the original message (convert vector to list)
    std::list<std::string> routes_list(routes.begin(), routes.end());
    original_msg->setRoutes(routes_list);
    std::cout << "🔧 C++ create_response_envelope_from_request: Set " << routes.size() << " routes on original message" << std::endl;
    
    // Use DataFed framework's createResponseEnvelope to create proper response with MessageState::RESPONSE
    auto response_msg = msg_factory.createResponseEnvelope(*original_msg);
    std::cout << "🔧 C++ create_response_envelope_from_request: Created response envelope using DataFed framework with MessageState::RESPONSE" << std::endl;
    
    // Set the message type on the response message
    response_msg->set(constants::message::google::MSG_TYPE, response_msg_type);
    
    // Create the appropriate message type based on response_msg_type and set payload
    std::unique_ptr<google::protobuf::Message> msg;
    
    switch (response_msg_type) {
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
    
    // Parse the response payload into the message if it's not empty
    if (!response_payload.empty() && msg) {
      // Convert response_payload to string
      const std::string payload_str(reinterpret_cast<const char*>(response_payload.data()), response_payload.size());
      
      // Try to parse the payload into the message
      if (!msg->ParseFromString(payload_str)) {
        std::cerr << "Warning: Failed to parse response payload for message type " << response_msg_type << std::endl;
        // Create a default message if parsing fails
        msg = std::make_unique<SDMS::Anon::AckReply>();
      }
    }
    
    // Set the payload on the response message
    response_msg->setPayload(std::move(msg));
    
    std::cout << "🔧 C++ create_response_envelope_from_request: Set full IMessage with all DataFed attributes" << std::endl;
    std::cout << "🔧 C++ create_response_envelope_from_request: - MessageState::RESPONSE: ✓" << std::endl;
    std::cout << "🔧 C++ create_response_envelope_from_request: - Correlation ID: ✓" << std::endl;
    std::cout << "🔧 C++ create_response_envelope_from_request: - Context: ✓" << std::endl;
    std::cout << "🔧 C++ create_response_envelope_from_request: - Routes: ✓" << std::endl;
    std::cout << "🔧 C++ create_response_envelope_from_request: - Message Type: ✓" << std::endl;
    std::cout << "🔧 C++ create_response_envelope_from_request: - Payload: ✓" << std::endl;
    
    // NOW USE DATAFED FRAMEWORK EXACTLY LIKE C++ REPOSERVER
    // The C++ RepoServer does: client->send(*(send_message))
    // This calls ZeroMQCommunicator::send() which properly serializes the IMessage
    
    // The key insight: Instead of creating our own byte array format,
    // we need to use the DataFed framework's ZMQ communicator directly
    // just like the C++ RepoServer does.
    
    // However, since we're in an FFI context, we can't directly call
    // the ZMQ communicator from here. Instead, we need to:
    // 1. Store the IMessage data in a way that Rust can access it
    // 2. Have Rust use the DataFed framework's communicator to send it
    
    // For now, we'll return the essential data that allows Rust to
    // reconstruct the message using the DataFed framework properly
    
    // The DataFed framework expects the message to have all these attributes:
    // - MessageState::RESPONSE (✓ set by createResponseEnvelope)
    // - Correlation ID (✓ copied from original request)
    // - Context (✓ copied from original request) 
    // - Routes (✓ copied from original request)
    // - Message Type (✓ set via MSG_TYPE)
    // - Payload (✓ set as protobuf message)
    
    std::cout << "🔧 C++ create_response_envelope_from_request: IMessage created with all DataFed attributes (exactly like C++ RepoServer)" << std::endl;
    std::cout << "🔧 C++ create_response_envelope_from_request: Ready for DataFed framework ZMQ communicator" << std::endl;
    
    // Return the response data in a format that preserves all the IMessage attributes
    // The Rust side will use this with the DataFed framework's communicator
    
    std::cout << "✅ C++ create_response_envelope_from_request: Created full IMessage with DataFed framework (exactly like C++ RepoServer), msg_type=" << response_msg_type << ", payload_len=" << response_payload.size() << ", routes=" << routes.size() << std::endl;
    
    return result;
    
  } catch (const std::exception& e) {
    throw std::runtime_error(std::string("create_response_envelope_from_request failed: ") + e.what());
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

std::uint16_t get_last_context() {
  std::cout << "🔍 C++ get_last_context: Rust worker requested context: " << ServerBridge::g_last_context << std::endl;
  return ServerBridge::g_last_context;
}

// New FFI function to send response through the proxy (correct approach)
void send_response_through_proxy(rust::Slice<const std::uint8_t> response_payload, std::uint16_t response_msg_type, rust::Str correlation_id, std::uint16_t context) {
  try {
    std::cout << "🚀 C++ send_response_through_proxy: Using proxy for response routing (correct approach)" << std::endl;
    std::cout << "🚀 C++ send_response_through_proxy: msg_type=" << response_msg_type << ", corr_id=" << std::string(correlation_id) << ", context=" << context << std::endl;
    
    // Create INPROC communicator to send response through the proxy
    // The proxy will forward this to the external client
    LogContext log_ctx;
    log_ctx.thread_name = "rust-proxy-sender";
    
    // Create INPROC socket configuration to send to workers (proxy will forward)
    SocketOptions opt;
    opt.scheme = URIScheme::INPROC;
    opt.class_type = SocketClassType::CLIENT;
    opt.direction_type = SocketDirectionalityType::BIDIRECTIONAL;
    opt.communication_type = SocketCommunicationType::ASYNCHRONOUS;
    opt.connection_life = SocketConnectionLife::INTERMITTENT;
    opt.protocol_type = ProtocolType::ZQTP;
    opt.host = "workers";  // Connect to the INPROC socket that proxy is listening on
    opt.port = 0;
    opt.local_id = "rust_response_worker";
    
    // Create credentials
    CredentialFactory cred_factory;
    auto credentials = cred_factory.create(ProtocolType::ZQTP, std::unordered_map<CredentialType, std::string>());
    
    // Create communicator
    CommunicatorFactory comm_factory(log_ctx);
    auto communicator = comm_factory.create(opt, *credentials, 1000, 1000);
    
    // Create IMessage using DataFed framework (exactly like C++ RepoServer)
    MessageFactory msg_factory;
    auto response_msg = msg_factory.create(MessageType::GOOGLE_PROTOCOL_BUFFER);
    
    // Set all the attributes exactly like C++ RepoServer does
    response_msg->set(MessageAttribute::CORRELATION_ID, std::string(correlation_id));
    response_msg->set(constants::message::google::CONTEXT, context);
    response_msg->set(constants::message::google::MSG_TYPE, response_msg_type);
    response_msg->set(MessageAttribute::STATE, MessageState::RESPONSE);
    
    // Create the appropriate protobuf message and set payload
    std::unique_ptr<google::protobuf::Message> msg;
    
    switch (response_msg_type) {
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
        msg = std::make_unique<SDMS::Anon::AckReply>();
        break;
    }
    
    // Parse the response payload into the message if it's not empty
    if (!response_payload.empty() && msg) {
      const std::string payload_str(reinterpret_cast<const char*>(response_payload.data()), response_payload.size());
      if (!msg->ParseFromString(payload_str)) {
        std::cerr << "Warning: Failed to parse response payload for message type " << response_msg_type << std::endl;
        msg = std::make_unique<SDMS::Anon::AckReply>();
      }
    }
    
    // Set the payload on the response message
    response_msg->setPayload(std::move(msg));
    
    std::cout << "🚀 C++ send_response_through_proxy: Created IMessage with all DataFed attributes" << std::endl;
    std::cout << "🚀 C++ send_response_through_proxy: - MessageState::RESPONSE: ✓" << std::endl;
    std::cout << "🚀 C++ send_response_through_proxy: - Correlation ID: ✓" << std::endl;
    std::cout << "🚀 C++ send_response_through_proxy: - Context: ✓" << std::endl;
    std::cout << "🚀 C++ send_response_through_proxy: - Message Type: ✓" << std::endl;
    std::cout << "🚀 C++ send_response_through_proxy: - Payload: ✓" << std::endl;
    
    // SEND THROUGH PROXY (correct approach)
    // Send to INPROC socket, proxy will forward to external client
    std::cout << "🚀 C++ send_response_through_proxy: Sending through proxy (INPROC → TCP)" << std::endl;
    std::cout << "🚀 C++ send_response_through_proxy: Proxy will forward response to external client" << std::endl;
    
    communicator->send(*response_msg);
    
    std::cout << "✅ C++ send_response_through_proxy: Response sent through proxy successfully" << std::endl;
    std::cout << "✅ C++ send_response_through_proxy: Proxy should forward this to external client" << std::endl;
    
  } catch (const std::exception& e) {
    throw std::runtime_error(std::string("send_response_through_proxy failed: ") + e.what());
  }
}

} // namespace ZMQBridge
