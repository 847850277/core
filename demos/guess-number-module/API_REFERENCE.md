# 🔌 Calimero 猜数字游戏 API 参考

## API 概览

本文档描述了猜数字游戏与 Calimero Network 集成的所有 API 接口和方法。

## 目录

- [核心 API](#核心-api)
- [游戏管理](#游戏管理)  
- [数据存储](#数据存储)
- [统计查询](#统计查询)
- [监控接口](#监控接口)
- [错误处理](#错误处理)

---

## 核心 API

### CalimeroClient

主要的 Calimero 网络客户端类。

```rust
pub struct CalimeroClient {
    context_manager: Arc<RwLock<Option<ContextManager>>>,
    context_id: Option<ContextId>,
    identity: Option<Identity>,
}
```

#### 方法

##### `initialize(node_url: &str) -> Result<()>`

初始化 Calimero 客户端连接。

**参数:**
- `node_url`: Calimero 节点 URL

**返回值:**
- `Ok(())`: 连接成功
- `Err(eyre::Error)`: 连接失败

**示例:**
```rust
let mut client = CalimeroClient::new();
client.initialize("http://127.0.0.1:2428").await?;
```

##### `is_connected() -> bool`

检查客户端是否已连接到 Calimero 网络。

**返回值:**
- `true`: 已连接
- `false`: 未连接

##### `context_id() -> Option<&ContextId>`

获取当前上下文 ID。

**返回值:**
- `Some(&ContextId)`: 上下文 ID
- `None`: 未设置上下文

---

## 游戏管理

### Game

游戏实例管理类。

```rust
pub struct Game {
    id: Uuid,
    config: GameConfig,
    target_number: u32,
    attempts: u32,
    guesses: Vec<u32>,
    start_time: SystemTime,
    player_id: String,
    difficulty: String,
}
```

#### 构造方法

##### `new(config: GameConfig, player_id: String, difficulty: String) -> Self`

创建新的游戏实例。

**参数:**
- `config`: 游戏配置
- `player_id`: 玩家标识
- `difficulty`: 难度等级

**示例:**
```rust
let config = GameConfig {
    min_number: 1,
    max_number: 100,
    max_attempts: 10,
};
let game = Game::new(config, "player123".to_string(), "normal".to_string());
```

#### 实例方法

##### `make_guess(guess: u32) -> GameResult`

进行一次猜测。

**参数:**
- `guess`: 猜测的数字

**返回值:**
- `GameResult::TooSmall`: 猜测值太小
- `GameResult::TooLarge`: 猜测值太大  
- `GameResult::Correct`: 猜测正确
- `GameResult::GameOver`: 游戏结束(用完机会)

**示例:**
```rust
let result = game.make_guess(50);
match result {
    GameResult::Correct => println!("恭喜！猜对了！"),
    GameResult::TooSmall => println!("太小了"),
    GameResult::TooLarge => println!("太大了"),
    GameResult::GameOver => println!("游戏结束"),
}
```

##### `to_record(success: bool) -> GameRecord`

将游戏转换为记录对象。

**参数:**
- `success`: 游戏是否成功

**返回值:**
- `GameRecord`: 游戏记录对象

---

## 数据存储

### GameStorage

游戏数据存储管理类。

```rust
pub struct GameStorage {
    client: CalimeroClient,
}
```

#### 方法

##### `store_game_result(record: &GameRecord) -> Result<String>`

将游戏结果存储到 Calimero 网络。

**参数:**
- `record`: 游戏记录对象

**返回值:**
- `Ok(String)`: 交易哈希
- `Err(eyre::Error)`: 存储失败

**示例:**
```rust
let storage = GameStorage::new(client);
let tx_hash = storage.store_game_result(&record).await?;
println!("交易哈希: {}", tx_hash);
```

##### `batch_store_results(records: &[GameRecord]) -> Result<Vec<String>>`

批量存储多个游戏结果。

**参数:**
- `records`: 游戏记录数组

**返回值:**
- `Ok(Vec<String>)`: 交易哈希列表
- `Err(eyre::Error)`: 批量存储失败

**示例:**
```rust
let records = vec![record1, record2, record3];
let tx_hashes = storage.batch_store_results(&records).await?;
```

##### `get_player_stats(player_id: &str) -> Result<PlayerStats>`

获取玩家统计数据。

**参数:**
- `player_id`: 玩家标识

**返回值:**
- `Ok(PlayerStats)`: 玩家统计数据
- `Err(eyre::Error)`: 查询失败

##### `get_game_history(player_id: &str, limit: Option<usize>) -> Result<Vec<GameRecord>>`

获取玩家游戏历史。

**参数:**
- `player_id`: 玩家标识
- `limit`: 可选的结果数量限制

**返回值:**
- `Ok(Vec<GameRecord>)`: 游戏记录列表
- `Err(eyre::Error)`: 查询失败

---

## 数据结构

### GameRecord

```rust
#[derive(Debug, Clone, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct GameRecord {
    pub game_id: Uuid,
    pub player_id: String,
    pub target_number: u32,
    pub attempts: u32,
    pub guesses: Vec<u32>,
    pub duration_seconds: u64,
    pub timestamp: u64,
    pub success: bool,
    pub difficulty: String,
    
    // Calimero 集成字段
    pub context_id: Option<String>,
    pub transaction_hash: Option<String>,
    pub block_height: Option<u64>,
}
```

### PlayerStats

```rust
#[derive(Debug, Clone, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct PlayerStats {
    pub player_id: String,
    pub total_games: u32,
    pub total_wins: u32,
    pub average_attempts: f64,
    pub best_score: u32,
    pub total_time: u64,
    pub win_rate: f64,
    
    // 数据完整性
    pub last_updated: u64,
    pub data_hash: Option<String>,
}
```

### GameConfig

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameConfig {
    pub min_number: u32,
    pub max_number: u32,
    pub max_attempts: u32,
}
```

**预设配置:**

```rust
// 简单模式
let easy_config = GameConfig {
    min_number: 1,
    max_number: 50,
    max_attempts: 8,
};

// 普通模式
let normal_config = GameConfig {
    min_number: 1,
    max_number: 100,
    max_attempts: 10,
};

// 困难模式
let hard_config = GameConfig {
    min_number: 1,
    max_number: 200,
    max_attempts: 12,
};
```

### GameEvent

```rust
#[derive(Debug, Clone, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub enum GameEvent {
    GameStarted {
        game_id: Uuid,
        player_id: String,
        difficulty: String,
        timestamp: u64,
    },
    GameEnded {
        game_id: Uuid,
        result: GameRecord,
    },
    StatisticsUpdated {
        player_id: String,
        stats: PlayerStats,
    },
}
```

---

## 统计查询

### 全局统计

##### `get_global_leaderboard(limit: Option<usize>) -> Result<Vec<PlayerRanking>>`

获取全球排行榜。

**参数:**
- `limit`: 返回结果数量限制

**返回值:**
- 排行榜列表，按胜率和最佳成绩排序

##### `get_difficulty_stats(difficulty: &str) -> Result<DifficultyStats>`

获取特定难度的统计信息。

**参数:**
- `difficulty`: 难度级别 ("easy", "normal", "hard")

**返回值:**
- 难度统计数据

---

## 监控接口

### 健康检查

##### `health_check() -> Json<serde_json::Value>`

系统健康状态检查。

**返回值:**
```json
{
  "status": "healthy",
  "timestamp": "2024-08-28T10:00:00Z",
  "version": "1.0.0",
  "services": {
    "calimero": "connected",
    "database": "healthy",
    "cache": "active"
  }
}
```

### 指标收集

##### `get_metrics() -> Json<serde_json::Value>`

获取系统运行指标。

**返回值:**
```json
{
  "games_total": 1500,
  "successful_stores": 1485,
  "failed_stores": 15,
  "success_rate": 99.0,
  "average_store_time_ms": 250,
  "active_connections": 45
}
```

---

## Web API 端点

### 游戏接口

#### `POST /api/game/start`

开始新游戏。

**请求体:**
```json
{
  "player_id": "player123",
  "difficulty": "normal",
  "max_attempts": 10
}
```

**响应:**
```json
{
  "game_id": "550e8400-e29b-41d4-a716-446655440000",
  "target_range": {
    "min": 1,
    "max": 100
  },
  "max_attempts": 10
}
```

#### `POST /api/game/guess`

提交猜测。

**请求体:**
```json
{
  "game_id": "550e8400-e29b-41d4-a716-446655440000",
  "guess": 42
}
```

**响应:**
```json
{
  "result": "too_small",
  "attempts_used": 3,
  "attempts_remaining": 7,
  "game_over": false
}
```

#### `POST /api/game/{game_id}/finish`

结束游戏并保存结果。

**响应:**
```json
{
  "success": true,
  "tx_hash": "0x1234567890abcdef...",
  "block_height": 12345,
  "game_record": {
    "game_id": "550e8400-e29b-41d4-a716-446655440000",
    "success": true,
    "attempts": 5,
    "duration_seconds": 120
  }
}
```

### 统计接口

#### `GET /api/stats/player/{player_id}`

获取玩家统计。

**响应:**
```json
{
  "player_id": "player123",
  "total_games": 50,
  "total_wins": 42,
  "win_rate": 84.0,
  "average_attempts": 4.2,
  "best_score": 2,
  "total_time": 3600
}
```

#### `GET /api/stats/leaderboard`

获取排行榜。

**查询参数:**
- `limit`: 结果数量限制 (默认: 10)
- `difficulty`: 难度过滤 (可选)

**响应:**
```json
{
  "leaderboard": [
    {
      "rank": 1,
      "player_id": "top_player",
      "win_rate": 95.5,
      "average_attempts": 3.1,
      "total_games": 200
    }
  ]
}
```

#### `GET /api/history/{player_id}`

获取游戏历史。

**查询参数:**
- `limit`: 结果数量限制 (默认: 20)
- `difficulty`: 难度过滤 (可选)

**响应:**
```json
{
  "history": [
    {
      "game_id": "550e8400-e29b-41d4-a716-446655440000",
      "timestamp": 1640995200,
      "success": true,
      "attempts": 4,
      "difficulty": "normal",
      "tx_hash": "0x1234..."
    }
  ]
}
```

---

## 错误处理

### 错误类型

#### `CalimeroError`

Calimero 网络相关错误。

```rust
#[derive(Debug, thiserror::Error)]
pub enum CalimeroError {
    #[error("连接失败: {0}")]
    ConnectionFailed(String),
    
    #[error("上下文未找到: {0}")]
    ContextNotFound(String),
    
    #[error("存储失败: {0}")]
    StorageFailed(String),
    
    #[error("序列化错误: {0}")]
    SerializationError(String),
    
    #[error("网络超时")]
    NetworkTimeout,
}
```

#### HTTP 错误代码

| 状态码 | 描述 | 示例场景 |
|--------|------|----------|
| 200 | 成功 | 操作完成 |
| 400 | 请求错误 | 无效参数 |
| 401 | 未授权 | 身份验证失败 |
| 404 | 未找到 | 游戏或玩家不存在 |
| 429 | 请求过多 | 超出速率限制 |
| 500 | 服务器错误 | 内部错误 |
| 503 | 服务不可用 | Calimero 网络故障 |

### 重试机制

```rust
pub struct RetryConfig {
    pub max_attempts: usize,      // 最大重试次数
    pub base_delay: Duration,     // 基础延迟时间
    pub max_delay: Duration,      // 最大延迟时间
    pub exponential_base: f64,    // 指数退避基数
}

// 默认配置
impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay: Duration::from_millis(1000),
            max_delay: Duration::from_secs(30),
            exponential_base: 2.0,
        }
    }
}
```

---

## 配置选项

### 环境变量

| 变量名 | 描述 | 默认值 |
|--------|------|--------|
| `CALIMERO_NODE_URL` | Calimero 节点地址 | `http://127.0.0.1:2428` |
| `GAME_CONTEXT_ID` | 游戏上下文 ID | `guess-number-game` |
| `LOG_LEVEL` | 日志级别 | `info` |
| `CACHE_TTL_SECONDS` | 缓存过期时间 | `300` |
| `MAX_CONCURRENT_GAMES` | 最大并发游戏数 | `100` |

### 配置文件示例

```toml
# config.toml
[calimero]
node_url = "http://127.0.0.1:2428"
context_name = "guess-number-game"
timeout_seconds = 30
retry_attempts = 3

[game]
difficulties = ["easy", "normal", "hard"]
max_game_duration = 3600
allow_anonymous = true

[storage]
batch_size = 50
encryption_enabled = true
compression_enabled = false

[cache]
ttl_seconds = 300
max_size = 1000

[monitoring]
metrics_enabled = true
health_check_interval = 60
```

---

## 使用示例

### 完整游戏流程

```rust
#[tokio::main]
async fn main() -> eyre::Result<()> {
    // 1. 初始化客户端
    let mut client = CalimeroClient::new();
    client.initialize("http://127.0.0.1:2428").await?;
    
    let storage = GameStorage::new(client);
    
    // 2. 创建游戏
    let config = GameConfig {
        min_number: 1,
        max_number: 100,
        max_attempts: 10,
    };
    let mut game = Game::new(config, "player123".to_string(), "normal".to_string());
    
    // 3. 游戏循环
    loop {
        print!("请输入猜测: ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        if let Ok(guess) = input.trim().parse::<u32>() {
            match game.make_guess(guess) {
                GameResult::Correct => {
                    println!("🎉 恭喜！猜对了！");
                    let record = game.to_record(true);
                    let tx_hash = storage.store_game_result(&record).await?;
                    println!("交易哈希: {}", tx_hash);
                    break;
                }
                GameResult::TooSmall => println!("📈 太小了！"),
                GameResult::TooLarge => println!("📉 太大了！"),
                GameResult::GameOver => {
                    println!("💀 游戏结束！");
                    let record = game.to_record(false);
                    storage.store_game_result(&record).await?;
                    break;
                }
            }
        }
    }
    
    // 4. 显示统计
    let stats = storage.get_player_stats("player123").await?;
    println!("📊 你的统计: 总游戏 {}, 胜率 {:.1}%", 
             stats.total_games, stats.win_rate);
    
    Ok(())
}
```

### Web 服务器示例

```rust
use axum::{routing::get, Router, Json};

async fn create_app() -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/api/game/start", post(start_game))
        .route("/api/game/guess", post(make_guess))
        .route("/api/game/:id/finish", post(finish_game))
        .route("/api/stats/player/:id", get(get_player_stats))
        .route("/api/stats/leaderboard", get(get_leaderboard))
        .route("/api/history/:id", get(get_game_history))
}

#[tokio::main]
async fn main() {
    let app = create_app().await;
    
    println!("🚀 服务器启动在 http://localhost:8080");
    
    axum::Server::bind(&"0.0.0.0:8080".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
```

---

## 版本信息

- **API 版本**: v1.0.0
- **Calimero SDK 版本**: 0.1.0
- **最后更新**: 2024年8月28日

---

## 支持和反馈

- **GitHub Issues**: [提交问题](https://github.com/calimero-network/core/issues)
- **文档更新**: [贡献文档](https://github.com/calimero-network/core/blob/main/CONTRIBUTING.md)
- **社区讨论**: [Discord](https://discord.gg/calimero)
