#[cxx::bridge]
mod ffi {
    unsafe extern "C++" {
        include!("sdms_dynalog_wrapper.hpp");

        #[namespace = "SDMS"]
        fn sdms_set_level(level: u32);

        #[namespace = "SDMS"]
        fn sdms_set_syslog(on: bool);

        #[namespace = "SDMS"]
        fn sdms_log_u32(
            level: u32,
            file: &str, func: &str, line: i32,
            thread_name: &str, correlation_id: &str, thread_id: i32,
            message: &str
        );

        #[namespace = "SDMS"]
        fn sdms_info(
            file: &str, func: &str, line: i32,
            thread_name: &str, correlation_id: &str, thread_id: i32,
            message: &str
        );
        #[namespace = "SDMS"]
        fn sdms_add_stdout_stream();
        #[namespace = "SDMS"]
        fn sdms_add_stderr_stream();
    }
}

// Re-export so callers use crate::ffi::dynalog::sdms_* directly
pub use ffi::*;

pub mod level {
    pub const CRITICAL: u32 = 0;
    pub const ERROR:    u32 = 1;
    pub const WARNING:  u32 = 2;
    pub const INFO:     u32 = 3;
    pub const DEBUG:    u32 = 4;
    pub const TRACE:    u32 = 5;
}

#[derive(Clone, Default)]
pub struct LogCtx {
    pub thread_name: String,
    pub correlation_id: String,
    pub thread_id: i32,
}

#[macro_export]
macro_rules! dl_info {
    ($ctx:expr, $($arg:tt)*) => {{
        $crate::ffi::dynalog::sdms_info(
            file!(), module_path!(), line!() as i32,
            &$ctx.thread_name, &$ctx.correlation_id, $ctx.thread_id,
            &format!($($arg)*)
        )
    }};
}

#[macro_export]
macro_rules! dl_log {
    ($level_u32:expr, $ctx:expr, $($arg:tt)*) => {{
        $crate::ffi::dynalog::sdms_log_u32(
            $level_u32,
            file!(), module_path!(), line!() as i32,
            &$ctx.thread_name, &$ctx.correlation_id, $ctx.thread_id,
            &format!($($arg)*)
        )
    }};
}
