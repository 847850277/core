# NEAR 智能合约 - 猜数字游戏链上存储

这是一个部署在 NEAR 区块链上的智能合约，用于存储猜数字游戏的结果、管理玩家统计数据，并提供全球排行榜功能。

## 🎯 项目概述

本智能合约是 Calimero Network 猜数字游戏项目的链上组件，负责：

- **数据永久存储**: 将游戏结果安全存储在 NEAR 区块链上
- **统计数据管理**: 自动计算和更新玩家统计信息
- **排行榜系统**: 维护全球玩家排行榜
- **历史记录查询**: 提供完整的游戏历史查询功能
- **去中心化治理**: 支持多管理员模式和权限管理

## 🏆 核心特性

### 📊 数据管理
- **游戏记录存储**: 存储每局游戏的详细信息
- **玩家统计**: 自动聚合计算玩家的胜率、平均尝试次数等
- **历史查询**: 支持分页查询玩家游戏历史
- **数据验证**: 严格的数据格式和内容验证

### 🏅 排行榜系统
- **实时排名**: 基于胜率和平均尝试次数的智能排名
- **缓存优化**: 高效的排行榜缓存机制
- **多维度排序**: 支持按不同维度排序的灵活排行榜

### 🔐 安全特性
- **权限控制**: 多级权限管理（Owner、Admin）
- **数据验证**: 全面的输入数据验证
- **存储成本**: 基于存储使用量的合理收费机制
- **事件日志**: 完整的操作事件记录

## 🚀 快速开始

### 环境要求

- Rust 1.70+ (with `wasm32-unknown-unknown` target)
- NEAR CLI 4.0+
- Node.js 16+ (for NEAR CLI)

### 安装依赖

```bash
# 安装 NEAR CLI
npm install -g near-cli

# 安装 Rust wasm 目标
rustup target add wasm32-unknown-unknown

# 进入合约目录
cd core/demos/guess-number-module/on-chain-contract
```

### 构建合约

```bash
# 构建 WASM 合约
cargo build --target wasm32-unknown-unknown --release

# 检查构建结果
ls -la target/wasm32-unknown-unknown/release/guess_number_contract.wasm
```

### 部署合约

```bash
# 使用部署脚本
./deploy.sh --account mycontract.testnet --owner myaccount.testnet

# 或手动部署
near deploy mycontract.testnet --wasmFile target/wasm32-unknown-unknown/release/guess_number_contract.wasm --networkId testnet

# 初始化合约
near call mycontract.testnet new '{"owner_id": "myaccount.testnet"}' --accountId myaccount.testnet --networkId testnet
```

## 📡 合约API

### 游戏记录管理

#### `store_game_record`
存储一个新的游戏记录（需要支付存储费用）

```bash
near call CONTRACT_ID store_game_record '{
  "record": {
    "game_id": "game_12345",
    "player_id": "player.testnet",
    "target_number": 42,
    "attempts": 5,
    "guesses": [50, 25, 60, 40, 42],
    "duration_seconds": 120,
    "timestamp": 1703097600,
    "success": true,
    "difficulty": "normal",
    "score": 750
  }
}' --accountId player.testnet --deposit 0.01 --networkId testnet
```

#### `get_game_record`
获取特定游戏记录

```bash
near view CONTRACT_ID get_game_record '{"game_id": "game_12345"}' --networkId testnet
```

### 玩家统计

#### `get_player_stats`
获取玩家统计数据

```bash
near view CONTRACT_ID get_player_stats '{"player_id": "player.testnet"}' --networkId testnet
```

返回数据格式：
```json
{
  "player_id": "player.testnet",
  "total_games": 25,
  "total_wins": 20,
  "average_attempts": 4.2,
  "best_score": 2,
  "total_time": 3600,
  "win_rate": 80.0,
  "last_played": 1703097600,
  "total_score": 18500
}
```

#### `get_player_games`
获取玩家游戏历史（支持分页）

```bash
near view CONTRACT_ID get_player_games '{
  "player_id": "player.testnet",
  "from_index": 0,
  "limit": 10
}' --networkId testnet
```

### 排行榜

#### `get_leaderboard`
获取全球排行榜

```bash
near view CONTRACT_ID get_leaderboard '{"limit": 10}' --networkId testnet
```

#### `search_players`
搜索玩家

```bash
near view CONTRACT_ID search_players '{"query": "alice"}' --networkId testnet
```

### 系统查询

#### `get_contract_stats`
获取合约统计信息

```bash
near view CONTRACT_ID get_contract_stats --networkId testnet
```

#### `get_recent_games`
获取最近的游戏记录

```bash
near view CONTRACT_ID get_recent_games '{"limit": 20}' --networkId testnet
```

### 管理员功能

#### `add_admin` / `remove_admin`
管理员权限管理（仅 Owner）

```bash
# 添加管理员
near call CONTRACT_ID add_admin '{"admin_id": "admin.testnet"}' --accountId owner.testnet --networkId testnet

# 移除管理员
near call CONTRACT_ID remove_admin '{"admin_id": "admin.testnet"}' --accountId owner.testnet --networkId testnet
```

#### `rebuild_leaderboard_admin`
强制重建排行榜缓存（Admin/Owner）

```bash
near call CONTRACT_ID rebuild_leaderboard_admin --accountId admin.testnet --networkId testnet
```

#### `cleanup_old_records`
清理旧记录（Admin/Owner）

```bash
near call CONTRACT_ID cleanup_old_records '{
  "older_than_timestamp": 1640995200,
  "limit": 100
}' --accountId admin.testnet --networkId testnet
```

## 🧪 测试

### 运行单元测试

```bash
# 运行所有测试
cargo test

# 运行特定测试
cargo test test_store_game_record

# 详细测试输出
cargo test -- --nocapture
```

### 集成测试

```bash
# 使用测试脚本
./scripts/test_contract.sh

# 或手动测试
near call CONTRACT_ID store_game_record '{...test_data...}' --accountId test.testnet --deposit 0.01 --networkId testnet
```

## 💰 存储成本

合约采用"用多少付多少"的存储模型：

- **基础存储费用**: 每个游戏记录约 0.001-0.01 NEAR
- **自动退款**: 多付的费用会自动退还
- **存储优化**: 数据结构经过优化以减少存储成本

### 存储费用估算

```rust
// 每个游戏记录大约占用的存储
// - 基础字段: ~200 bytes
// - 猜测列表: ~4 bytes * 尝试次数
// - 字符串字段: 变长
// 总计: ~300-500 bytes per record
```

## 🔧 开发指南

### 项目结构

```
on-chain-contract/
├── src/
│   └── lib.rs              # 主合约代码
├── Cargo.toml              # Rust 项目配置
├── deploy.sh              # 部署脚本
├── scripts/               # 辅助脚本
│   ├── test_contract.sh   # 测试脚本
│   └── migrate.sh         # 迁移脚本
└── README.md              # 本文件
```

### 数据结构设计

#### GameRecord
```rust
pub struct GameRecord {
    pub game_id: String,        // 游戏唯一标识
    pub player_id: AccountId,   // 玩家账户
    pub target_number: u32,     // 目标数字
    pub attempts: u32,          // 尝试次数
    pub guesses: Vec<u32>,      // 所有猜测
    pub duration_seconds: u64,  // 游戏时长
    pub timestamp: u64,         // 完成时间
    pub success: bool,          // 是否成功
    pub difficulty: String,     // 难度等级
    pub score: u32,            // 游戏得分
}
```

#### PlayerStats
```rust
pub struct PlayerStats {
    pub player_id: AccountId,
    pub total_games: u32,
    pub total_wins: u32,
    pub average_attempts: f64,
    pub best_score: u32,
    pub total_time: u64,
    pub win_rate: f64,
    pub last_played: u64,
    pub total_score: u64,
}
```

### 添加新功能

1. **扩展游戏记录字段**
```rust
// 在 GameRecord 结构体中添加新字段
pub struct GameRecord {
    // ... 现有字段 ...
    pub new_field: SomeType,
}

// 更新验证逻辑
fn validate_game_record(&self, record: &GameRecord) {
    // ... 现有验证 ...
    assert!(/* 新字段验证 */, "New field validation failed");
}
```

2. **添加新的查询方法**
```rust
#[near_bindgen]
impl GuessNumberContract {
    pub fn new_query_method(&self, params: QueryParams) -> QueryResult {
        // 实现新的查询逻辑
    }
}
```

3. **扩展事件系统**
```rust
pub enum GameEvent {
    // ... 现有事件 ...
    NewEventType {
        field1: Type1,
        field2: Type2,
    },
}
```

### 性能优化

1. **批量操作**: 实现批量存储多个游戏记录
2. **索引优化**: 为常用查询添加专门的索引
3. **缓存策略**: 优化排行榜和统计数据的缓存
4. **分页查询**: 所有列表查询都支持分页

## 🚨 安全考虑

### 输入验证
- 所有用户输入都经过严格验证
- 防止数据溢出和无效输入
- 游戏逻辑完整性检查

### 权限控制
- 多级权限管理（Owner > Admin > User）
- 敏感操作需要适当权限
- 防止权限提升攻击

### 存储安全
- 防止存储滥用攻击
- 合理的存储成本设置
- 自动清理机制

### 重入攻击防护
- 状态更新在外部调用之前完成
- 使用 NEAR SDK 的内置安全特性

## 📊 监控和分析

### 事件日志
合约会发出以下事件：

```javascript
// 游戏记录存储事件
{
  "event": "GameRecorded",
  "data": {
    "game_id": "game_12345",
    "player_id": "player.testnet",
    "success": true,
    "attempts": 5,
    "difficulty": "normal"
  }
}

// 新最佳记录事件
{
  "event": "NewBestScore",
  "data": {
    "player_id": "player.testnet",
    "new_best": 3,
    "previous_best": 5
  }
}

// 统计更新事件
{
  "event": "StatsUpdated",
  "data": {
    "player_id": "player.testnet",
    "total_games": 26,
    "win_rate": 84.6
  }
}
```

### 合约指标
- 总游戏数量
- 活跃玩家数量
- 平均游戏时长
- 存储使用情况

## 🔄 升级和迁移

### 合约升级
```bash
# 构建新版本
cargo build --target wasm32-unknown-unknown --release

# 部署新版本（保持状态）
near deploy CONTRACT_ID --wasmFile target/wasm32-unknown-unknown/release/guess_number_contract.wasm --networkId testnet
```

### 数据迁移
```bash
# 使用迁移脚本
./scripts/migrate.sh --from-version 1.0.0 --to-version 2.0.0
```

## 🤝 贡献指南

1. **Fork 项目** 并创建功能分支
2. **编写测试** 确保新功能正常工作
3. **更新文档** 包括 API 文档和使用示例
4. **运行测试** 确保所有测试通过
5. **提交 PR** 并等待代码审查

### 代码规范
- 遵循 Rust 官方代码风格
- 添加充分的文档注释
- 为所有公共方法编写测试
- 使用有意义的变量和函数名

## 📝 许可证

本项目采用 MIT OR Apache-2.0 双重许可证。

## 🆘 支持与帮助

- [GitHub Issues](https://github.com/calimero-network/core/issues)
- [Discord 社区](https://discord.gg/wZRC73DVpU)
- [NEAR 官方文档](https://docs.near.org)

## 📚 相关资源

- [NEAR SDK 文档](https://docs.near.org/sdk/rust/introduction)
- [Rust 智能合约最佳实践](https://docs.near.org/sdk/rust/best-practices)
- [NEAR 存储和成本](https://docs.near.org/concepts/storage/storage-staking)
- [Calimero Network 文档](https://docs.calimero.network)

---

🎮 **开始使用**: `./deploy.sh --account mycontract.testnet`  
📊 **查看统计**: `near view CONTRACT_ID get_contract_stats --networkId testnet`  
🏆 **排行榜**: `near view CONTRACT_ID get_leaderboard --networkId testnet`
