#!/usr/bin/env python3
# -*- coding: utf-8 -*-

import ctypes
import time
import sys
import os

# ===================== 加载 DLL =====================
dll_path = os.path.join(os.path.dirname(__file__), "..",
                        "target", "release", "ipc_bridge.dll")


try:
    lib = ctypes.CDLL(dll_path)
except OSError as e:
    print(f"加载 DLL 失败: {e}")
    sys.exit(1)

# ===================== 函数签名 =====================
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

# ===================== 主逻辑 =====================
SOCKET_NAME = b"rust-ipc-lib-test"

print("[Python Server] 正在启动服务端...")
ret = lib.ipc_server_start(SOCKET_NAME)
if ret != 0:
    print(f"[Python Server] 启动失败，错误码: {ret}")
    sys.exit(1)

print("[Python Server] 服务端启动成功！")
print("[Python Server] 等待客户端连接... (按 Ctrl+C 退出)\n")

try:
    last_count = 0
    while True:
        count = lib.ipc_server_client_count()
        if count != last_count:
            print(f"[Python Server] 当前客户端数量: {count}")
            last_count = count

        # 简单轮询：尝试从 client_id=1 开始读取
        # 注意：当前 DLL 实现里客户端 ID 是自增的，这里仅作演示
        if count > 0:
            for client_id in range(1, int(count) + 5):  # 多试几个 ID
                ptr = lib.ipc_server_recv(client_id)
                if ptr:
                    msg = ctypes.cast(ptr, ctypes.c_char_p).value
                    if msg:
                        msg = msg.decode("utf-8", errors="ignore")
                        print(
                            f"[Python Server] 收到来自客户端 {client_id} 的消息: {msg}")

                        # 回复
                        reply = f"Echo from Python Server: {msg}".encode(
                            "utf-8")
                        lib.ipc_server_send(client_id, reply)
                    lib.ipc_free_string(ptr)

        time.sleep(0.3)

except KeyboardInterrupt:
    print("\n[Python Server] 正在停止服务端...")
    lib.ipc_server_stop()
    print("[Python Server] 已退出")
