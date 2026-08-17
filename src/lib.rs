use interprocess::local_socket::{
    GenericNamespaced, ListenerNonblockingMode, ListenerOptions, Name, Stream, prelude::*,
};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::io::{self, BufRead, BufReader, Write};
use std::os::raw::{c_char, c_int};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

// ===================== 全局状态 =====================

static NEXT_CLIENT_ID: AtomicU64 = AtomicU64::new(1);
static SERVER_RUNNING: AtomicBool = AtomicBool::new(false);

struct ServerState {
    clients: HashMap<u64, Stream>,
    listener_thread: Option<JoinHandle<()>>,
}

static SERVER: Lazy<Mutex<Option<ServerState>>> = Lazy::new(|| Mutex::new(None));
static CLIENTS: Lazy<Mutex<HashMap<u64, Stream>>> = Lazy::new(|| Mutex::new(HashMap::new()));

// ===================== 工具函数 =====================

/// 返回的字符串必须用 [`ipc_free_string`]  释放
fn to_c_string(s: &str) -> *mut c_char {
    CString::new(s)
        .map(|c| c.into_raw())
        .unwrap_or(std::ptr::null_mut())
}

fn from_c_str(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    unsafe { CStr::from_ptr(ptr) }
        .to_str()
        .ok()
        .map(|s| s.to_owned())
}

fn make_name(name: &str) -> Result<Name<'static>, String> {
    name.to_ns_name::<GenericNamespaced>()
        .map_err(|e| e.to_string())
        .map(|n| n.into_owned())
}

// ===================== 服务端 API =====================

/// 启动服务端（非阻塞，后台线程监听）
/// 返回值：0 成功，-1 通用错误，-2 参数错误，-4 已在运行
#[unsafe(no_mangle)]
pub extern "C" fn ipc_server_start(name: *const c_char) -> c_int {
    if SERVER_RUNNING.load(Ordering::SeqCst) {
        return -4;
    }

    let name_str = match from_c_str(name) {
        Some(s) => s,
        None => return -2,
    };

    let socket_name = match make_name(&name_str) {
        Ok(n) => n,
        Err(_) => return -1,
    };

    // 使用非阻塞 accept，避免 stop 时卡死
    let listener = match ListenerOptions::new()
        .name(socket_name)
        .nonblocking(ListenerNonblockingMode::Accept)
        .create_sync()
    {
        Ok(l) => l,
        Err(_) => return -1,
    };

    SERVER_RUNNING.store(true, Ordering::SeqCst);

    let handle = thread::spawn(move || {
        println!("[ipc_lib] 服务端监听线程启动");

        while SERVER_RUNNING.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok(stream) => {
                    let id = NEXT_CLIENT_ID.fetch_add(1, Ordering::SeqCst);
                    if let Ok(mut guard) = SERVER.lock() {
                        if let Some(state) = guard.as_mut() {
                            state.clients.insert(id, stream);
                            println!("[ipc_lib] 新客户端连接, id={}", id);
                        }
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    // 没有新连接，休眠一段时间再检查
                    thread::sleep(Duration::from_millis(200));
                }
                Err(e) => {
                    eprintln!("[ipc_lib] accept 错误: {e}");
                    thread::sleep(Duration::from_millis(200));
                }
            }
        }

        println!("[ipc_lib] 服务端监听线程退出");
    });

    let mut guard = SERVER.lock().unwrap();
    *guard = Some(ServerState {
        clients: HashMap::new(),
        listener_thread: Some(handle),
    });

    0
}

/// 停止服务端
#[unsafe(no_mangle)]
pub extern "C" fn ipc_server_stop() -> c_int {
    if !SERVER_RUNNING.swap(false, Ordering::SeqCst) {
        return -3; // 本来就没运行
    }

    let mut guard = SERVER.lock().unwrap();
    if let Some(mut state) = guard.take() {
        state.clients.clear();
        if let Some(handle) = state.listener_thread.take() {
            // 现在最多只会阻塞约 200ms
            let _ = handle.join();
        }
    }

    0
}

/// 向指定客户端发送消息
#[unsafe(no_mangle)]
pub extern "C" fn ipc_server_send(client_id: u64, data: *const c_char) -> c_int {
    let msg = match from_c_str(data) {
        Some(s) => s,
        None => return -2,
    };

    let mut guard = match SERVER.lock() {
        Ok(g) => g,
        Err(_) => return -1,
    };

    let state = match guard.as_mut() {
        Some(s) => s,
        None => return -3,
    };

    let stream = match state.clients.get_mut(&client_id) {
        Some(s) => s,
        None => return -3,
    };

    let line = format!("{}\n", msg);
    match stream
        .write_all(line.as_bytes())
        .and_then(|_| stream.flush())
    {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// 从指定客户端读取一行（阻塞）
/// 返回的字符串必须用 [`ipc_free_string`]  释放
#[unsafe(no_mangle)]
pub extern "C" fn ipc_server_recv(client_id: u64) -> *mut c_char {
    let mut guard = match SERVER.lock() {
        Ok(g) => g,
        Err(_) => return std::ptr::null_mut(),
    };

    let state = match guard.as_mut() {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };

    let stream = match state.clients.get_mut(&client_id) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };

    let mut reader = BufReader::new(&*stream);
    let mut buffer = String::new();
    match reader.read_line(&mut buffer) {
        Ok(0) | Err(_) => std::ptr::null_mut(),
        Ok(_) => to_c_string(buffer.trim_end()),
    }
}

/// 获取当前已连接的客户端数量
#[unsafe(no_mangle)]
pub extern "C" fn ipc_server_client_count() -> u64 {
    SERVER
        .lock()
        .ok()
        .and_then(|g| g.as_ref().map(|s| s.clients.len() as u64))
        .unwrap_or(0)
}

// ===================== 客户端 API =====================

/// 连接服务端，返回 handle（>0 表示成功，0 表示失败）
#[unsafe(no_mangle)]
pub extern "C" fn ipc_client_connect(name: *const c_char) -> u64 {
    let name_str = match from_c_str(name) {
        Some(s) => s,
        None => return 0,
    };

    let socket_name = match make_name(&name_str) {
        Ok(n) => n,
        Err(_) => return 0,
    };

    let stream = match Stream::connect(socket_name) {
        Ok(s) => s,
        Err(_) => return 0,
    };

    let id = NEXT_CLIENT_ID.fetch_add(1, Ordering::SeqCst);
    if let Ok(mut map) = CLIENTS.lock() {
        map.insert(id, stream);
        id
    } else {
        0
    }
}

/// 客户端发送消息
#[unsafe(no_mangle)]
pub extern "C" fn ipc_client_send(handle: u64, data: *const c_char) -> c_int {
    let msg = match from_c_str(data) {
        Some(s) => s,
        None => return -2,
    };

    let mut map = match CLIENTS.lock() {
        Ok(m) => m,
        Err(_) => return -1,
    };

    let stream = match map.get_mut(&handle) {
        Some(s) => s,
        None => return -3,
    };

    let line = format!("{}\n", msg);
    match stream
        .write_all(line.as_bytes())
        .and_then(|_| stream.flush())
    {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// 客户端接收一行消息（阻塞）
/// 返回的字符串必须用 [`ipc_free_string`] 释放
#[unsafe(no_mangle)]
pub extern "C" fn ipc_client_recv(handle: u64) -> *mut c_char {
    let mut map = match CLIENTS.lock() {
        Ok(m) => m,
        Err(_) => return std::ptr::null_mut(),
    };

    let stream = match map.get_mut(&handle) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };

    let mut reader = BufReader::new(&*stream);
    let mut buffer = String::new();
    match reader.read_line(&mut buffer) {
        Ok(0) | Err(_) => std::ptr::null_mut(),
        Ok(_) => to_c_string(buffer.trim_end()),
    }
}

/// 关闭客户端连接
#[unsafe(no_mangle)]
pub extern "C" fn ipc_client_close(handle: u64) -> c_int {
    let mut map = match CLIENTS.lock() {
        Ok(m) => m,
        Err(_) => return -1,
    };

    if map.remove(&handle).is_some() { 0 } else { -3 }
}

// ===================== 内存管理 =====================

/// 释放由本库返回的字符串
#[unsafe(no_mangle)]
pub extern "C" fn ipc_free_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            let _ = CString::from_raw(s);
        }
    }
}
