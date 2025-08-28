# 🚀 Calimero 集成快速指南

## 概述

本指南提供了将猜数字游戏与 Calimero Network 集成的核心步骤和代码示例。

## 核心集成步骤

### 1. 环境配置

```toml
# Cargo.toml 添加依赖
[dependencies]
calimero-sdk = { path = "../../../crates/sdk" }
calimero-primitives = { path = "../../../crates/primitives" }
calimero-context = { path = "../../../crates/context" }
```

### 2. Calimero 客户端初始化

```rust
use calimero_sdk::env;
use calimero_primitives::{ContextId, Identity};

pub struct CalimeroClient {
    context_id: Option<ContextId>,
    identity: Option<Identity>,
}

impl CalimeroClient {
    pub async fn initialize(&mut self, node_url: &str) -> eyre::Result<()> {
        println!("🔗 正在连接 Calimero 网络...");
        
        // 创建身份和连接
        self.identity = Some(self.create_identity().await?);
        let context_id = self.setup_game_context().await?;
        self.context_id = Some(context_id);
        
        println!("✅ 已连接到 Calimero 网络");
        Ok(())
    }
}
```

### 3. 游戏结果存储

```rust
pub async fn store_game_result(record: &GameRecord) -> eyre::Result<String> {
    println!("💾 正在保存游戏结果到区块链...");
    
    // 1. 准备数据
    let game_event = GameEvent::GameEnded {
        game_id: record.game_id,
        result: record.clone(),
    };
    
    // 2. 序列化并加密敏感数据
    let event_data = serde_json::to_vec(&game_event)?;
    
    // 3. 提交到 Calimero
    let result = env::context::call_method(
        &context_id,
        "store_game_result", 
        &event_data
    ).await?;
    
    println!("✅ 游戏记录已保存到区块链！");
    println!("📝 交易哈希: {}", result.tx_hash);
    
    Ok(result.tx_hash)
}
```

### 4. 数据结构定义

```rust
use calimero_sdk::{borsh, serde};
use borsh::{BorshDeserialize, BorshSerialize};

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

### 5. 错误处理和重试

```rust
use std::time::Duration;
use tokio::time::sleep;

pub async fn store_with_retry(record: &GameRecord) -> eyre::Result<String> {
    let max_attempts = 3;
    let mut last_error = None;
    
    for attempt in 0..max_attempts {
        match store_game_result(record).await {
            Ok(tx_hash) => return Ok(tx_hash),
            Err(error) => {
                last_error = Some(error);
                
                if attempt < max_attempts - 1 {
                    let delay = Duration::from_millis(1000 * 2_u64.pow(attempt as u32));
                    println!("⚠️ 尝试 {} 失败，{:?} 后重试...", attempt + 1, delay);
                    sleep(delay).await;
                }
            }
        }
    }
    
    Err(last_error.unwrap())
}
```

## 使用示例

### 基本使用

```rust
#[tokio::main]
async fn main() -> eyre::Result<()> {
    // 1. 初始化 Calimero 客户端
    let mut client = CalimeroClient::new();
    client.initialize("http://127.0.0.1:2428").await?;
    
    // 2. 运行游戏
    let config = GameConfig::default();
    let mut game = Game::new(config, "player123".to_string(), "normal".to_string());
    
    // 3. 游戏结束后存储结果
    let record = game.to_record(true);
    let tx_hash = store_with_retry(&record).await?;
    
    println!("🎉 游戏完成！交易哈希: {}", tx_hash);
    
    Ok(())
}
```

### Web API 集成

```rust
use axum::{Json, extract::Path};

#[axum::debug_handler]
async fn store_game_api(
    Json(record): Json<GameRecord>
) -> Result<Json<serde_json::Value>, StatusCode> {
    match store_with_retry(&record).await {
        Ok(tx_hash) => Ok(Json(json!({
            "success": true,
            "tx_hash": tx_hash
        }))),
        Err(error) => {
            eprintln!("存储失败: {}", error);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
```

## 配置文件

```toml
# config/local.toml
[calimero]
node_url = "http://127.0.0.1:2428"
context_name = "guess-number-game"
timeout_seconds = 30

[storage]
batch_size = 10
cache_ttl_seconds = 300
encryption_enabled = true

[performance]
max_concurrent_games = 100
retry_attempts = 3
```

## 运行命令

```bash
# 开发模式
cargo run --bin guess-number-client play --player "test-player"

# 查看统计
cargo run --bin guess-number-client stats --player "test-player"

# 启动 Web 服务
cargo run --bin guess-number-server

# 运行测试
cargo test integration_tests
```

## 监控和调试

### 日志配置

```rust
use tracing::{info, warn, error, instrument};

#[instrument]
pub async fn store_game_result(record: &GameRecord) -> eyre::Result<String> {
    info!("开始存储游戏记录: {}", record.game_id);
    
    match store_to_calimero(record).await {
        Ok(tx_hash) => {
            info!("存储成功，交易哈希: {}", tx_hash);
            Ok(tx_hash)
        }
        Err(error) => {
            error!("存储失败: {}", error);
            Err(error)
        }
    }
}
```

### 健康检查

```rust
pub async fn health_check() -> Json<serde_json::Value> {
    Json(json!({
        "status": "healthy",
        "calimero_connected": check_calimero_connection().await,
        "version": env!("CARGO_PKG_VERSION")
    }))
}
```

## 故障排除

### 常见问题

1. **连接失败**: 检查 `merod` 节点是否运行
2. **存储超时**: 增加 `timeout_seconds` 配置
3. **权限错误**: 验证身份和上下文设置
4. **序列化错误**: 确认数据结构实现了正确的 traits

### 调试命令

```bash
# 检查 Calimero 节点状态
meroctl node status

# 查看上下文列表
meroctl context list

# 测试连接
curl http://127.0.0.1:2428/health

# 查看日志
tail -f logs/guess-number.log
```

## 性能优化

### 批量操作

```rust
pub async fn batch_store_results(records: &[GameRecord]) -> eyre::Result<Vec<String>> {
    let batch_size = 10;
    let mut results = Vec::new();
    
    for chunk in records.chunks(batch_size) {
        let batch_results = futures::future::try_join_all(
            chunk.iter().map(|record| store_game_result(record))
        ).await?;
        
        results.extend(batch_results);
    }
    
    Ok(results)
}
```

### 缓存策略

```rust
use std::collections::HashMap;
use tokio::sync::RwLock;

pub struct GameCache {
    stats: RwLock<HashMap<String, (PlayerStats, Instant)>>,
}

impl GameCache {
    pub async fn get_cached_stats(&self, player_id: &str) -> Option<PlayerStats> {
        let cache = self.stats.read().await;
        if let Some((stats, cached_at)) = cache.get(player_id) {
            if cached_at.elapsed() < Duration::from_secs(300) {
                return Some(stats.clone());
            }
        }
        None
    }
}
```

---

**快速开始**: 复制上述代码片段，根据注释进行配置即可开始集成。

**完整文档**: 查看 `INTEGRATION_GUIDE.md` 获取详细的实施步骤。
