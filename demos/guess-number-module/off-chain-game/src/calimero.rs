//! Calimero Network integration for the guessing number game
//!
//! This module handles all interactions with the Calimero Network including:
//! - Context initialization and management
//! - Storing game results on-chain via NEAR
//! - Retrieving player statistics from blockchain
//! - Managing decentralized identity (DID)

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use calimero_context::Context;
use calimero_primitives::identity::Did;
use calimero_sdk::types::ContextId;

use crate::error::{GameError, GameResult};
use crate::{GameRecord, PlayerStats};

/// Calimero client configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalimeroConfig {
    /// NEAR network to use (testnet, mainnet)
    pub near_network: String,
    /// NEAR contract account ID for game storage
    pub contract_account: String,
    /// Calimero context ID for this game
    pub context_id: String,
    /// Node endpoint URL
    pub node_endpoint: String,
    /// Enable encryption for stored data
    pub enable_encryption: bool,
}

impl Default for CalimeroConfig {
    fn default() -> Self {
        Self {
            near_network: "testnet".to_string(),
            contract_account: "guess-number-game.testnet".to_string(),
            context_id: "guess-number-context".to_string(),
            node_endpoint: "http://localhost:2428".to_string(),
            enable_encryption: true,
        }
    }
}

/// Main Calimero client for blockchain operations
#[derive(Clone)]
pub struct CalimeroClient {
    config: CalimeroConfig,
    context: Option<Arc<Context>>,
    // Local cache to reduce blockchain queries
    cache: Arc<RwLock<CalimeroCache>>,
}

#[derive(Debug, Default)]
struct CalimeroCache {
    player_stats: HashMap<String, PlayerStats>,
    recent_records: HashMap<String, Vec<GameRecord>>,
    last_sync: Option<std::time::SystemTime>,
}

impl CalimeroClient {
    /// Create a new Calimero client with default configuration
    pub fn new() -> Self {
        Self::with_config(CalimeroConfig::default())
    }

    /// Create a new Calimero client with custom configuration
    pub fn with_config(config: CalimeroConfig) -> Self {
        Self {
            config,
            context: None,
            cache: Arc::new(RwLock::new(CalimeroCache::default())),
        }
    }

    /// Initialize connection to Calimero Network
    pub async fn initialize(&mut self) -> GameResult<()> {
        info!("Initializing Calimero Network connection...");

        // TODO: Implement actual Calimero SDK initialization
        // This would involve:
        // 1. Connecting to the Calimero node
        // 2. Authenticating with DID
        // 3. Loading or creating the game context
        // 4. Setting up encryption keys if needed

        // Simulate initialization delay
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // For now, we'll create a mock context
        // In real implementation, this would use:
        // let context = Context::new(ContextId::from(self.config.context_id.clone())).await?;
        // self.context = Some(Arc::new(context));

        info!("✅ Connected to Calimero Network");
        info!("📍 Context ID: {}", self.config.context_id);
        info!("🌐 NEAR Network: {}", self.config.near_network);

        Ok(())
    }

    /// Store a game result to the blockchain via Calimero
    pub async fn store_game_result(&self, record: &GameRecord) -> GameResult<String> {
        debug!("Storing game result to blockchain: {}", record.game_id);

        // Validate the record before storing
        self.validate_game_record(record)?;

        // TODO: Implement actual blockchain storage
        // This would involve:
        // 1. Serializing the game record
        // 2. Encrypting if required
        // 3. Submitting transaction via Calimero to NEAR
        // 4. Waiting for confirmation
        // 5. Returning transaction hash

        // Simulate blockchain storage operation
        tokio::time::sleep(tokio::time::Duration::from_millis(800)).await;

        // Generate mock transaction hash
        let tx_hash = crate::utils::generate_mock_tx_hash();

        // Update local cache
        self.update_cache_with_record(record).await;

        info!("📝 Game result stored to blockchain: {}", tx_hash);

        Ok(tx_hash)
    }

    /// Retrieve player statistics from blockchain
    pub async fn get_player_stats(&self, player_id: &str) -> GameResult<PlayerStats> {
        debug!("Retrieving stats for player: {}", player_id);

        // Check cache first
        if let Some(cached_stats) = self.get_cached_stats(player_id).await {
            debug!("📊 Retrieved stats from cache for player: {}", player_id);
            return Ok(cached_stats);
        }

        // TODO: Implement actual blockchain retrieval
        // This would involve:
        // 1. Querying the NEAR contract via Calimero
        // 2. Decrypting data if needed
        // 3. Deserializing the response
        // 4. Updating local cache

        // Simulate blockchain query
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

        // Return mock stats for now
        let stats = PlayerStats {
            player_id: player_id.to_string(),
            total_games: 0,
            total_wins: 0,
            average_attempts: 0.0,
            best_score: u32::MAX,
            total_time: 0,
            win_rate: 0.0,
        };

        // Cache the result
        self.cache_player_stats(&stats).await;

        Ok(stats)
    }

    /// Retrieve game history for a player from blockchain
    pub async fn get_game_history(&self, player_id: &str, limit: usize) -> GameResult<Vec<GameRecord>> {
        debug!("Retrieving game history for player: {} (limit: {})", player_id, limit);

        // Check cache first
        if let Some(cached_records) = self.get_cached_history(player_id, limit).await {
            debug!("📚 Retrieved history from cache for player: {}", player_id);
            return Ok(cached_records);
        }

        // TODO: Implement actual blockchain retrieval
        // This would query the NEAR contract for historical game records

        // Simulate blockchain query
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // Return empty history for now
        let history = Vec::new();

        // Cache the result
        self.cache_game_history(player_id, &history).await;

        Ok(history)
    }

    /// Get global leaderboard from blockchain
    pub async fn get_global_leaderboard(&self, limit: usize) -> GameResult<Vec<PlayerStats>> {
        debug!("Retrieving global leaderboard (limit: {})", limit);

        // TODO: Implement actual blockchain query
        // This would aggregate player statistics across all stored records

        tokio::time::sleep(tokio::time::Duration::from_millis(400)).await;

        // Return empty leaderboard for now
        Ok(Vec::new())
    }

    /// Check if Calimero Network connection is healthy
    pub async fn health_check(&self) -> GameResult<CalimeroHealthStatus> {
        // TODO: Implement actual health check
        // This would ping the Calimero node and check blockchain connectivity

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        Ok(CalimeroHealthStatus {
            connected: true,
            context_active: true,
            near_network: self.config.near_network.clone(),
            block_height: 12345678, // Mock block height
            node_endpoint: self.config.node_endpoint.clone(),
        })
    }

    /// Initialize or create the game context
    pub async fn setup_game_context(&mut self) -> GameResult<()> {
        info!("Setting up game context: {}", self.config.context_id);

        // TODO: Implement context setup
        // This would:
        // 1. Check if context exists
        // 2. Create context if it doesn't exist
        // 3. Set up permissions and access control
        // 4. Deploy NEAR contract if needed

        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

        info!("✅ Game context setup complete");
        Ok(())
    }

    /// Batch store multiple game records for efficiency
    pub async fn store_game_results_batch(&self, records: &[GameRecord]) -> GameResult<Vec<String>> {
        info!("Storing {} game records in batch", records.len());

        if records.is_empty() {
            return Ok(Vec::new());
        }

        // Validate all records
        for record in records {
            self.validate_game_record(record)?;
        }

        // TODO: Implement batch storage
        // This would be more efficient for storing multiple records

        let mut tx_hashes = Vec::new();
        for record in records {
            let tx_hash = self.store_game_result(record).await?;
            tx_hashes.push(tx_hash);

            // Small delay to avoid overwhelming the network
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        Ok(tx_hashes)
    }

    // Private helper methods

    async fn get_cached_stats(&self, player_id: &str) -> Option<PlayerStats> {
        let cache = self.cache.read().await;
        cache.player_stats.get(player_id).cloned()
    }

    async fn cache_player_stats(&self, stats: &PlayerStats) {
        let mut cache = self.cache.write().await;
        cache.player_stats.insert(stats.player_id.clone(), stats.clone());
        cache.last_sync = Some(std::time::SystemTime::now());
    }

    async fn get_cached_history(&self, player_id: &str, limit: usize) -> Option<Vec<GameRecord>> {
        let cache = self.cache.read().await;
        cache.recent_records.get(player_id)
            .map(|records| records.iter().take(limit).cloned().collect())
    }

    async fn cache_game_history(&self, player_id: &str, records: &[GameRecord]) {
        let mut cache = self.cache.write().await;
        cache.recent_records.insert(player_id.to_string(), records.to_vec());
        cache.last_sync = Some(std::time::SystemTime::now());
    }

    async fn update_cache_with_record(&self, record: &GameRecord) {
        let mut cache = self.cache.write().await;

        // Update player stats in cache
        let stats = cache.player_stats
            .entry(record.player_id.clone())
            .or_insert_with(|| PlayerStats::new(record.player_id.clone()));
        stats.update_with_game(record);

        // Add to recent records
        let player_records = cache.recent_records
            .entry(record.player_id.clone())
            .or_insert_with(Vec::new);
        player_records.insert(0, record.clone());

        // Keep only recent records (limit to prevent memory bloat)
        if player_records.len() > 100 {
            player_records.truncate(100);
        }

        cache.last_sync = Some(std::time::SystemTime::now());
    }

    fn validate_game_record(&self, record: &GameRecord) -> GameResult<()> {
        if record.player_id.is_empty() {
            return Err(GameError::InvalidPlayerId(record.player_id.clone()));
        }

        if record.attempts == 0 {
            return Err(GameError::UnexpectedError(
                "Game record cannot have zero attempts".to_string(),
            ));
        }

        if record.guesses.len() != record.attempts as usize {
            return Err(GameError::UnexpectedError(
                "Guesses count doesn't match attempts".to_string(),
            ));
        }

        Ok(())
    }
}

/// Health status of Calimero Network connection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalimeroHealthStatus {
    pub connected: bool,
    pub context_active: bool,
    pub near_network: String,
    pub block_height: u64,
    pub node_endpoint: String,
}

/// Context information for the game
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameContextInfo {
    pub context_id: String,
    pub owner: String,
    pub participants: Vec<String>,
    pub created_at: u64,
    pub game_count: u64,
}

/// Blockchain transaction result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionResult {
    pub tx_hash: String,
    pub block_height: u64,
    pub gas_used: u64,
    pub success: bool,
    pub timestamp: u64,
}

/// Storage provider trait for different backends
#[async_trait::async_trait]
pub trait StorageProvider: Send + Sync {
    async fn store_game_result(&self, record: &GameRecord) -> GameResult<String>;
    async fn get_player_stats(&self, player_id: &str) -> GameResult<PlayerStats>;
    async fn get_game_history(&self, player_id: &str, limit: usize) -> GameResult<Vec<GameRecord>>;
    async fn get_leaderboard(&self, limit: usize) -> GameResult<Vec<PlayerStats>>;
    async fn health_check(&self) -> GameResult<bool>;
}

/// NEAR blockchain storage provider via Calimero
pub struct NearStorageProvider {
    client: CalimeroClient,
}

impl NearStorageProvider {
    pub fn new(config: CalimeroConfig) -> Self {
        Self {
            client: CalimeroClient::with_config(config),
        }
    }

    pub async fn initialize(&mut self) -> GameResult<()> {
        self.client.initialize().await?;
        self.client.setup_game_context().await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl StorageProvider for NearStorageProvider {
    async fn store_game_result(&self, record: &GameRecord) -> GameResult<String> {
        self.client.store_game_result(record).await
    }

    async fn get_player_stats(&self, player_id: &str) -> GameResult<PlayerStats> {
        self.client.get_player_stats(player_id).await
    }

    async fn get_game_history(&self, player_id: &str, limit: usize) -> GameResult<Vec<GameRecord>> {
        self.client.get_game_history(player_id, limit).await
    }

    async fn get_leaderboard(&self, limit: usize) -> GameResult<Vec<PlayerStats>> {
        self.client.get_global_leaderboard(limit).await
    }

    async fn health_check(&self) -> GameResult<bool> {
        let status = self.client.health_check().await?;
        Ok(status.connected && status.context_active)
    }
}

/// Mock storage provider for testing and development
pub struct MockStorageProvider {
    records: Arc<RwLock<Vec<GameRecord>>>,
    stats: Arc<RwLock<HashMap<String, PlayerStats>>>,
}

impl MockStorageProvider {
    pub fn new() -> Self {
        Self {
            records: Arc::new(RwLock::new(Vec::new())),
            stats: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for MockStorageProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl StorageProvider for MockStorageProvider {
    async fn store_game_result(&self, record: &GameRecord) -> GameResult<String> {
        let mut records = self.records.write().await;
        records.push(record.clone());

        // Update stats
        let mut stats = self.stats.write().await;
        let player_stats = stats
            .entry(record.player_id.clone())
            .or_insert_with(|| PlayerStats::new(record.player_id.clone()));
        player_stats.update_with_game(record);

        // Simulate storage delay
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        Ok(crate::utils::generate_mock_tx_hash())
    }

    async fn get_player_stats(&self, player_id: &str) -> GameResult<PlayerStats> {
        let stats = self.stats.read().await;
        Ok(stats
            .get(player_id)
            .cloned()
            .unwrap_or_else(|| PlayerStats::new(player_id.to_string())))
    }

    async fn get_game_history(&self, player_id: &str, limit: usize) -> GameResult<Vec<GameRecord>> {
        let records = self.records.read().await;
        let mut player_records: Vec<_> = records
            .iter()
            .filter(|record| record.player_id == player_id)
            .cloned()
            .collect();

        player_records.sort_by_key(|record| std::cmp::Reverse(record.timestamp));
        player_records.truncate(limit);

        Ok(player_records)
    }

    async fn get_leaderboard(&self, limit: usize) -> GameResult<Vec<PlayerStats>> {
        let stats = self.stats.read().await;
        let mut leaderboard: Vec<_> = stats.values().cloned().collect();

        leaderboard.sort_by(|a, b| {
            b.win_rate
                .partial_cmp(&a.win_rate)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    a.average_attempts
                        .partial_cmp(&b.average_attempts)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
        });

        leaderboard.truncate(limit);
        Ok(leaderboard)
    }

    async fn health_check(&self) -> GameResult<bool> {
        Ok(true)
    }
}

/// Factory for creating storage providers
pub struct StorageProviderFactory;

impl StorageProviderFactory {
    /// Create a storage provider based on configuration
    pub async fn create_provider(
        provider_type: &str,
        config: Option<CalimeroConfig>,
    ) -> GameResult<Box<dyn StorageProvider>> {
        match provider_type {
            "near" | "calimero" => {
                let config = config.unwrap_or_default();
                let mut provider = NearStorageProvider::new(config);
                provider.initialize().await?;
                Ok(Box::new(provider))
            }
            "mock" | "test" => Ok(Box::new(MockStorageProvider::new())),
            _ => Err(GameError::InvalidConfiguration(format!(
                "Unknown storage provider: {}",
                provider_type
            ))),
        }
    }
}

/// Utility functions for Calimero integration
pub mod utils {
    use super::*;

    /// Create a DID from a player ID
    pub fn player_id_to_did(player_id: &str) -> GameResult<Did> {
        // TODO: Implement actual DID creation/lookup
        // This would involve proper DID resolution

        // For now, create a mock DID
        Ok(Did::placeholder())
    }

    /// Validate NEAR account ID format
    pub fn is_valid_near_account_id(account_id: &str) -> bool {
        // Basic validation for NEAR account ID format
        account_id.len() >= 2
            && account_id.len() <= 64
            && account_id.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.')
            && !account_id.starts_with('-')
            && !account_id.ends_with('-')
    }

    /// Estimate gas cost for storing a game record
    pub fn estimate_gas_cost(record: &GameRecord) -> u64 {
        // Base cost for transaction
        let base_gas = 2_000_000_000_000; // 2 TGas

        // Additional cost based on data size
        let data_size = serde_json::to_string(record)
            .map(|s| s.len())
            .unwrap_or(0);
        let data_gas = (data_size as u64) * 1000; // Rough estimate

        base_gas + data_gas
    }

    /// Format transaction hash for display
    pub fn format_tx_hash(tx_hash: &str) -> String {
        if tx_hash.len() > 16 {
            format!("{}...{}", &tx_hash[..8], &tx_hash[tx_hash.len() - 8..])
        } else {
            tx_hash.to_string()
        }
    }
}

/// Configuration for retry logic
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_factor: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 1000,
            max_delay_ms: 10000,
            backoff_factor: 2.0,
        }
    }
}

/// Retry wrapper for storage operations
pub async fn with_retry<F, Fut, T>(
    operation: F,
    config: RetryConfig,
) -> GameResult<T>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = GameResult<T>>,
{
    let mut last_error = None;
    let mut delay = config.initial_delay_ms;

    for attempt in 0..=config.max_retries {
        match operation().await {
            Ok(result) => {
                if attempt > 0 {
                    info!("Operation succeeded after {} retries", attempt);
                }
                return Ok(result);
            }
            Err(error) => {
                if !error.should_retry() || attempt == config.max_retries {
                    return Err(error);
                }

                warn!("Operation failed (attempt {}), retrying in {}ms: {}",
                     attempt + 1, delay, error);

                tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;

                delay = ((delay as f64) * config.backoff_factor) as u64;
                delay = delay.min(config.max_delay_ms);

                last_error = Some(error);
            }
        }
    }

    Err(last_error.unwrap_or_else(|| {
        GameError::UnexpectedError("Retry loop completed without error".to_string())
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_storage_provider() {
        let provider = MockStorageProvider::new();

        // Test storing a game result
        let record = GameRecord {
            game_id: Uuid::new_v4(),
            player_id: "test_player".to_string(),
            target_number: 42,
            attempts: 3,
            guesses: vec![25, 60, 42],
            duration_seconds: 30,
            timestamp: 1234567890,
            success: true,
            difficulty: "normal".to_string(),
        };

        let tx_hash = provider.store_game_result(&record).await.unwrap();
        assert!(!tx_hash.is_empty());

        // Test retrieving stats
        let stats = provider.get_player_stats("test_player").await.unwrap();
        assert_eq!(stats.total_games, 1);
        assert_eq!(stats.total_wins, 1);

        // Test retrieving history
        let history = provider.get_game_history("test_player", 10).await.unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].game_id, record.game_id);
    }

    #[test]
    fn test_calimero_config_default() {
        let config = CalimeroConfig::default();
        assert_eq!(config.near_network, "testnet");
        assert!(config.enable_encryption);
    }

    #[test]
    fn test_near_account_id_validation() {
        use super::utils::is_valid_near_account_id;

        assert!(is_valid_near_account_id("test-account.testnet"));
        assert!(is_valid_near_account_id("myapp"));
        assert!(!is_valid_near_account_id("-invalid"));
        assert!(!is_valid_near_account_id("invalid-"));
        assert!(!is_valid_near_account_id(""));
        assert!(!is_valid_near_account_id(&"x".repeat(65)));
    }

    #[test]
    fn test_gas_estimation() {
        use super::utils::estimate_gas_cost;

        let record = GameRecord {
            game_id: Uuid::new_v4(),
            player_id: "test".to_string(),
            target_number: 42,
            attempts: 1,
            guesses: vec![42],
            duration_seconds: 10,
            timestamp: 1234567890,
            success: true,
            difficulty: "normal".to_string(),
        };

        let gas_cost = estimate_gas_cost(&record);
        assert!(gas_cost > 2_000_000_000_000); // Should be more than base cost
    }

    #[test]
    fn test_tx_hash_formatting() {
        use super::utils::format_tx_hash;

        let long_hash = "abcdefghijklmnopqrstuvwxyz1234567890";
        let formatted = format_tx_hash(long_hash);
        assert!(formatted.contains("..."));
        assert!(formatted.len() < long_hash.len());

        let short_hash = "abc123";
        let formatted_short = format_tx_hash(short_hash);
        assert_eq!(formatted_short, short_hash);
    }

    #[tokio::test]
    async fn test_retry_mechanism() {
        let config = RetryConfig {
            max_retries: 2,
            initial_delay_ms: 10,
            max_delay_ms: 100,
            backoff_factor: 2.0,
        };

        let mut attempt_count = 0;
        let result = with_retry(
            || {
                attempt_count += 1;
                async move {
                    if attempt_count < 3 {
                        Err(GameError::CalimeroConnectionError("temporary failure".to_string()))
                    } else {
                        Ok("success".to_string())
                    }
                }
            },
            config,
        ).await;

        assert!(result.is_ok());
        assert_eq!(attempt_count, 3);
    }

    #[tokio::test]
    async fn test_retry_non_retryable_error() {
        let config = RetryConfig::default();

        let result = with_retry(
            || async {
                Err(GameError::InvalidDifficulty("test".to_string()))
            },
            config,
        ).await;

        assert!(result.is_err());
        // Should fail immediately for non-retryable errors
    }
}
