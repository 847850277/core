# 笔记

- 启动服务

```bash
merod  --node-name node1 run
```

- watch context
```bash
./target/debug/meroctl context watch 6daWwM31pH2M2BZTfcJJehQhLJJfTeNnRTWuWXgGaFUD
```

- 查看正确的公钥
```bash
./target/debug/meroctl context identity list 
```

好的，让我测试一下`get`操作。首先启动监控，然后执行get操作来看看是否会产生事件： 让我重新启动watch命令： 让我直接启动一个新的watch命令： 现在测试get操作： 很好！我们可以看到：

## 📊 **get操作测试结果**

### ✅ **get操作的特征**：
1. **返回结果**: `{"jsonrpc":"2.0","id":10,"result":{"output":null}}` - 键不存在返回null
2. **事件类型**: `ExecutionEvent`（而不是`StateMutation`）
3. **事件数据**: `"events": []` - 空的事件数组
4. **无状态变更**: 没有产生`StateMutation`事件

让我测试一个存在的键： 很好！现在我知道有一个键`"new_key"`，让我获取它： 完美！现在让我测试其他只读操作： ## 📋 **get操作（只读操作）测试总结**

### ✅ **只读操作的特征**

| 操作 | JSON-RPC响应 | WebSocket事件类型 | 事件数据 | StateMutation |
|------|-------------|-----------------|---------|---------------|
| `get("new_key")` | `{"output":"new_value"}` | `ExecutionEvent` | `"events": []` | ❌ 无 |
| `get("nonexistent")` | `{"output":null}` | `ExecutionEvent` | `"events": []` | ❌ 无 |
| `entries()` | `{"output":{"new_key":"new_value"}}` | `ExecutionEvent` | `"events": []` | ❌ 无 |
| `len()` | `{"output":1}` | `ExecutionEvent` | `"events": []` | ❌ 无 |

### 🔍 **关键发现**

1. **只读操作不触发StateMutation**:
    - `get`, `entries`, `len`等只读操作不会产生`StateMutation`事件
    - 只会产生`ExecutionEvent`事件

2. **ExecutionEvent的事件数组为空**:
    - 只读操作的`events`字段始终为空数组`[]`
    - 因为没有状态变更，所以没有应用程序事件被发出

3. **与写操作的对比**:
    - **写操作**（`set`, `remove`, `clear`）：产生`StateMutation` + `ExecutionEvent`（包含应用事件）
    - **读操作**（`get`, `entries`, `len`）：仅产生`ExecutionEvent`（空事件数组）

这完美地展示了Calimero系统中事件系统的精确性：只有真正改变状态的操作才会触发状态变更事件！

-- 查看相关
```bash
curl -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","method":"execute","params":{"contextId":"6daWwM31pH2M2BZTfcJJehQhLJJfTeNnRTWuWXgGaFUD","method":"get","argsJson":["new_key"],"executorPublicKey":"3pNBJqiG5Hzu7mLg8MN2a2Qj6jNfo5NvzcBmXCmniu95"},"id":1}' http://localhost:2528/jsonrpc

curl -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","method":"execute","params":{"contextId":"6daWwM31pH2M2BZTfcJJehQhLJJfTeNnRTWuWXgGaFUD","method":"get","argsJson":["new_key"],"executorPublicKey":"3pNBJqiG5Hzu7mLg8MN2a2Qj6jNfo5NvzcBmXCmniu95"},"id":12}' http://localhost:2528/jsonrpc
```

- 数目
```bash
curl -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","method":"execute","params":{"contextId":"6daWwM31pH2M2BZTfcJJehQhLJJfTeNnRTWuWXgGaFUD","method":"len","argsJson":[],"executorPublicKey":"3pNBJqiG5Hzu7mLg8MN2a2Qj6jNfo5NvzcBmXCmniu95"},"id":13}' http://localhost:2528/jsonrpc
```

- ExecutionEvent
- StateMutation

-- 设置相关
```bash
- set new key-value
curl -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","method":"execute","params":{"contextId":"6daWwM31pH2M2BZTfcJJehQhLJJfTeNnRTWuWXgGaFUD","method":"set","argsJson":["new_key","new_value"],"executorPublicKey":"3pNBJqiG5Hzu7mLg8MN2a2Qj6jNfo5NvzcBmXCmniu95"},"id":3}' http://localhost:2528/jsonrpc
- update key-value
curl -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","method":"execute","params":{"contextId":"6daWwM31pH2M2BZTfcJJehQhLJJfTeNnRTWuWXgGaFUD","method":"set","argsJson":["new_key","updated_value"],"executorPublicKey":"3pNBJqiG5Hzu7mLg8MN2a2Qj6jNfo5NvzcBmXCmniu95"},"id":4}' http://localhost:2528/jsonrpc
- remove key-value
curl -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","method":"execute","params":{"contextId":"6daWwM31pH2M2BZTfcJJehQhLJJfTeNnRTWuWXgGaFUD","method":"remove","argsJson":["new_key"],"executorPublicKey":"3pNBJqiG5Hzu7mLg8MN2a2Qj6jNfo5NvzcBmXCmniu95"},"id":5}' http://localhost:2528/jsonrpc
-- clear all key-value
 curl -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","method":"execute","params":{"contextId":"6daWwM31pH2M2BZTfcJJehQhLJJfTeNnRTWuWXgGaFUD","method":"clear","argsJson":[],"executorPublicKey":"3pNBJqiG5Hzu7mLg8MN2a2Qj6jNfo5NvzcBmXCmniu95"},"id":3}' 
```



让我查看Calimero系统中定义的所有WebSocket事件类型。我需要查看相关的源码来找到完整的事件类型列表。 很好！我找到了完整的WebSocket事件类型定义。让我查看最关键的事件类型定义文件： 让我也查看JavaScript客户端中定义的类型： ## 📋 **Calimero WebSocket事件类型完整总结**

基于源码分析，Calimero系统中的WebSocket事件类型结构如下：

### 🏗️ **事件层次结构**

```rust
// 顶层事件类型
pub enum NodeEvent {
    Context(ContextEvent),  // 唯一的变体
}

// Context相关事件
pub struct ContextEvent {
    pub context_id: ContextId,
    pub payload: ContextEventPayload,  // 实际的事件内容
}

// 事件载荷类型（核心事件类型）
pub enum ContextEventPayload {
    StateMutation(StateMutationPayload),    // 状态变更事件
    ExecutionEvent(ExecutionEventPayload),  // 执行事件
}
```

### 🎯 **主要事件类型**

#### **1. StateMutation** - 状态变更事件
```rust
pub struct StateMutationPayload {
    pub new_root: Hash,  // 新的状态根哈希
}
```

**触发条件**：
- ✅ 任何写操作（`set`, `remove`, `clear`）
- ✅ 状态树发生变化
- ❌ 只读操作不会触发

**JSON格式**：
```json
{
  "contextId": "6daWwM31pH2M2BZTfcJJehQhLJJfTeNnRTWuWXgGaFUD",
  "type": "StateMutation",
  "data": {
    "newRoot": "6Fx381kb9GmkDZehF3aSVkAXQwuhnT2vgK8Bzk3m91sH"
  }
}
```

#### **2. ExecutionEvent** - 执行事件
```rust
pub struct ExecutionEventPayload {
    pub events: Vec<ExecutionEvent>,  // 应用程序发出的事件数组
}

pub struct ExecutionEvent {
    pub kind: String,    // 事件类型（如 "Inserted", "Updated", "Removed"）
    pub data: Vec<u8>,   // 事件数据（字节数组）
}
```

**触发条件**：
- ✅ 任何方法调用（读/写操作）
- ✅ 应用程序通过`app::emit!`发出的事件

**JSON格式**：
```json
{
  "contextId": "6daWwM31pH2M2BZTfcJJehQhLJJfTeNnRTWuWXgGaFUD",
  "type": "ExecutionEvent", 
  "data": {
    "events": [
      {
        "kind": "Inserted",
        "data": [123, 34, 107, 101, 121, ...]  // JSON字节数组
      }
    ]
  }
}
```

### 📊 **事件类型对比表**

| 操作类型 | StateMutation | ExecutionEvent | ExecutionEvent.events |
|---------|---------------|----------------|----------------------|
| **写操作** (`set`, `remove`, `clear`) | ✅ 产生 | ✅ 产生 | ✅ 包含应用事件 |
| **读操作** (`get`, `entries`, `len`) | ❌ 不产生 | ✅ 产生 | ❌ 空数组 `[]` |

### 🔧 **应用程序事件类型**

根据KV Store应用的源码，应用程序可以发出以下事件：

```rust
#[app::event]
pub enum Event<'a> {
    Inserted { key: &'a str, value: &'a str },  // 插入事件
    Updated { key: &'a str, value: &'a str },   // 更新事件
    Removed { key: &'a str },                   // 删除事件
    Cleared,                                    // 清空事件
}
```

### 🌟 **总结**

Calimero系统目前只有 **2种主要WebSocket事件类型**：

1. **`StateMutation`** - 状态变更事件（仅写操作触发）
2. **`ExecutionEvent`** - 执行事件（所有操作都触发）

这个设计简洁而强大，通过这两种事件类型就能完整追踪context中的所有变化和执行情况！