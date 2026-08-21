use std::io::{self, Write};

/// 写入一帧：4 字节小端长度 + payload
pub fn write_frame(stream: &mut impl Write, data: &[u8]) -> io::Result<()> {
    let len = (data.len() as u32).to_le_bytes();
    stream.write_all(&len)?; // 固定 4 字节
    stream.write_all(data)?; // 正文
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
