# Off-Chain Game - 猜数字游戏客户端

这是猜数字游戏的链下客户端实现，提供命令行和Web界面两种方式来体验游戏。游戏逻辑完全在本地运行，确保快速响应，而游戏结果通过 Calimero Network 存储到 NEAR 区块链。

## 🎯 项目特性

- **🎮 双模式支持**: 命令行客户端和Web服务器
- **🏆 多难度等级**: 简单(1-50)、普通(1-100)、困难(1-200)
- **📊 实时统计**: 游戏统计、胜率分析、历史记录
- **💾 本地存储**: 本地缓存和文件存储
- **🔗 区块链集成**: 通过 Calimero 存储到 NEAR 链
- **🎨 美观界面**: 彩色命令行和现代Web界面

## 🚀 快速开始

### 安装依赖

```bash
# 确保已安装 Rust 1.70+
rustc --version

# 进入项目目录
cd core/demos/guess-number-module/off-chain-game

# 构建项目
cargo build
```

### 运行命令行版本

```bash
# 开始游戏（普通难度）
cargo run --bin guess-number-client play

# 指定难度和玩家ID
cargo run --bin guess-number-client play --difficulty hard --player "my_player_123"

# 查看统计信息
cargo run --bin guess-number-client stats

# 查看游戏历史
cargo run --bin guess-number-client history

# 查看配置帮助
cargo run --bin guess-number-client config
```

### 运行Web服务器

```bash
# 启动Web服务器
cargo run --bin guess-number-server

# 访问 http://127.0.0.1:8080
```

## 🎮 游戏说明

### 游戏规则
1. 系统随机生成一个目标数字
2. 玩家输入猜测的数字
3. 系统提示"太大了"、"太小了"或"猜对了"
4. 在规定次数内猜中即获胜
5. 游戏结果自动保存到区块链

### 难度等级
- **简单**: 1-50范围，最多8次机会
- **普通**: 1-100范围，最多10次机会  
- **困难**: 1-200范围，最多12次机会

## 🏗️ 项目结构

```
off-chain-game/
├── src/
│   ├── main.rs          # 命令行客户端入口
│   ├── server.rs        # Web服务器入口
│   ├── lib.rs          # 共享类型和工具函数
│   ├── game.rs         # 核心游戏逻辑
│   ├── error.rs        # 错误处理
│   ├── calimero.rs     # Calimero Network 集成
│   └── storage.rs      # 本地存储和缓存
├── static/
│   └── index.html      # Web界面
├── Cargo.toml          # 项目配置
└── README.md           # 本文件
```

## 📡 API 接口

### 游戏操作
- `POST /api/game/start` - 创建新游戏
- `POST /api/game/{id}/guess` - 提交猜测
- `GET /api/game/{id}/status` - 获取游戏状态
- `POST /api/game/{id}/finish` - 结束游戏

### 统计查询
- `GET /api/stats/player/{id}` - 玩家统计
- `GET /api/stats/leaderboard` - 排行榜
- `GET /api/history/{id}` - 游戏历史

### 系统接口
- `GET /health` - 健康检查
- `GET /` - Web界面

## 🔧 配置选项

### 命令行参数

```bash
# 基本用法
cargo run --bin guess-number-client [命令] [选项]

# 可用命令
play      # 开始游戏（默认）
stats     # 查看统计
history   # 查看历史
config    # 查看配置

# 可用选项
--player, -p <ID>           # 玩家ID
--difficulty, -d <LEVEL>    # 游戏难度 (easy/normal/hard)
--max-attempts, -m <NUM>    # 最大尝试次数
```

### 环境变量

```bash
# 设置日志级别
export RUST_LOG=debug

# Calimero配置
export CALIMERO_NODE_ENDPOINT=http://localhost:2428
export CALIMERO_CONTEXT_ID=guess-number-game

# NEAR网络配置
export NEAR_NETWORK=testnet
export NEAR_CONTRACT_ACCOUNT=guess-number.testnet
```

## 🧪 测试

```bash
# 运行单元测试
cargo test

# 运行特定模块测试
cargo test game
cargo test storage

# 运行集成测试
cargo test --test integration

# 查看测试覆盖率
cargo test --verbose
```

## 🏃 开发指南

### 添加新功能

1. **扩展游戏模式**
```rust
// 在 src/game.rs 中添加新的游戏变体
pub enum GameMode {
    Classic,
    Timed,
    Multiplayer,
}
```

2. **自定义存储后端**
```rust
// 实现 StorageProvider trait
#[async_trait]
impl StorageProvider for MyCustomProvider {
    async fn store_game_result(&self, record: &GameRecord) -> GameResult<String> {
        // 自定义存储逻辑
    }
}
```

3. **添加新的难度等级**
```rust
// 在 GameConfig::for_difficulty 中添加新选项
match difficulty {
    "expert" => GameConfig {
        min_number: 1,
        max_number: 1000,
        max_attempts: 15,
    },
    // ...
}
```

### 调试建议

```bash
# 启用详细日志
RUST_LOG=debug cargo run --bin guess-number-client

# 检查存储状态
find ./game_data -name "*.json" | wc -l

# 监控服务器日志
RUST_LOG=info cargo run --bin guess-number-server
```

## 🔍 故障排除

### 常见问题

1. **编译错误 - 缺少依赖**
```bash
# 更新依赖
cargo update
cargo build
```

2. **无法连接到 Calimero 节点**
```bash
# 检查节点是否运行
curl http://localhost:2428/health

# 启动本地节点（如果需要）
merod --config ../../../config/local-node.toml
```

3. **存储权限问题**
```bash
# 检查数据目录权限
ls -la ./game_data/
chmod 755 ./game_data/
```

4. **Web服务器端口被占用**
```bash
# 检查端口使用情况
lsof -i :8080

# 使用不同端口（需要修改源代码）
# 或者关闭占用端口的程序
```

### 日志分析

```bash
# 查看游戏活动
RUST_LOG=info cargo run 2>&1 | grep "Game"

# 监控存储操作
RUST_LOG=debug cargo run 2>&1 | grep -E "(store|cache)"

# 检查错误
RUST_LOG=warn cargo run 2>&1 | grep -E "(error|warn)"
```

## 🚀 性能优化

### 本地优化
- 游戏逻辑完全在内存中运行
- 智能缓存减少文件I/O
- 批量区块链操作降低延迟

### 存储优化
- 自动清理过期缓存
- 压缩历史数据
- 定期备份重要数据

## 🔗 相关资源

- [Rust 猜数字游戏教程](https://rustwiki.org/zh-CN/book/ch02-00-guessing-game-tutorial.html)
- [Calimero Network 文档](https://docs.calimero.network)
- [NEAR 协议文档](https://docs.near.org)

## 🛠️ 开发工具

推荐的开发工具链：

```bash
# 代码格式化
cargo fmt

# 代码检查
cargo clippy

# 文档生成
cargo doc --open

# 性能分析
cargo build --release
```

## 📈 监控和指标

项目提供多种监控方式：

- **命令行统计**: 使用 `stats` 命令查看详细数据
- **Web界面仪表板**: 实时显示游戏状态和统计
- **健康检查端点**: `/health` 接口监控服务状态
- **日志记录**: 详细的tracing日志支持

## 🤝 贡献指南

1. Fork 本项目
2. 创建功能分支
3. 编写测试用例
4. 确保所有测试通过
5. 提交 Pull Request

### 代码规范
- 使用 `cargo fmt` 格式化代码
- 通过 `cargo clippy` 检查
- 为公共API添加文档注释
- 编写充分的测试覆盖

## 📄 许可证

本项目采用 MIT OR Apache-2.0 双重许可证。

## 🆘 获取帮助

- 查看 [Issues](https://github.com/calimero-network/core/issues)
- 加入 [Discord 社区](https://discord.gg/wZRC73DVpU)
- 阅读 [项目文档](../README.md)

---

🎮 **开始游戏**: `cargo run --bin guess-number-client play`  
🌐 **启动服务器**: `cargo run --bin guess-number-server`  
📊 **查看统计**: `cargo run --bin guess-number-client stats`
