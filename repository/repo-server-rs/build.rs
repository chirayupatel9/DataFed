use std::path::Path;

fn main() {
    cxx_build::bridges(&[
        "src/ffi/dynalog.rs", // logging bridge
        "src/ffi/repo.rs",    // send_version_request bridge
    ])
    .files([
        "src/cpp/sdms_dynalog_wrapper.cc",
        "src/cpp/DynaLog.cpp",
        "src/cpp/repo_bridge.cc",
        "src/cpp/server_bridge.cc",
    ])
    .include("/opt/datafed/dependencies")
    .include("include")
    .include("/opt/datafed/dependencies/include")
    .include("/opt/datafed/dependencies/lib")
    .include("/opt/datafed/dependencies/bin")

    .flag_if_supported("-std=c++17")
    .compile("repo_bridge");

    // .include("common")
    // .include("/mnt/storage/datafed_rs/DataFed/common/include")
    // .include("/mnt/storage/datafed_rs/DataFed/build/common/")
    // .include("common/include/common")
    // .include("include")
    // --- DataFed static libs from the build tree:
    //
    println!("cargo:rustc-link-search=native=/mnt/storage/datafed_rs/DataFed/build/common");
    println!("cargo:rustc-link-lib=static=common");
    // println!("cargo:rustc-link-lib=static=datafed-protobuf"); // <- adjust to actual name if different
    println!("cargo:rustc-link-search=native=/opt/datafed/dependencies/lib");
    println!("cargo:rustc-link-lib=static=zmq");

    //    println!("cargo:rustc-link-search=native=/mnt/storage/datafed_rs/DataFed/build/common/proto/common");
    //    println!("cargo:rustc-link-lib=datafed-protobuf");
    // Search paths for your static libs
    // println!("cargo:rustc-link-search=native=/mnt/storage/datafed_rs/DataFed/build/common");
    // println!(
    //     "cargo:rustc-link-search=native=/mnt/storage/datafed_rs/DataFed/build/common/proto/common"
    // );
    println!("cargo:rerun-if-changed=/mnt/storage/datafed_rs/DataFed/repository/repo-server-rs/include/common/SDMS.pb.h");
    println!("cargo:rerun-if-changed=/mnt/storage/datafed_rs/DataFed/repository/repo-server-rs/include/common/SDMS_Anon.pb.h");
    println!("cargo:rerun-if-changed=/mnt/storage/datafed_rs/DataFed/repository/repo-server-rs/include/common/SDMS_Auth.pb.h");
    println!("cargo:rerun-if-changed=/mnt/storage/datafed_rs/DataFed/repository/repo-server-rs/include/common/Version.pb.h");

    // Group order-independent static libs
    println!("cargo:rustc-link-arg=-Wl,--start-group");
    println!("cargo:rustc-link-search=native=/mnt/storage/datafed_rs/DataFed/build/common/proto/common");
    println!("cargo:rustc-link-lib=static=datafed-protobuf"); 
    println!("cargo:rustc-link-arg=-Wl,--end-group");
    
    println!("cargo:rustc-link-search=native=/mnt/storage/datafed_rs/DataFed/build/common/proto/common");
    println!("cargo:rustc-link-lib=protobuf");
    println!("cargo:rustc-link-lib=protobuf-lite");
    println!("cargo:rustc-link-lib=z");
    println!("cargo:rustc-link-lib=utf8_range");
    println!("cargo:rustc-link-lib=utf8_validity");

    // // Group static libs (order-independent)
    // println!("cargo:rustc-link-arg=-Wl,--start-group");
    // println!("cargo:rustc-link-arg=-Wl,--end-group");

    // // Base dependencies

    // // Protobuf: use system .so and rpath
    // println!("cargo:rustc-link-search=native=/usr/lib/x86_64-linux-gnu");
    // println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/x86_64-linux-gnu");

    // // If your static libs were built against Abseil explicitly, keep these; otherwise you can drop them when protobuf is dynamic
    println!("cargo:rustc-link-lib=crypto");
    println!("cargo:rustc-link-lib=sodium");

   // High-level libraries
    println!("cargo:rustc-link-lib=static=absl_flags_parse");
    println!("cargo:rustc-link-lib=static=absl_flags_usage");
    println!("cargo:rustc-link-lib=static=absl_flags_usage_internal");
    println!("cargo:rustc-link-lib=static=absl_failure_signal_handler");
    
    // Flags libraries
    println!("cargo:rustc-link-lib=static=absl_flags");
    println!("cargo:rustc-link-lib=static=absl_flags_internal");
    println!("cargo:rustc-link-lib=static=absl_flags_reflection");
    println!("cargo:rustc-link-lib=static=absl_flags_private_handle_accessor");
    println!("cargo:rustc-link-lib=static=absl_flags_commandlineflag");
    println!("cargo:rustc-link-lib=static=absl_flags_commandlineflag_internal");
    println!("cargo:rustc-link-lib=static=absl_flags_config");
    println!("cargo:rustc-link-lib=static=absl_flags_program_name");
    println!("cargo:rustc-link-lib=static=absl_flags_marshalling");
    
    // Logging libraries
    println!("cargo:rustc-link-arg=-Wl,--start-group");
    println!("cargo:rustc-link-lib=static=absl_log_initialize");
    println!("cargo:rustc-link-lib=static=absl_log_flags");
    println!("cargo:rustc-link-lib=static=absl_log_globals");
    println!("cargo:rustc-link-lib=static=absl_log_internal_message");
    println!("cargo:rustc-link-lib=static=absl_log_internal_log_sink_set");
    println!("cargo:rustc-link-lib=static=absl_log_internal_format");
    println!("cargo:rustc-link-lib=static=absl_log_internal_proto");
    println!("cargo:rustc-link-lib=static=absl_log_internal_globals");
    println!("cargo:rustc-link-lib=static=absl_log_internal_nullguard");
    println!("cargo:rustc-link-lib=static=absl_log_internal_check_op");
    println!("cargo:rustc-link-lib=static=absl_log_internal_conditions");
    println!("cargo:rustc-link-lib=static=absl_log_entry");
    println!("cargo:rustc-link-lib=static=absl_log_sink");
    println!("cargo:rustc-link-lib=static=absl_die_if_null");
    println!("cargo:rustc-link-lib=static=absl_log_globals");
    println!("cargo:rustc-link-arg=-Wl,--end-group");
    
    // Cord libraries
    println!("cargo:rustc-link-lib=static=absl_cord");
    println!("cargo:rustc-link-lib=static=absl_cord_internal");
    println!("cargo:rustc-link-lib=static=absl_cordz_info");
    println!("cargo:rustc-link-lib=static=absl_cordz_sample_token");
    println!("cargo:rustc-link-lib=static=absl_cordz_handle");
    println!("cargo:rustc-link-lib=static=absl_cordz_functions");
    println!("cargo:rustc-link-lib=static=absl_crc_cord_state");
    
    // Status libraries
    println!("cargo:rustc-link-lib=static=absl_statusor");
    println!("cargo:rustc-link-lib=static=absl_status");
    
    // Synchronization
    println!("cargo:rustc-link-lib=static=absl_synchronization");
    println!("cargo:rustc-link-lib=static=absl_graphcycles_internal");
    println!("cargo:rustc-link-lib=static=absl_kernel_timeout_internal");
    
    // Random libraries
    println!("cargo:rustc-link-lib=static=absl_random_distributions");
    println!("cargo:rustc-link-lib=static=absl_random_seed_sequences");
    println!("cargo:rustc-link-lib=static=absl_random_internal_pool_urbg");
    println!("cargo:rustc-link-lib=static=absl_random_internal_seed_material");
    println!("cargo:rustc-link-lib=static=absl_random_internal_randen");
    println!("cargo:rustc-link-lib=static=absl_random_internal_randen_hwaes");
    println!("cargo:rustc-link-lib=static=absl_random_internal_randen_hwaes_impl");
    println!("cargo:rustc-link-lib=static=absl_random_internal_randen_slow");
    println!("cargo:rustc-link-lib=static=absl_random_internal_platform");
    println!("cargo:rustc-link-lib=static=absl_random_seed_gen_exception");
    println!("cargo:rustc-link-lib=static=absl_random_internal_distribution_test_util");
    println!("cargo:rustc-link-lib=static=absl_periodic_sampler");
    
    // Hash libraries
    println!("cargo:rustc-link-lib=static=absl_hash");
    println!("cargo:rustc-link-lib=static=absl_city");
    println!("cargo:rustc-link-lib=static=absl_low_level_hash");
    println!("cargo:rustc-link-lib=static=absl_raw_hash_set");
    println!("cargo:rustc-link-lib=static=absl_hashtablez_sampler");
    
    // Debugging libraries
    println!("cargo:rustc-link-lib=static=absl_examine_stack");
    println!("cargo:rustc-link-lib=static=absl_symbolize");
    println!("cargo:rustc-link-lib=static=absl_stacktrace");
    println!("cargo:rustc-link-lib=static=absl_debugging_internal");
    println!("cargo:rustc-link-lib=static=absl_demangle_internal");
    println!("cargo:rustc-link-lib=static=absl_leak_check");
    
    // String libraries
    println!("cargo:rustc-link-lib=static=absl_str_format_internal");
    println!("cargo:rustc-link-lib=static=absl_strings");
    println!("cargo:rustc-link-lib=static=absl_strings_internal");
    println!("cargo:rustc-link-lib=static=absl_string_view");
    
    // Time libraries
    println!("cargo:rustc-link-lib=static=absl_time");
    println!("cargo:rustc-link-lib=static=absl_time_zone");
    println!("cargo:rustc-link-lib=static=absl_civil_time");
    
    // CRC libraries
    println!("cargo:rustc-link-lib=static=absl_crc32c");
    println!("cargo:rustc-link-lib=static=absl_crc_internal");
    println!("cargo:rustc-link-lib=static=absl_crc_cpu_detect");
    
    // Numeric libraries
    println!("cargo:rustc-link-lib=static=absl_int128");
    
    // Type/variant libraries
    println!("cargo:rustc-link-lib=static=absl_bad_any_cast_impl");
    println!("cargo:rustc-link-lib=static=absl_bad_optional_access");
    println!("cargo:rustc-link-lib=static=absl_bad_variant_access");
    
    // Utility libraries
    println!("cargo:rustc-link-lib=static=absl_exponential_biased");
    println!("cargo:rustc-link-lib=static=absl_scoped_set_env");
    println!("cargo:rustc-link-lib=static=absl_strerror");
    
    // Base/core libraries (least dependent)
    println!("cargo:rustc-link-lib=static=absl_throw_delegate");
    println!("cargo:rustc-link-lib=static=absl_malloc_internal");
    println!("cargo:rustc-link-lib=static=absl_base");
    println!("cargo:rustc-link-lib=static=absl_spinlock_wait");
    println!("cargo:rustc-link-lib=static=absl_raw_logging_internal");
    println!("cargo:rustc-link-lib=static=absl_log_severity");

    // C++ stdlib
    #[cfg(target_os = "linux")]
    println!("cargo:rustc-link-lib=stdc++");

    // C++ stdlib
    #[cfg(target_os = "linux")]
    println!("cargo:rustc-link-lib=stdc++");
    #[cfg(target_os = "macos")]
    println!("cargo:rustc-link-lib=c++");
}
