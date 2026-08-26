use crate::ipc::frame::{try_pop_frame, write_frame};
use crate::ipc::ns::make_name;
use crate::ipc::state::{CLIENTS, ClientConn, CLIENT_NEXT_ID};
use crate::util::{from_c_str, to_c_string};
use interprocess::local_socket::{Stream, prelude::*};
use std::io::{ErrorKind, Read};
use std::os::raw::{c_char, c_int};
use std::sync::atomic::Ordering;

/// 连接服务端，返回 handle（>0 成功，0 失败）
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

    if let Err(e) = stream.set_nonblocking(true) {
        eprintln!("[ipc_lib] client set_nonblocking 失败: {e}");
        return 0;
    }

    let id = CLIENT_NEXT_ID.fetch_add(1, Ordering::SeqCst);
    if let Ok(mut map) = CLIENTS.lock() {
        map.insert(
            id,
            ClientConn {
                stream,
                recv_buf: Vec::new(),
            },
        );
        id
    } else {
        0
    }
}

/// 发送一帧完整消息（可含换行）
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

    let conn = match map.get_mut(&handle) {
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
/// - null：半包 / 暂无数据 / handle 无效 / 出错
#[unsafe(no_mangle)]
pub extern "C" fn ipc_client_recv(handle: u64) -> *mut c_char {
    let mut map = match CLIENTS.lock() {
        Ok(m) => m,
        Err(_) => return std::ptr::null_mut(),
    };

    let conn = match map.get_mut(&handle) {
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

/// 关闭客户端连接
#[unsafe(no_mangle)]
pub extern "C" fn ipc_client_close(handle: u64) -> c_int {
    let mut map = match CLIENTS.lock() {
        Ok(m) => m,
        Err(_) => return -1,
    };

    if map.remove(&handle).is_some() { 0 } else { -3 }
}
