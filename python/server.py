# -*- coding: utf-8 -*-

import ctypes
import time
import sys
import threading
from ipc import loadLib, SOCKET_NAME


# ===================== server 主逻辑 =====================

def server(stop_event):
    print("[Python Server] 正在启动服务端...")
    lib = loadLib()
    ret = lib.ipc_server_start(SOCKET_NAME)
    if ret != 0:
        print(f"[Python Server] 启动失败，错误码: {ret}")
        sys.exit(1)

    print("[Python Server] 服务端启动成功！")
    print("[Python Server] 等待客户端连接... (按 Ctrl+C 退出)\n")

    try:
        last_count = 0
        while not stop_event.is_set():
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
                                f"[Python Server] 收到来自客户端[{client_id}]的消息: {msg}")
                            # 回复
                            reply = f"Echo from Python Server: {msg}".encode(
                                "utf-8")
                            lib.ipc_server_send(client_id, reply)
                        lib.ipc_free_string(ptr)

            time.sleep(0.3)
            if stop_event.wait(timeout=1.0):
                print("\n[Python Server] 收到停止信号，清理资源后退出")
                print("[Python Server] 正在停止服务端...")
                lib.ipc_server_stop()
                print("[Python Server] 已退出")
                break
    except KeyboardInterrupt:
        print("\n[Python Server] 正在停止服务端...")
        lib.ipc_server_stop()
        print("[Python Server] 已退出")


if __name__ == "__main__":
    server(stop_event=threading.Event())
