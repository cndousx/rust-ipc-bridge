use crate::common::{from_c_str, make_name, to_c_string};
use crate::state::{CLIENTS, NEXT_ID};
use interprocess::local_socket::{Stream, prelude::*};
use std::io::{BufRead, BufReader, Write};
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

    let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
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

/// 客户端接收一行（阻塞）
/// 
/// 返回的字符串必须用 [`ipc_free_string`]  释放
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
