# Rust 封装IPC方便其他语言复用

## 单测

1. 编译生成动态链接库`cargo build --release`

2. 测试脚本里使用的是相对路径，要`cd`到`python`文件夹执行测试脚本
