mod ffi;

use ffi::dynalog::{self, LogCtx};
use ffi::dynalog::level;
use ffi::repo::send_version_request;

fn main() {

    dynalog::sdms_add_stdout_stream();
    dynalog::sdms_add_stderr_stream();


    // Configure SDMS logger
    dynalog::sdms_set_level(level::INFO);
    dynalog::sdms_set_syslog(false);

    // Build a context
    let ctx = LogCtx {
        thread_name: std::thread::current().name().unwrap_or("main").to_string(),
        correlation_id: "abc-123".to_string(),
        thread_id: std::process::id() as i32,
    };
    let addr = "tcp://localhost:9998";        // whatever your core address is
    let parts: Vec<&str> = addr.split("://").collect();
    let scheme = parts[0];
    let host_port = parts[1];

    let hp_parts: Vec<&str> = host_port.split(':').collect();
    let host = hp_parts[0];
    let port: u16 = hp_parts[1].parse().unwrap();

    let core_pub = "4X1hKiU5pdwdk.s7&=Q2(b1]p!^Nj=Dnk2&7vh@f"; // from your config

    let vi = send_version_request(host, port, scheme, core_pub, 20_000).expect("version query failed");

    dl_info!(ctx, "Rust app starting up");
    // dl_info!(ctx, "{:?}", vi);
    // dl_info!(
    //     ctx,
    //     "Core API {}.{}.{} | Repo component {}.{}.{} | Release {}-{:02}-{:02} {:02}:{:02}",
    //     vi.major, vi.minor, vi.patch,
    //     vi.component_major, vi.component_minor, vi.component_patch,
    //     vi.release_year, vi.release_month, vi.release_day,
    //     vi.release_hour, vi.release_minute
    // );


    println!("Scheme: {}", scheme);
    println!("Host: {}", host);
    println!("Port: {}", port);

    dl_log!(level::DEBUG, ctx, "Debug step {}", 1);
    dl_info!(ctx, "All done 🎉");
    dl_info!(ctx, "All done 🎉");
}
