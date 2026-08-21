pub(crate) mod ipc;

// 重新导出所有 C API，方便外部链接
pub use ipc::client::{ipc_client_close, ipc_client_connect, ipc_client_recv, ipc_client_send};
pub use ipc::common::ipc_free_string;
pub use ipc::server::{
    ipc_server_client_count, ipc_server_recv, ipc_server_send, ipc_server_start, ipc_server_stop,
};
