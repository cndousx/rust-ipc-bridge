use interprocess::local_socket::{GenericNamespaced, Name, ToNsName};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

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
    unsafe { CStr::from_ptr(ptr) }
        .to_str()
        .ok()
        .map(|s| s.to_owned())
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