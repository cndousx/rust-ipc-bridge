import os
import sys
import platform
import ctypes
SOCKET_NAME = b"rust-ipc-lib-test"


def loadLib():
    system = platform.system()
    if system == "Linux":
        lib_name = "libipc_bridge.so"
    elif system == "Darwin":
        lib_name = "ipc_bridge.dylib"
    elif system == "Windows":
        lib_name = "ipc_bridge.dll"
    else:
        print(f"不支持的平台: {system}")
        sys.exit(1)

    dll_path = os.path.join(os.path.dirname(__file__), "..",
                            "target", "release", lib_name)

    try:
        lib = ctypes.CDLL(dll_path)
        # ===================== server 函数签名 =====================
        lib.ipc_server_start.argtypes = [ctypes.c_char_p]
        lib.ipc_server_start.restype = ctypes.c_int

        lib.ipc_server_stop.argtypes = []
        lib.ipc_server_stop.restype = ctypes.c_int

        lib.ipc_server_send.argtypes = [ctypes.c_uint64, ctypes.c_char_p]
        lib.ipc_server_send.restype = ctypes.c_int

        lib.ipc_server_recv.argtypes = [ctypes.c_uint64]
        lib.ipc_server_recv.restype = ctypes.c_void_p

        lib.ipc_server_client_count.argtypes = []
        lib.ipc_server_client_count.restype = ctypes.c_uint64

        lib.ipc_free_string.argtypes = [ctypes.c_void_p]
        lib.ipc_free_string.restype = None

        # ===================== client 函数签名 =====================
        lib.ipc_client_connect.argtypes = [ctypes.c_char_p]
        lib.ipc_client_connect.restype = ctypes.c_uint64

        lib.ipc_client_send.argtypes = [ctypes.c_uint64, ctypes.c_char_p]
        lib.ipc_client_send.restype = ctypes.c_int

        lib.ipc_client_recv.argtypes = [ctypes.c_uint64]
        lib.ipc_client_recv.restype = ctypes.c_void_p

        lib.ipc_client_close.argtypes = [ctypes.c_uint64]
        lib.ipc_client_close.restype = ctypes.c_int

        lib.ipc_free_string.argtypes = [ctypes.c_void_p]
        lib.ipc_free_string.restype = None

        return lib
    except OSError as e:
        print(f"加载 DLL 失败: {e}")
        sys.exit(1)
