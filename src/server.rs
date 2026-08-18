use crate::common::{from_c_str, make_name, to_c_string};
use crate::state::{SERVER, SERVER_RUNNING, NEXT_ID, ServerState};
use interprocess::local_socket::{
    prelude::*, ListenerNonblockingMode, ListenerOptions,
};
use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Write};
use std::os::raw::{c_char, c_int};
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;

/// 启动服务端
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
                    let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
                    if let Ok(mut guard) = SERVER.lock() {
                        if let Some(state) = guard.as_mut() {
                            state.clients.insert(id, stream);
                            println!("[ipc_lib] 新客户端连接, id={}", id);
                        }
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
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
        return -3;
    }

    let mut guard = SERVER.lock().unwrap();
    if let Some(mut state) = guard.take() {
        state.clients.clear();
        if let Some(handle) = state.listener_thread.take() {
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
    match stream.write_all(line.as_bytes()).and_then(|_| stream.flush()) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// 从指定客户端读取一行（阻塞）
/// 
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

/// 当前已连接客户端数量
#[unsafe(no_mangle)]
pub extern "C" fn ipc_server_client_count() -> u64 {
    SERVER
        .lock()
        .ok()
        .and_then(|g| g.as_ref().map(|s| s.clients.len() as u64))
        .unwrap_or(0)
}