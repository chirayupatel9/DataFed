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
    ])
    .include("include")
    .include("/mnt/storage/opt/datafed/dependencies/include")
    .include("/mnt/storage/opt/datafed/dependencies/lib")
    .include("/mnt/storage/opt/datafed/dependencies/bin")
    .include("/mnt/storage/opt/datafed/dependencies")

    .flag_if_supported("-std=c++17")
    .compile("repo_bridge");

    // .include("common")
    // .include("/mnt/storage/datafed_rs/DataFed/common/include")
    // .include("/mnt/storage/datafed_rs/DataFed/build/common/")
    // .include("common/include/common")
    // .include("include")
    // --- DataFed static libs from the build tree:
    println!("cargo:rustc-link-search=native=/mnt/storage/datafed_rs/DataFed/build/common");
    println!("cargo:rustc-link-lib=static=common");
    // println!("cargo:rustc-link-lib=static=datafed-protobuf"); // <- adjust to actual name if different
    println!("cargo:rustc-link-lib=zmq");

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
    println!("cargo:rustc-link-search=native=/opt/datafed/dependencies");
    println!("cargo:rustc-link-lib=protobuf");

    // // Group static libs (order-independent)
    // println!("cargo:rustc-link-arg=-Wl,--start-group");
    // println!("cargo:rustc-link-arg=-Wl,--end-group");

    // // Base dependencies

    // // Protobuf: use system .so and rpath
    // println!("cargo:rustc-link-search=native=/usr/lib/x86_64-linux-gnu");
    // println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/x86_64-linux-gnu");

    // // If your static libs were built against Abseil explicitly, keep these; otherwise you can drop them when protobuf is dynamic
    // println!("cargo:rustc-link-lib=absl_strings");
    // println!("cargo:rustc-link-lib=absl_base");
    // println!("cargo:rustc-link-lib=absl_raw_logging_internal");
    // println!("cargo:rustc-link-lib=absl_throw_delegate");

    // C++ stdlib
    #[cfg(target_os = "linux")]
    println!("cargo:rustc-link-lib=stdc++");

    // C++ stdlib
    #[cfg(target_os = "linux")]
    println!("cargo:rustc-link-lib=stdc++");
    #[cfg(target_os = "macos")]
    println!("cargo:rustc-link-lib=c++");
}

// #[allow(dead_code)]
// fn link_lib(name: &str, search_dir: &str) {
//     assert!(
//         Path::new(search_dir).exists(),
//         "Missing link dir: {search_dir}"
//     );
//     println!("cargo:rustc-link-search=native={search_dir}");
//     println!("cargo:rustc-link-lib={name}");
// }
