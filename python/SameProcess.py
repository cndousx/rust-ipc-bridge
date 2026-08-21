import threading
import time
import time
import sys
from ipc import SOCKET_NAME
from client import client
from server import server


def main():

    stop_event = threading.Event()
    # 创建并启动线程
    t1 = threading.Thread(target=server, args=(stop_event,))
    try:
        t2 = threading.Thread(target=client)
        t1.start()
        # 等server启动后在启动client
        time.sleep(3)
        t2.start()

        # 等待所有线程执行完毕
        t1.join()
        t2.join()
        print("所有任务完成")
    except KeyboardInterrupt:
        # 捕获 Ctrl+C，通知子线程停止
        print("\n[Main] 捕获到 Ctrl+C，正在通知子线程停止...")
        stop_event.set()
        t1.join(timeout=3)  # 给子线程最多3秒清理时间
        if t1.is_alive():
            print("[Main] ⚠️ 子线程未在规定时间内退出，强制终止")
        else:
            print("[Main] ✅ 子线程已优雅退出")
        sys.exit(0)


if __name__ == "__main__":
    main()
