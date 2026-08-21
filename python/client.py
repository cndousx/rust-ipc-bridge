# -*- coding: utf-8 -*-

import ctypes
import time
import sys
import os
from ipc import loadLib, SOCKET_NAME


def client():
    lib = loadLib()
    # ===================== 主逻辑 =====================

    print("[Python Client] 正在连接服务端...")
    handle = lib.ipc_client_connect(SOCKET_NAME)

    if handle == 0:
        print("[Python Client] 连接失败！请先启动 server.py")
        sys.exit(1)

    print(f"[Python Client] 连接成功，handle = {handle}\n")

    messages = [
        "Hello from Python Client!",
        """
        换行
        测试""",
        "                   ",
        "Rust IPC DLL 测试",
        "再见 server"
    ]

    try:
        for msg in messages:
            print(f"[Python Client] 发送: {msg}")
            ret = lib.ipc_client_send(handle, msg.encode("utf-8"))
            if ret != 0:
                print(f"[Python Client] 发送失败，错误码: {ret}")
                break

            # 非阻塞读取
            ptr = lib.ipc_client_recv(handle)
            if ptr:
                response = ctypes.cast(ptr, ctypes.c_char_p).value
                if response:
                    print(
                        f"[Python Client] 收到回复: {response.decode('utf-8', errors='ignore')}\n")
                lib.ipc_free_string(ptr)

            time.sleep(3)

    except KeyboardInterrupt:
        print("\n[Python Client] 用户中断")

    finally:
        print("[Python Client] 关闭连接")
        lib.ipc_client_close(handle)
        print("[Python Client] 结束")


if __name__ == "__main__":
    client()
