// build.rs
fn main() {
    // Compile the cxx bridge (Rust<->C++) and our glue .cc
    cxx_build::bridge("src/ffi.rs")
        .file("cxx/bridge.cc")
        // Make sure the compiler can find DataFed C++ headers + our cxx dir
        .include("/mnt/storage/rust/DataFed/common/include")
        .include("/mnt/storage/rust/DataFed/build")
        .include("cxx")
        .flag_if_supported("-std=c++17")
        .compile("repo_server_ffi");

    // Where the linker will look for DataFed + deps
    for dir in &[
        "/mnt/storage/rust/DataFed/build/common",
        "/mnt/storage/rust/DataFed/build/common/proto/common",
        "/mnt/storage/opt/datafed/dependencies/lib",
        "/usr/local/lib",
    ] {
        println!("cargo:rustc-link-search=native={dir}");
    }

    // Link DataFed static libs built by CMake
    println!("cargo:rustc-link-lib=static=common");
    println!("cargo:rustc-link-lib=static=datafed-protobuf");
    // And our cxx object from above
    println!("cargo:rustc-link-lib=static=repo_server_ffi");

    // Abseil pieces required by protobuf 25.x (static, present in your deps lib dir)
    for lib in &[
        "absl_log_entry","absl_log_flags","absl_log_globals","absl_log_initialize",
        "absl_log_internal_check_op","absl_log_internal_conditions","absl_log_internal_format","absl_log_internal_globals",
        "absl_log_internal_log_sink_set","absl_log_internal_message","absl_log_internal_nullguard","absl_log_internal_proto",
        "absl_log_severity","absl_log_sink","absl_raw_logging_internal","absl_strings","absl_strings_internal",
        "absl_str_format_internal","absl_status","absl_statusor","absl_time","absl_time_zone","absl_raw_hash_set",
        "absl_hash","absl_city","absl_low_level_hash","absl_int128","absl_synchronization","absl_spinlock_wait",
        "absl_kernel_timeout_internal","absl_stacktrace","absl_symbolize","absl_debugging_internal","absl_graphcycles_internal",
        "absl_demangle_internal","absl_examine_stack","absl_cord","absl_cord_internal","absl_cordz_functions","absl_cordz_handle",
        "absl_cordz_info","absl_cordz_sample_token","absl_malloc_internal","absl_base","absl_periodic_sampler",
        "absl_scoped_set_env","absl_exponential_biased","absl_crc32c","absl_crc_cord_state","absl_crc_cpu_detect",
        "absl_crc_internal","absl_die_if_null","absl_throw_delegate","utf8_range","utf8_validity",
    ] {
        println!("cargo:rustc-link-lib=static={lib}");
    }

    // Dynamic libs present in your deps dir
    for lib in &["protobuf", "zmq", "stdc++", "ssl", "crypto", "z", "sodium"] {
        println!("cargo:rustc-link-lib=dylib={lib}");
    }

    // Rebuild when bridge files change
    println!("cargo:rerun-if-changed=src/ffi.rs");
    println!("cargo:rerun-if-changed=cxx/bridge.cc");

    // Optional: generate a tiny version file at build time
    let _ = std::fs::write(
        "src/version.rs",
        "pub const MAJOR:i32=0; pub const MINOR:i32=1; pub const PATCH:i32=0;",
    );
}
