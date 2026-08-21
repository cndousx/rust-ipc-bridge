use interprocess::local_socket::{GenericNamespaced, Name, ToNsName};

/// 生成跨平台的 local socket 名称
pub fn make_name(name: &str) -> Result<Name<'static>, String> {
    name.to_ns_name::<GenericNamespaced>()
        .map_err(|e| e.to_string())
        .map(|n| n.into_owned())
}
