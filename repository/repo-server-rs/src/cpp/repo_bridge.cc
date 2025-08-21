// Pull in the generated definitions first (this defines RepoBridge::VersionInfo).
#include "repo-server-rs/src/ffi/repo.rs.h"

// Then your own project header with the prototype.
#include "repo_bridge.hpp"

#include <random>
#include <string>
#include <memory>

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
#include "common/KeyGenerator.hpp"


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
  const std::string scheme_s(scheme);
  const std::string core_pub_s(core_public_key);

  // 1) Make a transient keypair and set the remote server’s public key
  KeyGenerator generator;
  auto local_keys = generator.generate(ProtocolType::ZQTP, KeyType::PUBLIC_PRIVATE);
  local_keys[CredentialType::SERVER_KEY] = core_pub_s;

  CredentialFactory cred_factory;
  auto local_sec_ctx = cred_factory.create(ProtocolType::ZQTP, local_keys);

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

  auto* payload = std::get<google::protobuf::Message*>(resp.message->getPayload());
  auto* ver     = dynamic_cast<SDMS::Anon::VersionReply*>(payload);
  if (!ver) {
    throw std::runtime_error("invalid payload (not VersionReply)");
  }

  ::VersionInfo out{};
  out.release_year     = ver->release_year();
  out.release_month    = ver->release_month();
  out.release_day      = ver->release_day();
  out.release_hour     = ver->release_hour();
  out.release_minute   = ver->release_minute();
  out.api_major        = ver->api_major();
  out.api_minor        = ver->api_minor();
  out.api_patch        = ver->api_patch();
  out.component_major  = ver->component_major();
  out.component_minor  = ver->component_minor();
  out.component_patch  = ver->component_patch();
  return out;
}

} // namespace RepoBridge

// #include "repo-server-rs/src/ffi/repo.rs.h" 
// #include "repo_bridge.hpp"
// #include <random>

// // Try a few common names so your build works across trees:
// #if __has_include("common/KeyGenerator.hpp")
//   #include "common/KeyGenerator.hpp"       // SDMS::KeyGenerator, SDMS::KeyType
// #elif __has_include("common/Crypto/KeyGenerator.hpp")
//   #include "common/Crypto/KeyGenerator.hpp"
// #endif

// #if __has_include("common/CredentialFactory.hpp")
//   #include "common/CredentialFactory.hpp"    // SDMS::CredentialType
// #endif

// #if __has_include("common/SocketOptions.hpp")
//   #include "common/SocketOptions.hpp"    // SDMS::AddressSplitter
// #endif
// using namespace SDMS;

// namespace {
// std::string random_id() {
//   static const char* chars = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
//   std::random_device rd; std::mt19937 gen(rd()); std::uniform_int_distribution<> d(0,61);
//   std::string s = "version-client-"; for (int i=0;i<8;++i) s.push_back(chars[d(gen)]); return s;
// }
// } // anonymous

// namespace RepoBridge {

// ::VersionInfo send_version_request(rust::Str host,
//                                  rust::Str port,
//                                  rust::Str scheme,
//                                  rust::Str core_public_key,
//                                  uint32_t timeout_ms) {
//   LogContext log_ctx; // optional: fill thread_name/correlation_id if you like

//   // 1) Make a transient keypair and set the remote server's public key
//   KeyGenerator generator;
//   auto local_keys = generator.generate(ProtocolType::ZQTP, KeyType::PUBLIC_PRIVATE);
//   local_keys[CredentialType::SERVER_KEY] = std::string(core_public_key);

//   CredentialFactory cred_factory;
//   auto local_sec_ctx = cred_factory.create(ProtocolType::ZQTP, local_keys);

//   // 2) Build a secure client communicator for core_addr
//   // SDMS::AddressSplitter splitter(std::string(host), std::string(port), std::string(scheme));
//   SocketOptions opt;
//   opt.scheme = scheme;
//   opt.class_type = SocketClassType::CLIENT;
//   opt.direction_type = SocketDirectionalityType::BIDIRECTIONAL;
//   opt.communication_type = SocketCommunicationType::ASYNCHRONOUS;
//   opt.connection_life = SocketConnectionLife::INTERMITTENT;
//   opt.protocol_type = ProtocolType::ZQTP;
//   opt.connection_security = SocketConnectionSecurity::SECURE;
//   opt.host = host;
//   opt.port = port;
//   opt.local_id = random_id();

//   SDMS::CommunicatorFactory comm_factory(log_ctx);
//   auto client = comm_factory.create(opt, *local_sec_ctx, timeout_ms, timeout_ms);

//   // 3) Build + send VersionRequest
//   SDMS::MessageFactory msg_factory(log_ctx);
//   auto msg = std::make_unique<SDMS::VersionRequest>();
//   auto envelope = msg_factory.create(MessageType::GOOGLE_PROTOCOL_BUFFER);
//   envelope->setPayload(std::move(msg));
//   envelope->set(MessageAttribute::KEY, local_sec_ctx->get(CredentialType::PUBLIC_KEY));
//   client->send(*envelope);

//   // 4) Receive + parse VersionReply
//   auto resp = client->receive(MessageType::GOOGLE_PROTOCOL_BUFFER);
//   if (resp.time_out)             throw rust::Error("timeout waiting for VersionReply");
//   if (resp.error)                throw rust::Error(std::string("error: ") + resp.error_msg);

//   auto payload = std::get<google::protobuf::Message*>(resp.message->getPayload());
//   auto* ver = dynamic_cast<SDMS::VersionReply*>(payload);
//   if (!ver)                      throw rust::Error("invalid payload (not VersionReply)");

//   ::VersionInfo out{};
//   out.release_year   = ver->release_year();
//   out.release_month  = ver->release_month();
//   out.release_day    = ver->release_day();
//   out.release_hour   = ver->release_hour();
//   out.release_minute = ver->release_minute();
//   out.api_major      = ver->api_major();
//   out.api_minor      = ver->api_minor();
//   out.api_patch      = ver->api_patch();
//   out.component_major= ver->component_major();
//   out.component_minor= ver->component_minor();
//   out.component_patch= ver->component_patch();
//   return out;
// }

// } // namespace RepoBridge

// // #include "repo-server-rs/src/ffi/repo.rs.h"  // cxx-generated header for VersionInfo mapping
// // #include "repo_bridge.hpp"
// // #include <cstdint>



// // namespace RepoBridge {

// //   static constexpr std::uint32_t kMaj = 1;
// //   static constexpr std::uint32_t kMin = 0;
// //   static constexpr std::uint32_t kPat = 1;
  
// //   ::VersionInfo send_version_request(::rust::Str /*core_addr*/,
// //                                      ::rust::Str /*core_pub_key*/,
// //                                      std::uint32_t /*creds_mask*/) {
// //     // Works even if the field names differ: aggregate-init in the declared order.
// //     ::VersionInfo vi{ kMaj, kMin, kPat };
// //     return vi;
// //   }
// // } // namespace RepoBridge
