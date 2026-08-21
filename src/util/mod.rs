use std::ffi::{CStr, CString};
use std::os::raw::c_char;

/// 释放 C 字符串
#[unsafe(no_mangle)]
pub extern "C" fn ipc_free_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            let _ = CString::from_raw(s);
        }
    }
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
