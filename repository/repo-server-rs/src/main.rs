mod server;
mod request_worker;
mod config;
mod ffi;
mod version;

use clap::{Arg, Command};
use ffi::{repo_log_info, repo_set_log_defaults, CxxLogContext};
// use std::{fs::File, io::Write, path::PathBuf};

// fn gen_keys(cred_dir: &str) -> anyhow::Result<()> {
//     let pub_key = "4X1hKiU5pdwdk.s7&=Q2(b1]p!^Nj=Dnk2&7vh@f";
//     let priv_key = "G1DpacgVoCcRmLYQ6PA8:Q$]/w5SE*Qm?)}L!@Gv";
//     std::fs::create_dir_all(cred_dir)?;
//     let mut f = File::create(PathBuf::from(cred_dir).join("mock-datafed-core-key.pub"))?;
//     write!(f, "{}", pub_key)?;
//     let mut f = File::create(PathBuf::from(cred_dir).join("mock-datafed-core-key.priv"))?;
//     write!(f, "{}", priv_key)?;
//     Ok(())
// }

fn main() -> anyhow::Result<()> {
    repo_set_log_defaults();
    let log = CxxLogContext { thread_name: "repo_server".into(), thread_id: 0, correlation_id: String::new() };

    let m = Command::new("datafed-repo")
        .about("DataFed Repo Server (Rust)")
        .arg(Arg::new("cred-dir").short('c').long("cred-dir").value_name("PATH").required(false))
        .arg(Arg::new("port").short('p').long("port").value_parser(clap::value_parser!(u16)))
        .arg(Arg::new("server").short('s').long("server").help("Core server address").value_name("ADDR"))
        .arg(Arg::new("globus-collection-path").short('g').long("globus-collection-path").value_name("PATH"))
        .arg(Arg::new("threads").short('t').long("threads").value_parser(clap::value_parser!(u32)))
        .arg(Arg::new("cfg").long("cfg").value_name("FILE"))
        // .arg(Arg::new("gen-keys").long("gen-keys").action(clap::ArgAction::SetTrue))
        .arg(Arg::new("version").short('v').long("version").action(clap::ArgAction::SetTrue))
        .get_matches();

    let mut cred_dir = m.get_one::<String>("cred-dir").cloned().unwrap_or_else(|| "/mnt/storage/rust/DataFed".into());
    if !cred_dir.is_empty() && !cred_dir.ends_with('/') { cred_dir.push('/'); }

    let mut globus_path = m.get_one::<String>("globus-collection-path").cloned().unwrap_or_else(|| "/mnt/datafed-repo".into());
    while globus_path.ends_with('/') && globus_path.len() > 1 { globus_path.pop(); }

    let port = *m.get_one::<u16>("port").unwrap_or(&1340);
    let core_server = m.get_one::<String>("server").cloned().unwrap_or_else(|| "tcp://localhost:9999".into());
    let num_threads = *m.get_one::<u32>("threads").unwrap_or(&2);

    if m.get_flag("version") {
        println!("Repo Server: {}.{}.{}", 0, 1, 0);
        return Ok(());
    }
    // if m.get_flag("gen-keys") {
    //     gen_keys(&cred_dir)?;
    //     return Ok(());
    // }

    config::Config::init(config::Config {
        cred_dir,
        port,
        core_server,
        globus_collection_path: globus_path,
        num_req_worker_threads: num_threads,
    });

    repo_log_info(&log.thread_name, &log.correlation_id, log.thread_id, &format!("DataFed repo server starting, ver {}.{}.{}", 0, 1, 0));
    server::Server::new(log).run();
    Ok(())
}