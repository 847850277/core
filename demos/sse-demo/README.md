# SSE Demo

这个演示项目展示了如何使用Server-Sent Events (SSE) 替代WebSocket来实现单向事件推送功能。

## 功能特性

### 🎯 核心功能
- **SSE事件流**: 实现基于SSE的实时事件推送
- **上下文订阅**: 支持按context ID订阅特定事件
- **事件过滤**: 服务端智能过滤，只推送订阅的事件
- **自动重连**: 客户端自动重连机制
- **连接管理**: 支持多客户端并发连接

### 📡 支持的事件类型
- **Context事件**: 模拟应用上下文执行事件
- **System事件**: 系统状态和通知事件
- **Heartbeat事件**: 连接保活心跳事件
- **Error事件**: 错误和异常通知

### 🛠️ 提供的工具
1. **HTTP服务器**: 提供SSE端点和Web界面
2. **命令行客户端**: 用于测试SSE连接的CLI工具
3. **Web界面**: 可视化的事件监控界面
4. **API端点**: 手动触发事件的REST API

## 快速开始

### 1. 启动服务器
```bash
cd demos/sse-demo
cargo run --bin sse-server
```

服务器将在 http://127.0.0.1:3000 启动

### 2. 使用Web界面
访问 http://127.0.0.1:3000 来查看可视化的SSE演示界面。

### 3. 使用命令行客户端
```bash
# 监听所有事件
cargo run --bin sse-client listen

# 只监听特定上下文的事件
cargo run --bin sse-client listen context-1,context-2

# 触发自定义事件
cargo run --bin sse-client trigger test-context "Hello SSE!"

# 查看服务器统计信息
cargo run --bin sse-client stats
```

## API端点

### SSE端点
- `GET /events` - 主SSE事件流端点
  - 查询参数:
    - `connection_id`: 可选的连接标识符
    - `contexts`: 逗号分隔的上下文ID列表

示例:
```bash
# 订阅所有事件
curl -N http://127.0.0.1:3000/events

# 订阅特定上下文
curl -N http://127.0.0.1:3000/events?contexts=context-1,context-2
```

### REST端点
- `GET /` - 演示Web界面
- `GET /trigger` - 触发自定义事件
- `GET /stats` - 获取连接统计信息

## SSE vs WebSocket 对比

| 特性 | WebSocket | SSE |
|------|-----------|-----|
| 通信方向 | 双向 | 单向(服务器→客户端) |
| 协议复杂度 | 较高 | 简单 |
| 浏览器支持 | 需要JavaScript | 原生EventSource API |
| 自动重连 | 需要手动实现 | 浏览器自动处理 |
| 代码复杂度 | 较高 | 较低 |
| 适用场景 | 需要双向通信 | 单向事件推送 |

## 架构优势

### 1. 简化的服务端实现
- 无需处理复杂的WebSocket握手和状态管理
- 标准的HTTP流响应，易于调试和监控
- 更好的与现有HTTP基础设施集成

### 2. 客户端友好
- 浏览器原生支持，无需额外库
- 自动重连和错误恢复
- 标准的HTTP缓存和代理支持

### 3. 运维友好
- 标准HTTP日志和监控
- 更好的负载均衡器支持  
- 防火墙和代理友好

## 演示场景

1. **基本连接**: 演示SSE连接建立和事件接收
2. **上下文订阅**: 展示按context过滤事件的能力
3. **多客户端**: 支持多个客户端同时连接
4. **错误处理**: 演示网络中断后的自动重连
5. **性能测试**: 模拟高频事件推送场景

## 项目结构

```
sse-demo/
├── src/
│   ├── main.rs          # SSE服务器实现
│   └── client.rs        # 命令行客户端
├── static/
│   └── index.html       # Web演示界面
├── Cargo.toml           # 项目配置
└── README.md            # 说明文档
```

## 技术实现细节

### 服务端核心组件
- **事件广播器**: 使用`tokio::sync::broadcast`实现事件分发
- **连接管理**: 跟踪活跃连接和订阅状态
- **事件过滤**: 基于订阅列表过滤推送事件
- **流处理**: 使用`axum::response::sse`处理SSE流

### 客户端实现
- **EventSource**: 浏览器原生SSE客户端
- **reqwest-eventsource**: Rust CLI客户端库
- **自动重连**: 实现连接断开后的自动重连逻辑

## 扩展可能

这个演示可以很容易地扩展来支持：
- 认证和授权
- 事件持久化和重放
- 集群化部署
- 指标和监控集成
- 更复杂的事件路由规则

## 运行要求

- Rust 1.70+
- 现代Web浏览器(支持EventSource)
- 网络连接

## 许可证

MIT OR Apache-2.0
