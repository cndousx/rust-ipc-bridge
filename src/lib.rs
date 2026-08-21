pub(crate) mod ipc;
pub(crate) mod util;

// 重新导出所有 C API，方便外部链接
pub use ipc::client::{ipc_client_close, ipc_client_connect, ipc_client_recv, ipc_client_send};
pub use ipc::server::{
    ipc_server_client_count, ipc_server_recv, ipc_server_send, ipc_server_start, ipc_server_stop,
};
pub use util::ipc_free_string;
