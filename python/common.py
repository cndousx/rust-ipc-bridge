import os
import sys
import platform
SOCKET_NAME = b"rust-ipc-lib-test"


system = platform.system()
if system == "Linux":
    lib_name = "libipc_bridge"
elif system == "Darwin":
    lib_name = "ipc_bridge.dylib"
elif system == "Windows":
    lib_name = "ipc_bridge.dll"
else:
    print(f"不支持的平台: {system}")
    sys.exit(1)

dll_path = os.path.join(os.path.dirname(__file__), "..",
                        "target", "release", lib_name)
