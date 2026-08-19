# Rust 封装IPC方便其他语言复用

## 单测

1. 编译生成动态链接库`cargo build --release`

2. 测试脚本里使用的是相对路径，要`cd`到`python`文件夹执行测试脚本

## 长度前缀协议示意

1. 单帧结构

| 偏移 | 0..3          | 4 .. 4+len-1   |
| ---- | ------------- | -------------- |
| 含义 | 长度 (u32LE)  | payload 正文   |
| 示例 | `0B 00 00 00` | `Hello\nWorld` |

2. 发送 / 接收流程

```mermaid
sequenceDiagram
    participant C as 发送端
    participant K as 内核缓冲区
    participant S as 接收端 recv_buf

    C->>K: write 长度(4字节)
    C->>K: write payload
    C->>K: flush

    S->>K: read（可能多次）
    K-->>S: 追加到 recv_buf
    Note over S: try_pop_frame
    alt 数据不够
        S-->>S: 返回空，继续等
    else 凑齐一帧
        S-->>S: 取出完整消息
    end
```

3. 两帧、分三次读完（半包 + 粘包）

```mermaid
flowchart TB
    subgraph 发送字节流
        F1["帧1: len=2 + Hi"]
        F2["帧2: len=11 + Hello\\nWorld"]
        F1 --- F2
    end

    subgraph 第1次read
        R1["读到: 帧1完整 + 帧2长度的前2字节"]
        R1 --> P1["pop 出 Hi"]
        P1 --> B1["buf 剩余: 半个长度字段"]
    end

    subgraph 第2次read
        R2["读到: 长度剩余2字节 + 正文前5字节"]
        R2 --> B2["仍凑不齐帧2 → 返回空"]
    end

    subgraph 第3次read
        R3["读到: 正文剩余6字节"]
        R3 --> P2["pop 出 Hello\nWorld"]
    end

    发送字节流 --> 第1次read --> 第2次read --> 第3次read
```

4. try_pop_frame 判断逻辑

```mermaid
flowchart TD
    A[recv_buf] --> B{长度 ≥ 4?}
    B -->|否| Z1[返回 None<br/>半包：长度未齐]
    B -->|是| C[解析 len = u32LE]
    C --> D{len 合法?<br/>如 ≤ 16MB}
    D -->|否| E[清空 buf / 报错]
    D -->|是| F{buf.len ≥ 4+len?}
    F -->|否| Z2[返回 None<br/>半包：正文未齐]
    F -->|是| G[去掉 4 字节长度]
    G --> H[取出 len 字节 payload]
    H --> I[返回完整一帧]
```
