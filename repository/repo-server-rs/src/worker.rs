use std::{
    io,
    path::PathBuf,
    sync::{Arc, atomic::{AtomicBool, Ordering}},
    thread,
    time::Duration,
};

// ===== Envelope + Transport (kept minimal so it works today) =====
#[derive(Debug, Clone)]
pub struct Envelope {
    pub correlation_id: String,
    pub msg_type: u16,
    pub payload: Vec<u8>,
    pub key: Option<String>,
}

pub trait Messenger: Send + Sync + 'static {
    fn recv(&self, _timeout_ms: i32) -> io::Result<Option<Envelope>> { Ok(None) }
    fn send(&self, _env: Envelope) -> io::Result<()> { Ok(()) }
}

/// Placeholder messenger so the worker can run idle until you plug in inproc.
#[derive(Clone, Default)]
pub struct NoopMessenger;
impl Messenger for NoopMessenger {}

// ===== Message type IDs (adjust to your real mapper when you wire proto) =====
const MT_VERSION_REQUEST: u16            = 1;
const MT_VERSION_REPLY: u16              = 2;
const MT_REPO_DATA_DELETE_REQUEST: u16   = 1001;
const MT_REPO_DATA_GET_SIZE_REQUEST: u16 = 1002;
const MT_REPO_PATH_CREATE_REQUEST: u16   = 1003;
const MT_REPO_PATH_DELETE_REQUEST: u16   = 1004;
// simple acks/nacks for now
const MT_ACK_REPLY: u16                  = 1100;
const MT_NACK_REPLY: u16                 = 9999;

/// Spawn one worker thread. Replace `NoopMessenger` with your real inproc client later.
pub fn spawn_worker<M: Messenger + Clone + 'static>(
    id: usize,
    messenger: M,
    base_path: impl Into<PathBuf>,
    running: Arc<AtomicBool>,
    _repo_semver: (u32, u32, u32),
    _api_version: (u32, u32),
) -> std::thread::JoinHandle<()> {
    let base_root = base_path.into();

    thread::spawn(move || {
        while running.load(Ordering::Relaxed) {
            match messenger.recv(10) {
                Ok(Some(env)) => {
                    // --- decode, dispatch, reply (DONE) ---
                    let correlation_id = env.correlation_id.clone();

                    // Route by message type. For now:
                    // - VersionRequest -> VersionReply (empty payload stub)
                    // - Repo* requests  -> AckReply (empty payload stub)
                    // - Unknown         -> NackReply (json payload with error)
                    let (reply_type, payload) = match env.msg_type {
                        MT_VERSION_REQUEST => {
                            // TODO (later): fill with real VersionReply bytes (prost)
                            (MT_VERSION_REPLY, Vec::new())
                        }
                        MT_REPO_DATA_DELETE_REQUEST => {
                            // TODO (later): parse env.payload to paths and delete under base_root
                            let _ = &base_root;
                            (MT_ACK_REPLY, Vec::new())
                        }
                        MT_REPO_DATA_GET_SIZE_REQUEST => {
                            // TODO (later): compute sizes and return real reply
                            (MT_ACK_REPLY, Vec::new())
                        }
                        MT_REPO_PATH_CREATE_REQUEST => {
                            // TODO (later): parse path and mkdir -p under base_root
                            (MT_ACK_REPLY, Vec::new())
                        }
                        MT_REPO_PATH_DELETE_REQUEST => {
                            // TODO (later): parse path and rm -r under base_root
                            (MT_ACK_REPLY, Vec::new())
                        }
                        _ => {
                            let body = encode_nack(-2, format!("unsupported msg type {}", env.msg_type));
                            (MT_NACK_REPLY, body)
                        }
                    };

                    // Send reply (preserve correlation id)
                    let _ = messenger.send(Envelope {
                        correlation_id,
                        msg_type: reply_type,
                        payload,
                        key: None,
                    });
                }
                Ok(None) => { /* poll timeout */ }
                Err(e) => {
                    eprintln!("worker[{id}] recv error: {e}");
                    thread::sleep(Duration::from_millis(5));
                }
            }
        }
        eprintln!("worker[{id}] exiting");
    })
}

// simple JSON NACK payload for now (so you can inspect in tests/logs)
fn encode_nack(code: i32, msg: String) -> Vec<u8> {
    format!(r#"{{"err_code":{},"err_msg":"{}"}}"#, code, msg.replace('"', "'")).into_bytes()
}
