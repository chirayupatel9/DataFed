use std::env;
use std::path::PathBuf;

fn main() {
    // Tell cargo to rerun this script if any of the proto files change
    println!("cargo:rerun-if-changed=proto/sdms.proto");
    println!("cargo:rerun-if-changed=proto/sdms_auth.proto");
    println!("cargo:rerun-if-changed=proto/sdms_anon.proto");
    println!("cargo:rerun-if-changed=proto/version.proto");

    // Generate Protocol Buffer code
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    
    // Configure prost to generate Protocol Buffer code
    let mut config = prost_build::Config::new();
    config.out_dir(&out_dir);
    
    // Generate Protocol Buffer code from .proto files
    config
        .compile_protos(
            &[
                "proto/sdms.proto",
                "proto/sdms_auth.proto", 
                "proto/sdms_anon.proto",
                "proto/version.proto"
            ],
            &["proto"],
        )
        .unwrap();
}
