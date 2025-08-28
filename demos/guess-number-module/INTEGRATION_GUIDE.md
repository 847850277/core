# Calimero SDK集成指南 - 猜数字游戏

## 📖 文档概述

本文档提供了将现有猜数字游戏与 Calimero Network 集成的完整指南。涵盖从环境搭建到生产部署的所有步骤，帮助开发者实现链下游戏逻辑与链上数据存储的混合架构。

## 🎯 集成目标

- **链下游戏体验**：在本地运行游戏逻辑，确保流畅的用户体验
- **链上数据持久化**：将游戏结果和统计数据存储到区块链
- **身份管理**：集成去中心化身份(DID)进行用户认证
- **数据隐私**：保护游戏过程隐私，仅结果上链
- **成本优化**：最小化链上交互，降低 Gas 费用

## 📋 集成步骤概览

### 第一阶段：环境准备 (Phase 1: Environment Setup)
1. **开发环境配置**
2. **Calimero SDK 依赖安装**
3. **网络连接测试**
4. **身份认证设置**

### 第二阶段：客户端连接 (Phase 2: Client Connection)
5. **Calimero 客户端初始化**
6. **上下文管理器设置**
7. **连接状态监控**
8. **错误处理机制**

### 第三阶段：数据处理 (Phase 3: Data Handling)
9. **游戏记录序列化**
10. **数据结构映射**
11. **加密传输配置**
12. **数据验证机制**

### 第四阶段：业务逻辑 (Phase 4: Business Logic)
13. **游戏结果存储逻辑**
14. **统计数据更新**
15. **历史记录查询**
16. **排行榜维护**

### 第五阶段：错误处理 (Phase 5: Error Handling)
17. **网络异常处理**
18. **重试机制实现**
19. **回退策略设计**
20. **用户反馈机制**

### 第六阶段：高级特性 (Phase 6: Advanced Features)
21. **批量操作优化**
22. **缓存策略实现**
23. **性能监控**
24. **安全审计**

### 第七阶段：测试验证 (Phase 7: Testing & Validation)
25. **单元测试编写**
26. **集成测试设计**
27. **端到端测试**
28. **性能基准测试**

### 第八阶段：部署配置 (Phase 8: Deployment Configuration)
29. **生产环境配置**
30. **监控告警设置**
31. **版本管理策略**
32. **文档完善**

## 🔧 详细实施指南

## Phase 1: 环境准备

### Step 1: 开发环境配置

#### 必要工具安装
```bash
# 1. 安装 Rust 工具链
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup update stable
rustup target add wasm32-unknown-unknown

# 2. 安装 Calimero 工具
cargo install meroctl
cargo install merod

# 3. 安装 NEAR CLI (如果使用 NEAR 作为后端)
npm install -g near-cli
```

#### 项目结构调整
```
demos/guess-number-module/
├── off-chain-game/           # 客户端游戏逻辑
│   ├── src/
│   │   ├── main.rs          # 游戏主入口
│   │   ├── game.rs          # 游戏逻辑核心
│   │   ├── calimero.rs      # Calimero 集成层
│   │   └── types.rs         # 数据类型定义
│   └── Cargo.toml
├── on-chain-contract/        # 智能合约
│   ├── src/
│   │   └── lib.rs           # 合约逻辑
│   └── Cargo.toml
└── integration-tests/        # 集成测试
```

### Step 2: Calimero SDK 依赖配置

更新 `Cargo.toml` 文件：

```toml
[dependencies]
# 现有依赖保持不变...

# Calimero SDK 集成
calimero-sdk = { path = "../../../crates/sdk" }
calimero-primitives = { path = "../../../crates/primitives" }
calimero-context = { path = "../../../crates/context" }
calimero-store = { path = "../../../crates/store" }
calimero-network = { path = "../../../crates/network" }

# 额外的网络和加密库
tokio-tungstenite = "0.20"
url = "2.5"
base58 = "0.2"
ed25519-dalek = "2.0"
```

### Step 3: 网络连接测试

创建连接测试脚本：

```bash
#!/bin/bash
# scripts/test-connection.sh

echo "🔗 测试 Calimero 网络连接..."

# 1. 检查本地节点状态
merod --version
echo "✅ Calimero 节点可用"

# 2. 测试网络连接
meroctl node ping --node-url http://127.0.0.1:2428
echo "✅ 网络连接正常"

# 3. 验证上下文创建
meroctl context create --name test-context
echo "✅ 上下文创建成功"

# 4. 清理测试资源
meroctl context delete --name test-context
echo "✅ 连接测试完成"
```

## Phase 2: 客户端连接

### Step 4: Calimero 客户端初始化

创建 `src/calimero.rs` 集成模块：

```rust
use std::sync::Arc;
use tokio::sync::RwLock;
use calimero_sdk::env;
use calimero_primitives::{ContextId, Identity};
use calimero_context::ContextManager;
use eyre::{Result, Context};

pub struct CalimeroClient {
    context_manager: Arc<RwLock<Option<ContextManager>>>,
    context_id: Option<ContextId>,
    identity: Option<Identity>,
}

impl CalimeroClient {
    pub fn new() -> Self {
        Self {
            context_manager: Arc::new(RwLock::new(None)),
            context_id: None,
            identity: None,
        }
    }

    /// 初始化 Calimero 连接
    pub async fn initialize(&mut self, node_url: &str) -> Result<()> {
        println!("🔗 正在连接 Calimero 网络...");
        
        // 1. 创建身份
        self.identity = Some(self.create_identity().await?);
        
        // 2. 连接到节点
        let context_manager = self.connect_to_node(node_url).await
            .context("Failed to connect to Calimero node")?;
        
        // 3. 创建或获取游戏上下文
        let context_id = self.setup_game_context(&context_manager).await?;
        
        *self.context_manager.write().await = Some(context_manager);
        self.context_id = Some(context_id);
        
        println!("✅ 已连接到 Calimero 网络");
        Ok(())
    }

    /// 检查连接状态
    pub async fn is_connected(&self) -> bool {
        self.context_manager.read().await.is_some()
    }

    /// 获取上下文ID
    pub fn context_id(&self) -> Option<&ContextId> {
        self.context_id.as_ref()
    }
}

// 私有方法实现
impl CalimeroClient {
    async fn create_identity(&self) -> Result<Identity> {
        // 创建或加载用户身份
        todo!("实现身份创建逻辑")
    }

    async fn connect_to_node(&self, node_url: &str) -> Result<ContextManager> {
        // 连接到 Calimero 节点
        todo!("实现节点连接逻辑")
    }

    async fn setup_game_context(&self, manager: &ContextManager) -> Result<ContextId> {
        // 设置游戏专用上下文
        todo!("实现上下文设置逻辑")
    }
}
```

### Step 5: 上下文管理器配置

创建游戏专用上下文配置：

```rust
// src/context.rs
use calimero_sdk::types::{ContextConfig, ExecutionEnvironment};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameContextConfig {
    pub max_players: u32,
    pub game_timeout: u64,
    pub result_retention_days: u32,
    pub privacy_level: PrivacyLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PrivacyLevel {
    Public,    // 所有数据公开
    Private,   // 仅结果公开
    Anonymous, // 完全匿名
}

impl Default for GameContextConfig {
    fn default() -> Self {
        Self {
            max_players: 1000,
            game_timeout: 3600, // 1 hour
            result_retention_days: 365,
            privacy_level: PrivacyLevel::Private,
        }
    }
}

pub fn create_game_context_config() -> ContextConfig {
    ContextConfig {
        environment: ExecutionEnvironment::Local,
        // 其他配置项...
    }
}
```

## Phase 3: 数据处理

### Step 6: 数据结构映射

更新游戏数据结构以支持 Calimero 序列化：

```rust
// src/types.rs
use calimero_sdk::{borsh, serde};
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
    // Calimero 专用字段
    pub context_id: Option<String>,
    pub transaction_hash: Option<String>,
    pub block_height: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct PlayerStats {
    pub player_id: String,
    pub total_games: u32,
    pub total_wins: u32,
    pub average_attempts: f64,
    pub best_score: u32,
    pub total_time: u64,
    pub win_rate: f64,
    // 链上数据验证
    pub last_updated: u64,
    pub data_hash: Option<String>,
}

// 游戏事件定义
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

### Step 7: 数据加密和序列化

实现数据保护机制：

```rust
// src/encryption.rs
use calimero_sdk::env;
use eyre::{Result, Context};
use serde::{Deserialize, Serialize};

pub struct DataProtection;

impl DataProtection {
    /// 加密敏感游戏数据
    pub fn encrypt_game_data<T>(data: &T) -> Result<Vec<u8>>
    where
        T: Serialize,
    {
        let serialized = serde_json::to_vec(data)
            .context("Failed to serialize game data")?;
        
        // 使用 Calimero 环境提供的加密功能
        let encrypted = env::crypto::encrypt(&serialized)
            .context("Failed to encrypt game data")?;
        
        Ok(encrypted)
    }

    /// 解密游戏数据
    pub fn decrypt_game_data<T>(encrypted_data: &[u8]) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let decrypted = env::crypto::decrypt(encrypted_data)
            .context("Failed to decrypt game data")?;
        
        let deserialized = serde_json::from_slice(&decrypted)
            .context("Failed to deserialize game data")?;
        
        Ok(deserialized)
    }

    /// 计算数据完整性哈希
    pub fn compute_hash<T>(data: &T) -> Result<String>
    where
        T: Serialize,
    {
        let serialized = serde_json::to_vec(data)?;
        let hash = env::crypto::hash(&serialized);
        Ok(base58::encode(&hash))
    }
}
```

## Phase 4: 业务逻辑实现

### Step 8: 游戏结果存储逻辑

实现核心的 `store_game_result` 方法：

```rust
// src/storage.rs
use crate::{CalimeroClient, GameRecord, GameEvent, DataProtection};
use calimero_sdk::{env, app::Result};
use uuid::Uuid;

pub struct GameStorage {
    client: CalimeroClient,
}

impl GameStorage {
    pub fn new(client: CalimeroClient) -> Self {
        Self { client }
    }

    /// 存储游戏结果到 Calimero
    pub async fn store_game_result(&self, record: &GameRecord) -> Result<String> {
        // 1. 验证连接状态
        if !self.client.is_connected().await {
            return Err("Not connected to Calimero network".into());
        }

        // 2. 准备数据
        let mut record = record.clone();
        record.context_id = self.client.context_id()
            .map(|id| id.to_string());

        // 3. 加密敏感数据
        let encrypted_guesses = DataProtection::encrypt_game_data(&record.guesses)?;
        
        // 4. 创建游戏事件
        let game_event = GameEvent::GameEnded {
            game_id: record.game_id,
            result: record.clone(),
        };

        // 5. 提交到链上
        let transaction_result = self.submit_to_blockchain(&game_event).await?;
        
        // 6. 更新本地记录
        record.transaction_hash = Some(transaction_result.tx_hash.clone());
        record.block_height = Some(transaction_result.block_height);

        // 7. 更新玩家统计
        self.update_player_stats(&record).await?;

        println!("✅ 游戏结果已保存到区块链");
        println!("📝 交易哈希: {}", transaction_result.tx_hash);

        Ok(transaction_result.tx_hash)
    }

    /// 批量存储多个游戏结果
    pub async fn batch_store_results(&self, records: &[GameRecord]) -> Result<Vec<String>> {
        let mut transaction_hashes = Vec::new();
        
        for record in records {
            match self.store_game_result(record).await {
                Ok(hash) => transaction_hashes.push(hash),
                Err(e) => {
                    println!("⚠️ 存储游戏记录 {} 失败: {}", record.game_id, e);
                    // 继续处理其他记录
                }
            }
        }

        Ok(transaction_hashes)
    }
}

// 私有方法
impl GameStorage {
    async fn submit_to_blockchain(&self, event: &GameEvent) -> Result<TransactionResult> {
        // 使用 Calimero SDK 提交交易
        let context_id = self.client.context_id()
            .ok_or("No context available")?;

        // 序列化事件数据
        let event_data = serde_json::to_vec(event)?;

        // 调用智能合约方法
        let result = env::context::call_method(
            context_id,
            "store_game_result",
            &event_data,
        ).await?;

        Ok(result.into())
    }

    async fn update_player_stats(&self, record: &GameRecord) -> Result<()> {
        // 获取当前统计数据
        let mut stats = self.get_player_stats(&record.player_id).await
            .unwrap_or_else(|_| self.create_default_stats(&record.player_id));

        // 更新统计数据
        stats.total_games += 1;
        if record.success {
            stats.total_wins += 1;
        }
        
        // 重新计算平均值
        let total_attempts: u32 = stats.average_attempts as u32 * (stats.total_games - 1) + record.attempts;
        stats.average_attempts = total_attempts as f64 / stats.total_games as f64;
        
        // 更新最佳记录
        if record.success && (stats.best_score == 0 || record.attempts < stats.best_score) {
            stats.best_score = record.attempts;
        }
        
        // 更新胜率
        stats.win_rate = (stats.total_wins as f64 / stats.total_games as f64) * 100.0;
        
        // 存储更新后的统计数据
        self.store_player_stats(&stats).await?;

        Ok(())
    }
}

#[derive(Debug)]
struct TransactionResult {
    tx_hash: String,
    block_height: u64,
}
```

### Step 9: 错误处理和重试机制

实现健壮的错误处理：

```rust
// src/error_handling.rs
use std::time::Duration;
use tokio::time::sleep;
use eyre::{Result, Context};

pub struct RetryConfig {
    pub max_attempts: usize,
    pub base_delay: Duration,
    pub max_delay: Duration,
    pub exponential_base: f64,
}

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

pub async fn retry_with_backoff<F, T, E>(
    mut operation: F,
    config: RetryConfig,
) -> Result<T>
where
    F: FnMut() -> Result<T, E>,
    E: std::error::Error + Send + Sync + 'static,
{
    let mut last_error = None;
    
    for attempt in 0..config.max_attempts {
        match operation() {
            Ok(result) => return Ok(result),
            Err(error) => {
                last_error = Some(error);
                
                if attempt < config.max_attempts - 1 {
                    let delay = calculate_delay(attempt, &config);
                    println!("⚠️ 尝试 {} 失败，{} 秒后重试...", attempt + 1, delay.as_secs());
                    sleep(delay).await;
                }
            }
        }
    }

    Err(eyre::eyre!(
        "操作失败，已重试 {} 次。最后错误: {:?}",
        config.max_attempts,
        last_error.unwrap()
    ))
}

fn calculate_delay(attempt: usize, config: &RetryConfig) -> Duration {
    let delay = config.base_delay.as_millis() as f64 
        * config.exponential_base.powi(attempt as i32);
    
    let delay = Duration::from_millis(delay as u64);
    std::cmp::min(delay, config.max_delay)
}

// 网络错误处理
pub fn handle_network_error(error: &dyn std::error::Error) -> bool {
    let error_str = error.to_string().to_lowercase();
    
    // 可重试的网络错误
    error_str.contains("timeout") ||
    error_str.contains("connection refused") ||
    error_str.contains("network unreachable") ||
    error_str.contains("temporary failure")
}
```

## Phase 5: 高级特性实现

### Step 10: 缓存和性能优化

实现本地缓存机制：

```rust
// src/cache.rs
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{Duration, Instant};
use serde::{Deserialize, Serialize};

pub struct GameCache {
    stats_cache: Arc<RwLock<HashMap<String, CachedPlayerStats>>>,
    history_cache: Arc<RwLock<HashMap<String, CachedGameHistory>>>,
    cache_ttl: Duration,
}

#[derive(Clone)]
struct CachedPlayerStats {
    data: PlayerStats,
    cached_at: Instant,
}

#[derive(Clone)]
struct CachedGameHistory {
    data: Vec<GameRecord>,
    cached_at: Instant,
}

impl GameCache {
    pub fn new(cache_ttl: Duration) -> Self {
        Self {
            stats_cache: Arc::new(RwLock::new(HashMap::new())),
            history_cache: Arc::new(RwLock::new(HashMap::new())),
            cache_ttl,
        }
    }

    /// 缓存玩家统计数据
    pub async fn cache_player_stats(&self, player_id: &str, stats: PlayerStats) {
        let cached_stats = CachedPlayerStats {
            data: stats,
            cached_at: Instant::now(),
        };
        
        self.stats_cache.write().await
            .insert(player_id.to_string(), cached_stats);
    }

    /// 获取缓存的玩家统计数据
    pub async fn get_cached_player_stats(&self, player_id: &str) -> Option<PlayerStats> {
        let cache = self.stats_cache.read().await;
        
        if let Some(cached) = cache.get(player_id) {
            if cached.cached_at.elapsed() < self.cache_ttl {
                return Some(cached.data.clone());
            }
        }
        
        None
    }

    /// 清理过期缓存
    pub async fn cleanup_expired_cache(&self) {
        let mut stats_cache = self.stats_cache.write().await;
        let mut history_cache = self.history_cache.write().await;
        
        stats_cache.retain(|_, cached| {
            cached.cached_at.elapsed() < self.cache_ttl
        });
        
        history_cache.retain(|_, cached| {
            cached.cached_at.elapsed() < self.cache_ttl
        });
    }
}
```

### Step 11: 监控和日志

实现监控系统：

```rust
// src/monitoring.rs
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::time::{Duration, Instant};
use tracing::{info, warn, error};

pub struct GameMetrics {
    total_games: AtomicU64,
    successful_stores: AtomicU64,
    failed_stores: AtomicU64,
    average_store_time: Arc<RwLock<Duration>>,
    last_successful_store: Arc<RwLock<Option<Instant>>>,
}

impl GameMetrics {
    pub fn new() -> Self {
        Self {
            total_games: AtomicU64::new(0),
            successful_stores: AtomicU64::new(0),
            failed_stores: AtomicU64::new(0),
            average_store_time: Arc::new(RwLock::new(Duration::default())),
            last_successful_store: Arc::new(RwLock::new(None)),
        }
    }

    pub fn record_game_start(&self) {
        self.total_games.fetch_add(1, Ordering::Relaxed);
        info!("新游戏开始，总游戏数: {}", self.total_games.load(Ordering::Relaxed));
    }

    pub async fn record_successful_store(&self, duration: Duration) {
        self.successful_stores.fetch_add(1, Ordering::Relaxed);
        *self.last_successful_store.write().await = Some(Instant::now());
        
        // 更新平均存储时间
        let mut avg_time = self.average_store_time.write().await;
        let total_successful = self.successful_stores.load(Ordering::Relaxed) as f64;
        let current_avg = avg_time.as_millis() as f64;
        let new_avg = (current_avg * (total_successful - 1.0) + duration.as_millis() as f64) / total_successful;
        *avg_time = Duration::from_millis(new_avg as u64);

        info!(
            "游戏结果存储成功，用时: {:?}ms，成功率: {:.2}%",
            duration.as_millis(),
            self.success_rate()
        );
    }

    pub fn record_failed_store(&self, error: &str) {
        self.failed_stores.fetch_add(1, Ordering::Relaxed);
        error!("游戏结果存储失败: {}，失败率: {:.2}%", error, 100.0 - self.success_rate());
    }

    pub fn success_rate(&self) -> f64 {
        let total_attempts = self.successful_stores.load(Ordering::Relaxed) + 
                           self.failed_stores.load(Ordering::Relaxed);
        
        if total_attempts == 0 {
            return 100.0;
        }
        
        (self.successful_stores.load(Ordering::Relaxed) as f64 / total_attempts as f64) * 100.0
    }

    pub async fn print_metrics(&self) {
        let avg_time = self.average_store_time.read().await;
        let last_store = self.last_successful_store.read().await;
        
        println!("\n📊 游戏统计指标:");
        println!("   🎮 总游戏数: {}", self.total_games.load(Ordering::Relaxed));
        println!("   ✅ 成功存储: {}", self.successful_stores.load(Ordering::Relaxed));
        println!("   ❌ 失败存储: {}", self.failed_stores.load(Ordering::Relaxed));
        println!("   📈 成功率: {:.2}%", self.success_rate());
        println!("   ⏱️  平均存储时间: {:?}ms", avg_time.as_millis());
        
        if let Some(last) = *last_store {
            println!("   🕐 最后成功存储: {:?} 前", last.elapsed());
        }
    }
}
```

## Phase 6: 测试框架

### Step 12: 集成测试

创建完整的测试套件：

```rust
// tests/integration_tests.rs
use tokio_test;
use tempdir::TempDir;
use std::time::Duration;

mod common;
use common::*;

#[tokio::test]
async fn test_game_result_storage() -> Result<()> {
    let test_env = setup_test_environment().await?;
    
    // 1. 创建测试游戏记录
    let game_record = create_test_game_record("test_player_1", true);
    
    // 2. 存储游戏结果
    let storage = GameStorage::new(test_env.client);
    let tx_hash = storage.store_game_result(&game_record).await?;
    
    // 3. 验证存储结果
    assert!(!tx_hash.is_empty());
    
    // 4. 验证统计数据更新
    let stats = storage.get_player_stats("test_player_1").await?;
    assert_eq!(stats.total_games, 1);
    assert_eq!(stats.total_wins, 1);
    
    Ok(())
}

#[tokio::test]
async fn test_batch_storage() -> Result<()> {
    let test_env = setup_test_environment().await?;
    
    // 创建多个测试记录
    let records = vec![
        create_test_game_record("test_player_2", true),
        create_test_game_record("test_player_2", false),
        create_test_game_record("test_player_2", true),
    ];
    
    let storage = GameStorage::new(test_env.client);
    let tx_hashes = storage.batch_store_results(&records).await?;
    
    assert_eq!(tx_hashes.len(), 3);
    
    // 验证统计数据
    let stats = storage.get_player_stats("test_player_2").await?;
    assert_eq!(stats.total_games, 3);
    assert_eq!(stats.total_wins, 2);
    assert_eq!(stats.win_rate, 66.67);
    
    Ok(())
}

#[tokio::test]
async fn test_error_handling_and_retry() -> Result<()> {
    let mut test_env = setup_test_environment().await?;
    
    // 模拟网络错误
    test_env.simulate_network_error().await;
    
    let game_record = create_test_game_record("test_player_3", true);
    let storage = GameStorage::new(test_env.client);
    
    // 应该重试并最终成功
    let result = storage.store_game_result(&game_record).await;
    
    // 根据重试机制，应该最终成功
    assert!(result.is_ok());
    
    Ok(())
}

#[tokio::test]
async fn test_cache_functionality() -> Result<()> {
    let test_env = setup_test_environment().await?;
    let cache = GameCache::new(Duration::from_secs(60));
    
    // 测试缓存存储和获取
    let stats = create_test_player_stats("test_player_4");
    cache.cache_player_stats("test_player_4", stats.clone()).await;
    
    let cached_stats = cache.get_cached_player_stats("test_player_4").await;
    assert!(cached_stats.is_some());
    assert_eq!(cached_stats.unwrap().player_id, stats.player_id);
    
    Ok(())
}

// 性能基准测试
#[tokio::test]
async fn benchmark_storage_performance() -> Result<()> {
    let test_env = setup_test_environment().await?;
    let storage = GameStorage::new(test_env.client);
    let metrics = GameMetrics::new();
    
    let start_time = Instant::now();
    let num_games = 100;
    
    for i in 0..num_games {
        let record = create_test_game_record(&format!("benchmark_player_{}", i), i % 2 == 0);
        metrics.record_game_start();
        
        let store_start = Instant::now();
        let result = storage.store_game_result(&record).await;
        let store_duration = store_start.elapsed();
        
        match result {
            Ok(_) => metrics.record_successful_store(store_duration).await,
            Err(e) => metrics.record_failed_store(&e.to_string()),
        }
    }
    
    let total_time = start_time.elapsed();
    metrics.print_metrics().await;
    
    println!("基准测试完成:");
    println!("  总时间: {:?}", total_time);
    println!("  平均每个游戏: {:?}", total_time / num_games);
    println!("  每秒游戏数: {:.2}", num_games as f64 / total_time.as_secs_f64());
    
    // 性能断言
    assert!(metrics.success_rate() > 95.0, "成功率应该大于95%");
    assert!(total_time < Duration::from_secs(30), "100个游戏应该在30秒内完成");
    
    Ok(())
}
```

## Phase 7: 部署配置

### Step 13: 生产环境配置

创建生产环境配置文件：

```toml
# config/production.toml
[calimero]
node_url = "https://mainnet.calimero.network"
context_id = "guess-number-production"
timeout_seconds = 30

[network]
max_connections = 100
keepalive_interval = 30
reconnect_attempts = 5

[storage]
batch_size = 50
batch_timeout = 5
cache_ttl_seconds = 300

[security]
encryption_enabled = true
data_validation = true
rate_limiting = true
max_requests_per_minute = 60

[monitoring]
enable_metrics = true
log_level = "info"
metrics_port = 9090

[performance]
worker_threads = 4
max_concurrent_games = 1000
```

### Step 14: 部署脚本

```bash
#!/bin/bash
# deploy.sh

set -e

echo "🚀 开始部署猜数字游戏..."

# 1. 构建项目
echo "📦 构建项目..."
cargo build --release --bin guess-number-client
cargo build --release --bin guess-number-server

# 2. 运行测试
echo "🧪 运行测试..."
cargo test --release

# 3. 构建 Docker 镜像
echo "🐳 构建 Docker 镜像..."
docker build -t guess-number-game:latest .

# 4. 部署到生产环境
echo "🌐 部署到生产环境..."
docker-compose -f docker-compose.prod.yml up -d

# 5. 健康检查
echo "🔍 执行健康检查..."
timeout 60 bash -c '
while [[ "$(curl -s -o /dev/null -w "%{http_code}" http://localhost:8080/health)" != "200" ]]; do
    echo "等待服务启动..."
    sleep 5
done
'

echo "✅ 部署完成！"
echo "🌐 游戏服务: http://localhost:8080"
echo "📊 监控面板: http://localhost:9090"
```

## 📊 监控和运维

### 健康检查端点

```rust
// src/health.rs
use axum::{Json, response::Json as JsonResponse};
use serde_json::json;

pub async fn health_check() -> JsonResponse<serde_json::Value> {
    JsonResponse(json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "version": env!("CARGO_PKG_VERSION"),
        "services": {
            "calimero": check_calimero_connection().await,
            "database": check_database_connection().await,
            "cache": check_cache_status().await
        }
    }))
}

async fn check_calimero_connection() -> &'static str {
    // 检查 Calimero 连接状态
    "connected"
}
```

### 指标收集

```rust
// src/metrics.rs
use prometheus::{Counter, Histogram, register_counter, register_histogram};
use std::sync::Once;

static INIT: Once = Once::new();

pub struct Metrics {
    pub games_total: Counter,
    pub storage_duration: Histogram,
    pub errors_total: Counter,
}

impl Metrics {
    pub fn new() -> Self {
        INIT.call_once(|| {
            // 注册指标
        });

        Self {
            games_total: register_counter!("games_total", "Total number of games played").unwrap(),
            storage_duration: register_histogram!("storage_duration_seconds", "Time taken to store game results").unwrap(),
            errors_total: register_counter!("errors_total", "Total number of errors").unwrap(),
        }
    }
}
```

## 🔒 安全考虑

### 数据验证

```rust
// src/validation.rs
use eyre::{Result, bail};

pub fn validate_game_record(record: &GameRecord) -> Result<()> {
    // 基本字段验证
    if record.player_id.is_empty() {
        bail!("玩家ID不能为空");
    }
    
    if record.attempts == 0 {
        bail!("尝试次数必须大于0");
    }
    
    if record.guesses.len() != record.attempts as usize {
        bail!("猜测记录数量与尝试次数不匹配");
    }
    
    // 游戏逻辑验证
    if let Some(&last_guess) = record.guesses.last() {
        if record.success && last_guess != record.target_number {
            bail!("成功标记与最后猜测不符");
        }
    }
    
    // 时间验证
    let max_game_duration = 3600; // 1小时
    if record.duration_seconds > max_game_duration {
        bail!("游戏时长超过最大允许时间");
    }
    
    Ok(())
}
```

### 权限管理

```rust
// src/auth.rs
use calimero_primitives::Identity;

pub struct AuthManager {
    allowed_identities: Vec<Identity>,
}

impl AuthManager {
    pub fn verify_player_identity(&self, identity: &Identity) -> bool {
        // 验证玩家身份
        self.allowed_identities.contains(identity)
    }
    
    pub fn check_rate_limit(&self, player_id: &str) -> bool {
        // 检查速率限制
        true // 实现具体逻辑
    }
}
```

## 📝 使用指南

### 1. 快速开始

```bash
# 安装和运行
git clone https://github.com/calimero-network/core.git
cd core/demos/guess-number-module/off-chain-game

# 安装依赖
cargo build

# 运行游戏
cargo run --bin guess-number-client play --player "your-player-id"
```

### 2. 高级配置

```bash
# 自定义难度和尝试次数
cargo run --bin guess-number-client play --difficulty hard --max-attempts 15

# 查看统计数据
cargo run --bin guess-number-client stats --player "your-player-id"

# 查看历史记录
cargo run --bin guess-number-client history --player "your-player-id"
```

### 3. Web 界面

```bash
# 启动 Web 服务
cargo run --bin guess-number-server

# 访问游戏界面
open http://localhost:8080
```

## 🔧 故障排除

### 常见问题

1. **连接失败**
   ```bash
   # 检查 Calimero 节点状态
   meroctl node status
   
   # 重启本地节点
   merod restart
   ```

2. **存储失败**
   ```bash
   # 检查上下文状态
   meroctl context list
   
   # 重新创建上下文
   meroctl context create --name guess-number-game
   ```

3. **性能问题**
   ```bash
   # 查看日志
   tail -f logs/guess-number.log
   
   # 检查资源使用
   docker stats guess-number-game
   ```

## 🚀 后续开发计划

### 短期目标 (1-2 个月)
- [ ] 完善错误处理和日志记录
- [ ] 实现更多游戏模式 (多人模式、竞速模式)
- [ ] 优化存储性能和批量操作
- [ ] 添加游戏内购和奖励机制

### 长期目标 (3-6 个月)
- [ ] 集成更多区块链网络 (Ethereum, Polygon)
- [ ] 实现跨链游戏资产转移
- [ ] 开发移动端应用
- [ ] 构建游戏生态系统

## 📚 参考资源

- [Calimero Network 文档](https://docs.calimero.network)
- [Calimero SDK 参考](https://sdk.calimero.network)
- [NEAR Protocol 开发指南](https://docs.near.org)
- [Rust 异步编程](https://rust-lang.github.io/async-book/)

---

**版本**: 1.0.0  
**最后更新**: 2024年8月28日  
**维护者**: Calimero 开发团队
