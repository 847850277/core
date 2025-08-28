//! Local storage and caching module for the guessing number game
//!
//! This module provides:
//! - Local file-based storage for game data persistence
//! - In-memory caching for fast data access
//! - Data serialization and backup/restore functionality
//! - Storage configuration and management
//!
//! Note: This is currently mocked/disabled to avoid compilation issues

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use tracing::{debug, info};


use crate::error::{GameError, GameResult};
use crate::{GameRecord, PlayerStats};

/// Configuration for local storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Directory to store game data
    pub data_dir: PathBuf,
    /// Maximum number of records to keep in memory cache
    pub max_cache_size: usize,
    /// How long to keep data in cache before considering it stale (seconds)
    pub cache_ttl_seconds: u64,
    /// Whether to enable automatic backup
    pub auto_backup: bool,
    /// Backup interval in seconds
    pub backup_interval_seconds: u64,
    /// Maximum number of backup files to keep
    pub max_backups: usize,
    /// Whether to compress backup files
    pub compress_backups: bool,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from("./game_data"),
            max_cache_size: 10000,
            cache_ttl_seconds: 3600, // 1 hour
            auto_backup: true,
            backup_interval_seconds: 86400, // 24 hours
            max_backups: 7,
            compress_backups: false,
        }
    }
}

/// Cache entry with timestamp for TTL management
#[derive(Debug, Clone)]
struct CacheEntry<T> {
    data: T,
    timestamp: SystemTime,
}

impl<T> CacheEntry<T> {
    fn new(data: T) -> Self {
        Self {
            data,
            timestamp: SystemTime::now(),
        }
    }

    fn is_expired(&self, ttl_seconds: u64) -> bool {
        self.timestamp
            .elapsed()
            .unwrap_or_default()
            .as_secs() > ttl_seconds
    }
}

/// In-memory cache for frequently accessed data
#[derive(Debug, Default)]
struct GameDataCache {
    /// Cached game records by player ID
    records: HashMap<String, CacheEntry<Vec<GameRecord>>>,
    /// Cached player statistics
    stats: HashMap<String, CacheEntry<PlayerStats>>,
    /// Cache configuration
    ttl_seconds: u64,
    max_size: usize,
}

impl GameDataCache {
    fn new(config: &StorageConfig) -> Self {
        Self {
            records: HashMap::new(),
            stats: HashMap::new(),
            ttl_seconds: config.cache_ttl_seconds,
            max_size: config.max_cache_size,
        }
    }

    fn get_records(&self, player_id: &str) -> Option<Vec<GameRecord>> {
        self.records.get(player_id)
            .filter(|entry| !entry.is_expired(self.ttl_seconds))
            .map(|entry| entry.data.clone())
    }

    fn put_records(&mut self, player_id: &str, records: Vec<GameRecord>) {
        self.cleanup_if_needed();
        self.records.insert(
            player_id.to_string(),
            CacheEntry::new(records),
        );
    }

    fn get_stats(&self, player_id: &str) -> Option<PlayerStats> {
        self.stats.get(player_id)
            .filter(|entry| !entry.is_expired(self.ttl_seconds))
            .map(|entry| entry.data.clone())
    }

    fn put_stats(&mut self, player_id: &str, stats: PlayerStats) {
        self.cleanup_if_needed();
        self.stats.insert(
            player_id.to_string(),
            CacheEntry::new(stats),
        );
    }

    fn cleanup_if_needed(&mut self) {
        let total_size = self.records.len() + self.stats.len();
        if total_size >= self.max_size {
            // Remove expired entries first
            let ttl_seconds = self.ttl_seconds;
            self.records.retain(|_, entry| !entry.is_expired(ttl_seconds));
            self.stats.retain(|_, entry| !entry.is_expired(ttl_seconds));

            // If still over limit, remove oldest entries
            if self.records.len() + self.stats.len() >= self.max_size {
                // Simple LRU - remove 10% of entries (simplified)
                let remove_count = self.max_size / 10;
                let keys_to_remove: Vec<_> = self.records.keys().take(remove_count).cloned().collect();
                for key in keys_to_remove {
                    self.records.remove(&key);
                }

                let keys_to_remove: Vec<_> = self.stats.keys().take(remove_count).cloned().collect();
                for key in keys_to_remove {
                    self.stats.remove(&key);
                }
            }
        }
    }

    fn clear(&mut self) {
        self.records.clear();
        self.stats.clear();
    }

    fn size(&self) -> usize {
        self.records.len() + self.stats.len()
    }
}

/// Local storage provider for game data
pub struct LocalStorage {
    config: StorageConfig,
    cache: GameDataCache,
}

impl LocalStorage {
    /// Create a new local storage instance
    pub fn new(config: StorageConfig) -> GameResult<Self> {
        let cache = GameDataCache::new(&config);

        // Ensure data directory exists
        if !config.data_dir.exists() {
            fs::create_dir_all(&config.data_dir).map_err(|e| {
                GameError::StorageError(format!("Failed to create data directory: {}", e))
            })?;
        }

        Ok(Self {
            config,
            cache,
        })
    }

    /// Create a new local storage instance with default configuration
    pub fn new_default() -> GameResult<Self> {
        Self::new(StorageConfig::default())
    }

    /// Store a game record locally
    pub async fn store_game_record(&mut self, record: &GameRecord) -> GameResult<()> {
        info!("Storing game record locally: {}", record.game_id);

        // TODO: Implement actual file storage
        // This would involve:
        // 1. Serializing the record to JSON
        // 2. Writing to a file in the data directory
        // 3. Updating indexes for fast retrieval

        // For now, just update the cache
        let mut player_records = self.get_player_records_from_cache(&record.player_id);
        player_records.push(record.clone());
        self.cache.put_records(&record.player_id, player_records);

        // Update player stats in cache
        let mut stats = self.get_player_stats_from_cache(&record.player_id);
        stats.update_with_game(record);
        self.cache.put_stats(&record.player_id, stats);

        debug!("Game record stored locally: {}", record.game_id);
        Ok(())
    }

    /// Retrieve game records for a player
    pub async fn get_player_records(&mut self, player_id: &str) -> GameResult<Vec<GameRecord>> {
        debug!("Retrieving records for player: {}", player_id);

        // Check cache first
        if let Some(records) = self.cache.get_records(player_id) {
            debug!("Retrieved {} records from cache for player: {}", records.len(), player_id);
            return Ok(records);
        }

        // TODO: Load from file storage
        // This would involve:
        // 1. Reading the player's records file
        // 2. Deserializing from JSON
        // 3. Updating cache

        // For now, return empty records
        let records = Vec::new();
        self.cache.put_records(player_id, records.clone());
        Ok(records)
    }

    /// Get player statistics
    pub async fn get_player_stats(&mut self, player_id: &str) -> GameResult<PlayerStats> {
        debug!("Retrieving stats for player: {}", player_id);

        // Check cache first
        if let Some(stats) = self.cache.get_stats(player_id) {
            debug!("Retrieved stats from cache for player: {}", player_id);
            return Ok(stats);
        }

        // TODO: Calculate from stored records or load from stats file
        // For now, return default stats
        let stats = PlayerStats::new(player_id.to_string());
        self.cache.put_stats(player_id, stats.clone());
        Ok(stats)
    }

    /// Get all player IDs with stored data
    pub async fn get_all_player_ids(&self) -> GameResult<Vec<String>> {
        debug!("Retrieving all player IDs");

        // TODO: Implement by scanning the data directory
        // For now, return cached player IDs
        let mut player_ids: Vec<_> = self.cache.records.keys().cloned().collect();
        let stats_ids: Vec<_> = self.cache.stats.keys().cloned().collect();

        for id in stats_ids {
            if !player_ids.contains(&id) {
                player_ids.push(id);
            }
        }

        Ok(player_ids)
    }

    /// Create a backup of all data
    pub async fn create_backup(&self) -> GameResult<String> {
        info!("Creating data backup...");

        // TODO: Implement actual backup creation
        // This would involve:
        // 1. Creating a backup directory with timestamp
        // 2. Copying all data files
        // 3. Optionally compressing the backup

        let backup_name = format!("backup_{}", chrono::Utc::now().format("%Y%m%d_%H%M%S"));
        info!("Backup created (mocked): {}", backup_name);
        Ok(backup_name)
    }

    /// Restore from a backup
    pub async fn restore_backup(&mut self, backup_name: &str) -> GameResult<()> {
        info!("Restoring from backup: {}", backup_name);

        // TODO: Implement actual backup restoration
        // This would involve:
        // 1. Locating the backup directory
        // 2. Extracting if compressed
        // 3. Replacing current data files
        // 4. Clearing cache to force reload

        self.cache.clear();
        info!("Backup restored (mocked): {}", backup_name);
        Ok(())
    }

    /// Get storage statistics
    pub fn get_storage_stats(&self) -> StorageStats {
        StorageStats {
            cache_size: self.cache.size(),
            cache_hit_ratio: 0.0, // TODO: Track hit/miss ratio
            data_dir: self.config.data_dir.clone(),
            total_records: self.cache.records.values().map(|entry| entry.data.len()).sum(),
            cached_players: self.cache.stats.len(),
        }
    }

    /// Clear all cached data
    pub fn clear_cache(&mut self) {
        self.cache.clear();
        debug!("Storage cache cleared");
    }

    // Private helper methods
    fn get_player_records_from_cache(&self, player_id: &str) -> Vec<GameRecord> {
        self.cache.get_records(player_id).unwrap_or_default()
    }

    fn get_player_stats_from_cache(&self, player_id: &str) -> PlayerStats {
        self.cache.get_stats(player_id).unwrap_or_else(|| PlayerStats::new(player_id.to_string()))
    }
}

/// Storage statistics
#[derive(Debug, Serialize)]
pub struct StorageStats {
    pub cache_size: usize,
    pub cache_hit_ratio: f64,
    pub data_dir: PathBuf,
    pub total_records: usize,
    pub cached_players: usize,
}

/// Storage provider trait for abstracting different storage backends
pub trait StorageProvider: Send + Sync {
    fn store_record(&mut self, record: &GameRecord) -> impl std::future::Future<Output = GameResult<()>> + Send;
    fn get_records(&mut self, player_id: &str) -> impl std::future::Future<Output = GameResult<Vec<GameRecord>>> + Send;
    fn get_stats(&mut self, player_id: &str) -> impl std::future::Future<Output = GameResult<PlayerStats>> + Send;
}

impl StorageProvider for LocalStorage {
    async fn store_record(&mut self, record: &GameRecord) -> GameResult<()> {
        self.store_game_record(record).await
    }

    async fn get_records(&mut self, player_id: &str) -> GameResult<Vec<GameRecord>> {
        self.get_player_records(player_id).await
    }

    async fn get_stats(&mut self, player_id: &str) -> GameResult<PlayerStats> {
        self.get_player_stats(player_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempdir::TempDir;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_local_storage_creation() {
        let temp_dir = TempDir::new("game_test").unwrap();
        let config = StorageConfig {
            data_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let storage = LocalStorage::new(config);
        assert!(storage.is_ok());
    }

    #[tokio::test]
    async fn test_store_and_retrieve_record() {
        let temp_dir = TempDir::new("game_test").unwrap();
        let config = StorageConfig {
            data_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let mut storage = LocalStorage::new(config).unwrap();

        let record = GameRecord::new(
            Uuid::new_v4(),
            "test_player".to_string(),
            42,
            3,
            vec![25, 60, 42],
            30,
            true,
            "normal".to_string(),
        );

        // Store record
        let result = storage.store_game_record(&record).await;
        assert!(result.is_ok());

        // Retrieve records
        let records = storage.get_player_records("test_player").await.unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].game_id, record.game_id);
    }

    #[tokio::test]
    async fn test_player_stats_update() {
        let temp_dir = TempDir::new("game_test").unwrap();
        let config = StorageConfig {
            data_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let mut storage = LocalStorage::new(config).unwrap();

        let record = GameRecord::new(
            Uuid::new_v4(),
            "test_player".to_string(),
            42,
            3,
            vec![25, 60, 42],
            30,
            true,
            "normal".to_string(),
        );

        // Store record (which should update stats)
        storage.store_game_record(&record).await.unwrap();

        // Retrieve stats
        let stats = storage.get_player_stats("test_player").await.unwrap();
        assert_eq!(stats.total_games, 1);
        assert_eq!(stats.total_wins, 1);
        assert_eq!(stats.win_rate, 100.0);
    }

    #[test]
    fn test_cache_operations() {
        let config = StorageConfig::default();
        let mut cache = GameDataCache::new(&config);

        // Test stats caching
        let stats = PlayerStats::new("test_player".to_string());
        cache.put_stats("test_player", stats.clone());

        let retrieved_stats = cache.get_stats("test_player");
        assert!(retrieved_stats.is_some());
        assert_eq!(retrieved_stats.unwrap().player_id, "test_player");

        // Test cache clearing
        cache.clear();
        assert_eq!(cache.size(), 0);
    }
}
