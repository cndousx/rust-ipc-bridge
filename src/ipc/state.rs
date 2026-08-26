use interprocess::local_socket::Stream;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::thread::JoinHandle;

pub static SERVER_NEXT_ID: AtomicU64 = AtomicU64::new(1);
pub static CLIENT_NEXT_ID: AtomicU64 = AtomicU64::new(1);
pub static SERVER_RUNNING: AtomicBool = AtomicBool::new(false);

pub struct ClientConn {
    pub stream: Stream,
    pub recv_buf: Vec<u8>,
}

pub struct ServerState {
    pub clients: HashMap<u64, ClientConn>,
    pub listener_thread: Option<JoinHandle<()>>,
}

pub static SERVER: Lazy<Mutex<Option<ServerState>>> = Lazy::new(|| Mutex::new(None));
pub static CLIENTS: Lazy<Mutex<HashMap<u64, ClientConn>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
