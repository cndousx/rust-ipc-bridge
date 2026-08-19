use interprocess::local_socket::{GenericNamespaced, Name, ToNsName};
use std::ffi::{CStr, CString};
use std::io::{self, Write};
use std::os::raw::c_char;

/// 写入一帧：4 字节小端长度 + payload
pub fn write_frame(stream: &mut impl Write, data: &[u8]) -> io::Result<()> {
    let len = (data.len() as u32).to_le_bytes();
    stream.write_all(&len)?;
    stream.write_all(data)?;
    stream.flush()?;
    Ok(())
}

/// 从 recv_buf 里尝试拆出一帧。
/// 
/// 数据不够时返回 None，不消费缓冲区。
pub fn try_pop_frame(buf: &mut Vec<u8>) -> Option<Vec<u8>> {
    if buf.len() < 4 {
        return None;
    }
    let len = u32::from_le_bytes(buf[0..4].try_into().unwrap()) as usize;

    // 简单保护：单帧最大 16MB，防止异常长度
    if len > 16 * 1024 * 1024 {
        buf.clear();
        return None;
    }

    if buf.len() < 4 + len {
        return None; // 半包
    }

    buf.drain(..4);
    Some(buf.drain(..len).collect())
}

/// 把 Rust 字符串转成 C 字符串
///
/// 返回的字符串必须用 [`ipc_free_string`]  释放
pub fn to_c_string(s: &str) -> *mut c_char {
    CString::new(s)
        .map(|c| c.into_raw())
        .unwrap_or(std::ptr::null_mut())
}

/// 从 C 字符串指针安全地转成 Rust String
pub fn from_c_str(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    match unsafe { CStr::from_ptr(ptr) }.to_str() {
        Ok(s) => Some(s.to_owned()),
        Err(e) => unsafe {
            // 如果不能正常解码，尝试gbk解码
            let str = CStr::from_ptr(ptr);
            let received_bytes = str.to_bytes();
            let (gbk_decoded, _, had_errors) = encoding_rs::GBK.decode(received_bytes);
            if !had_errors {
                let corrected_string = gbk_decoded.to_string();
                return Some(corrected_string);
            }
            println!("Error encoder error : {:?}", e);
            None
        },
    }
}

/// 生成跨平台的 local socket 名称
pub fn make_name(name: &str) -> Result<Name<'static>, String> {
    name.to_ns_name::<GenericNamespaced>()
        .map_err(|e| e.to_string())
        .map(|n| n.into_owned())
}

/// 释放 C 字符串
#[unsafe(no_mangle)]
pub extern "C" fn ipc_free_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            let _ = CString::from_raw(s);
        }
    }
}
