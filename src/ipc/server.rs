use crate::ipc::frame::{try_pop_frame, write_frame};
use crate::ipc::ns::make_name;
use crate::ipc::state::{ClientConn, SERVER, SERVER_NEXT_ID, SERVER_RUNNING, ServerState};
use crate::util::{from_c_str, to_c_string};
use interprocess::local_socket::{ListenerNonblockingMode, ListenerOptions, prelude::*};
use std::collections::HashMap;
use std::io::{ErrorKind, Read};
use std::os::raw::{c_char, c_int};
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;

/// 启动服务端
/// 返回值：0 成功，-1 通用错误，-2 参数错误，-3 未运行，-4 已在运行
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
                    if let Err(e) = stream.set_nonblocking(true) {
                        eprintln!("[ipc_lib] set_nonblocking 失败: {e}");
                        continue;
                    }

                    let id = SERVER_NEXT_ID.fetch_add(1, Ordering::SeqCst);
                    if let Ok(mut guard) = SERVER.lock() {
                        if let Some(state) = guard.as_mut() {
                            state.clients.insert(
                                id,
                                ClientConn {
                                    stream,
                                    recv_buf: Vec::new(),
                                },
                            );
                            println!("[ipc_lib] 新客户端连接, id={}", id);
                        }
                    }
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
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

/// 向指定客户端发送一帧完整消息（可含换行）
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

    let conn = match state.clients.get_mut(&client_id) {
        Some(c) => c,
        None => return -3,
    };

    match write_frame(&mut conn.stream, msg.as_bytes()) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// 非阻塞接收一帧
///
/// - 非 null：完整消息，需 ipc_free_string 释放
/// - null：半包 / 暂无数据 / 连接不存在 / 出错
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

    let conn = match state.clients.get_mut(&client_id) {
        Some(c) => c,
        None => return std::ptr::null_mut(),
    };

    let mut tmp = [0u8; 4096];
    loop {
        match conn.stream.read(&mut tmp) {
            Ok(0) => break,
            Ok(n) => conn.recv_buf.extend_from_slice(&tmp[..n]),
            Err(e) if e.kind() == ErrorKind::WouldBlock => break,
            Err(_) => return std::ptr::null_mut(),
        }
    }

    match try_pop_frame(&mut conn.recv_buf) {
        Some(data) => {
            let s = String::from_utf8_lossy(&data).into_owned();
            to_c_string(&s)
        }
        None => std::ptr::null_mut(),
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

/// 列出当前客户端 id
/// out_ids: 调用方数组；max_count: 容量；返回实际写入数量
#[unsafe(no_mangle)]
pub extern "C" fn ipc_server_list_clients(out_ids: *mut u64, max_count: u64) -> u64 {
    if out_ids.is_null() || max_count == 0 {
        return 0;
    }

    let guard = match SERVER.lock() {
        Ok(g) => g,
        Err(_) => return 0,
    };

    let Some(state) = guard.as_ref() else {
        return 0;
    };

    let mut n = 0u64;
    for &id in state.clients.keys() {
        if n >= max_count {
            break;
        }
        unsafe {
            *out_ids.add(n as usize) = id;
        }
        n += 1;
    }
    n
}
