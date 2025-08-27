#pragma once

#include "DynaLog.hpp"
#include "rust/cxx.h"

namespace SDMS {

// Safe conversion from u32 to LogLevel
inline LogLevel level_from_u32(unsigned int lvl) {
  if (lvl < static_cast<unsigned int>(LogLevel::LAST_SENTINEL)) {
    return static_cast<LogLevel>(lvl);
  }
  return LogLevel::INFO;
}

// Controls
void sdms_set_level(unsigned int level) noexcept;
void sdms_set_syslog(bool on) noexcept;

// Generic logging entrypoint
void sdms_log_u32(
    unsigned int level,
    rust::Str file, rust::Str func, int line,
    rust::Str thread_name, rust::Str correlation_id, int thread_id,
    rust::Str message);

// Convenience: INFO
void sdms_info(
    rust::Str file, rust::Str func, int line,
    rust::Str thread_name, rust::Str correlation_id, int thread_id,
    rust::Str message);

void sdms_add_stdout_stream() noexcept;
void sdms_add_stderr_stream() noexcept;

} // namespace SDMS
