#include "sdms_dynalog_wrapper.hpp"

namespace SDMS {

static inline LogContext make_ctx(rust::Str tn, rust::Str cid, int tid) {
  LogContext ctx;
  ctx.thread_name    = std::string(tn);
  ctx.correlation_id = std::string(cid);
  ctx.thread_id      = tid;
  return ctx;
}

void sdms_set_level(unsigned int level) noexcept {
  global_logger.setLevel(level_from_u32(level));
}

void sdms_set_syslog(bool on) noexcept {
  global_logger.setSysLog(on);
}

void sdms_log_u32(
    unsigned int level,
    rust::Str file, rust::Str func, int line,
    rust::Str tn, rust::Str cid, int tid,
    rust::Str msg) {

  auto ctx = make_ctx(tn, cid, tid);
  global_logger.log(level_from_u32(level),
                    std::string(file),
                    std::string(func),
                    line,
                    ctx,
                    std::string(msg));
}

void sdms_info(
    rust::Str file, rust::Str func, int line,
    rust::Str tn, rust::Str cid, int tid,
    rust::Str msg) {

  auto ctx = make_ctx(tn, cid, tid);
  global_logger.info(std::string(file),
                     std::string(func),
                     line,
                     ctx,
                     std::string(msg));
}
void sdms_add_stdout_stream() noexcept {
    global_logger.addStream(std::cout);
  }
  void sdms_add_stderr_stream() noexcept {
    global_logger.addStream(std::cerr);
  }

} // namespace SDMS
