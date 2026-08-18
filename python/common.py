import os

SOCKET_NAME = b"rust-ipc-lib-test"

dll_path = os.path.join(os.path.dirname(__file__), "..",
                        "target", "release", "ipc_bridge.dll")
