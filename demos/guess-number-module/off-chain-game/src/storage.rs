//! Local storage and caching module for the guessing number game
//!
//! This module provides:
//! - Local file-based storage for game data persistence
//! - In-memory caching for fast data access
//! - Data serialization and backup/restore functionality
//! - Storage configuration and management

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::error::{GameError, GameResult};
use crate::{GameRecord, PlayerStats};

/// Configuration for local storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Directory to store game data
    pub data_dir: PathBuf,
    /// Maximum number of records to keep in memory cache
    pub max_cache_size: usize,
    /// How long to keep data in cache before considering it stale
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

    fn is_expired(&self, ttl: Duration) -> bool {
        self.timestamp.elapsed().unwrap_or(Duration::from_secs(0)) > ttl
    }
}

/// In-memory cache for frequently accessed data
#[derive(Debug)]
struct GameDataCache {
    /// Cached game records by player ID
    records: HashMap<String, CacheEntry<Vec<GameRecord>>>,
    /// Cached player statistics
    stats: HashMap<String, CacheEntry<PlayerStats>>,
    /// Cache configuration
    ttl: Duration,
    max_size: usize,
}

impl GameDataCache {
    fn new(config: &StorageConfig) -> Self {
        Self {
            records: HashMap::new(),
            stats: HashMap::new(),
            ttl: Duration::from_secs(config.cache_ttl_seconds),
            max_size: config.max_cache_size,
        }
    }

    fn get_records(&self, player_id: &str) -> Option<Vec<GameRecord>> {
        self.records
            .get(player_id)
            .filter(|entry| !entry.is_expired(self.ttl))
            .map(|entry| entry.data.clone())
    }

    fn cache_records(&mut self, player_id: String, records: Vec<GameRecord>) {
        self.cleanup_if_needed();
        self.records.insert(player_id, CacheEntry::new(records));
    }

    fn get_stats(&self, player_id: &str) -> Option<PlayerStats> {
        self.stats
            .get(player_id)
            .filter(|entry| !entry.is_expired(self.ttl))
            .map(|entry| entry.data.clone())
    }

    fn cache_stats(&mut self, stats: PlayerStats) {
        self.cleanup_if_needed();
        self.stats.insert(stats.player_id.clone(), CacheEntry::new(stats));
    }

    fn cleanup_expired(&mut self) {
        let ttl = self.ttl;

        self.records.retain(|_, entry| !entry.is_expired(ttl));
        self.stats.retain(|_, entry| !entry.is_expired(ttl));
    }

    fn cleanup_if_needed(&mut self) {
        let total_entries = self.records.len() + self.stats.len();
        if total_entries >= self.max_size {
            self.cleanup_expired();

            // If still too many, remove oldest entries
            if self.records.len() + self.stats.len() >= self.max_size {
                // Simple LRU: remove oldest timestamp entries
                if self.records.len() > self.max_size / 2 {
                    if let Some(oldest_key) = self.find_oldest_records_key() {
                        self.records.remove(&oldest_key);
                    }
                }
                if self.stats.len() > self.max_size / 2 {
                    if let Some(oldest_key) = self.find_oldest_stats_key() {
                        self.stats.remove(&oldest_key);
                    }
                }
            }
        }
    }

    fn find_oldest_records_key(&self) -> Option<String> {
        self.records
            .iter()
            .min_by_key(|(_, entry)| entry.timestamp)
            .map(|(key, _)| key.clone())
    }

    fn find_oldest_stats_key(&self) -> Option<String> {
        self.stats
            .iter()
            .min_by_key(|(_, entry)| entry.timestamp)
            .map(|(key, _)| key.clone())
    }

    fn clear(&mut self) {
        self.records.clear();
        self.stats.clear();
    }
}

/// Local storage manager for game data
pub struct LocalStorage {
    config: StorageConfig,
    cache: Arc<RwLock<GameDataCache>>,
}

impl LocalStorage {
    /// Create a new local storage instance
    pub fn new(config: StorageConfig) -> GameResult<Self> {
        // Create data directory if it doesn't exist
        if !config.data_dir.exists() {
            fs::create_dir_all(&config.data_dir)
                .map_err(|e| GameError::StorageError(format!("Failed to create data directory: {}", e)))?;
        }

        let cache = GameDataCache::new(&config);

        Ok(Self {
            config,
            cache: Arc::new(RwLock::new(cache)),
        })
    }

    /// Store a game record to local storage
    pub async fn store_game_record(&self, record: &GameRecord) -> GameResult<()> {
        debug!("Storing game record locally: {}", record.game_id);

        // Store to file
        self.save_record_to_file(record).await?;

        // Update cache
        self.update_cache_with_record(record).await;

        info!("Game record stored locally: {}", record.game_id);
        Ok(())
    }

    /// Retrieve game records for a player
    pub async fn get_player_records(&self, player_id: &str, limit: Option<usize>) -> GameResult<Vec<GameRecord>> {
        debug!("Retrieving records for player: {}", player_id);

        // Try cache first
        {
            let cache = self.cache.read().await;
            if let Some(cached_records) = cache.get_records(player_id) {
                debug!("Retrieved {} records from cache for {}", cached_records.len(), player_id);
                let limited_records = if let Some(limit) = limit {
                    cached_records.into_iter().take(limit).collect()
                } else {
                    cached_records
                };
                return Ok(limited_records);
            }
        }

        // Load from file storage
        let records = self.load_player_records_from_files(player_id).await?;

        // Cache the results
        {
            let mut cache = self.cache.write().await;
            cache.cache_records(player_id.to_string(), records.clone());
        }

        let limited_records = if let Some(limit) = limit {
            records.into_iter().take(limit).collect()
        } else {
            records
        };

        Ok(limited_records)
    }

    /// Get player statistics
    pub async fn get_player_stats(&self, player_id: &str) -> GameResult<PlayerStats> {
        debug!("Retrieving stats for player: {}", player_id);

        // Try cache first
        {
            let cache = self.cache.read().await;
            if let Some(cached_stats) = cache.get_stats(player_id) {
                debug!("Retrieved stats from cache for {}", player_id);
                return Ok(cached_stats);
            }
        }

        // Calculate from stored records
        let records = self.get_player_records(player_id, None).await?;
        let stats = self.calculate_stats_from_records(player_id, &records);

        // Cache the calculated stats
        {
            let mut cache = self.cache.write().await;
            cache.cache_stats(stats.clone());
        }

        Ok(stats)
    }

    /// Get all player statistics for leaderboard
    pub async fn get_all_player_stats(&self) -> GameResult<HashMap<String, PlayerStats>> {
        debug!("Retrieving all player statistics");

        let all_records = self.load_all_records().await?;
        let mut player_records: HashMap<String, Vec<GameRecord>> = HashMap::new();

        // Group records by player
        for record in all_records {
            player_records
                .entry(record.player_id.clone())
                .or_insert_with(Vec::new)
                .push(record);
        }

        // Calculate stats for each player
        let mut all_stats = HashMap::new();
        for (player_id, records) in player_records {
            let stats = self.calculate_stats_from_records(&player_id, &records);
            all_stats.insert(player_id, stats);
        }

        Ok(all_stats)
    }

    /// Create a backup of all game data
    pub async fn create_backup(&self) -> GameResult<PathBuf> {
        info!("Creating backup of game data");

        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let backup_filename = format!("game_data_backup_{}.json", timestamp);
        let backup_path = self.config.data_dir.join("backups").join(backup_filename);

        // Create backups directory if it doesn't exist
        if let Some(parent) = backup_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| GameError::StorageError(format!("Failed to create backup directory: {}", e)))?;
        }

        // Load all data
        let all_records = self.load_all_records().await?;
        let all_stats = self.get_all_player_stats().await?;

        let backup_data = BackupData {
            records: all_records,
            stats: all_stats,
            created_at: SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        };

        // Write backup file
        let file = File::create(&backup_path)
            .map_err(|e| GameError::StorageError(format!("Failed to create backup file: {}", e)))?;
        let writer = BufWriter::new(file);

        serde_json::to_writer_pretty(writer, &backup_data)
            .map_err(|e| GameError::SerializationError(format!("Failed to serialize backup: {}", e)))?;

        // Clean up old backups
        self.cleanup_old_backups().await?;

        info!("Backup created: {:?}", backup_path);
        Ok(backup_path)
    }

    /// Restore data from a backup file
    pub async fn restore_from_backup(&self, backup_path: &Path) -> GameResult<()> {
        info!("Restoring data from backup: {:?}", backup_path);

        let file = File::open(backup_path)
            .map_err(|e| GameError::StorageError(format!("Failed to open backup file: {}", e)))?;
        let reader = BufReader::new(file);

        let backup_data: BackupData = serde_json::from_reader(reader)
            .map_err(|e| GameError::DeserializationError(format!("Failed to deserialize backup: {}", e)))?;

        // Clear existing cache
        {
            let mut cache = self.cache.write().await;
            cache.clear();
        }

        // Restore records
        for record in backup_data.records {
            self.save_record_to_file(&record).await?;
        }

        info!("Restored {} game records from backup", backup_data.records.len());
        Ok(())
    }

    /// Clear all cached data
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
        info!("Cache cleared");
    }

    /// Get storage statistics
    pub async fn get_storage_stats(&self) -> GameResult<StorageStats> {
        let data_size = self.calculate_data_size().await?;
        let record_count = self.count_total_records().await?;

        let cache = self.cache.read().await;
        let cache_stats = CacheStats {
            cached_players: cache.records.len(),
            cached_stats: cache.stats.len(),
            total_memory_usage: self.estimate_cache_memory_usage(&*cache),
        };

        Ok(StorageStats {
            total_records: record_count,
            total_size_bytes: data_size,
            cache_stats,
            data_directory: self.config.data_dir.clone(),
        })
    }

    // Private helper methods

    async fn save_record_to_file(&self, record: &GameRecord) -> GameResult<()> {
        let records_dir = self.config.data_dir.join("records");
        fs::create_dir_all(&records_dir)
            .map_err(|e| GameError::StorageError(format!("Failed to create records directory: {}", e)))?;

        let filename = format!("{}.json", record.game_id.simple());
        let file_path = records_dir.join(filename);

        let file = File::create(&file_path)
            .map_err(|e| GameError::StorageError(format!("Failed to create record file: {}", e)))?;
        let writer = BufWriter::new(file);

        serde_json::to_writer_pretty(writer, record)
            .map_err(|e| GameError::SerializationError(format!("Failed to serialize record: {}", e)))?;

        Ok(())
    }

    async fn load_player_records_from_files(&self, player_id: &str) -> GameResult<Vec<GameRecord>> {
        let records_dir = self.config.data_dir.join("records");

        if !records_dir.exists() {
            return Ok(Vec::new());
        }

        let entries = fs::read_dir(&records_dir)
            .map_err(|e| GameError::StorageError(format!("Failed to read records directory: {}", e)))?;

        let mut records = Vec::new();

        for entry in entries {
            let entry = entry
                .map_err(|e| GameError::StorageError(format!("Failed to read directory entry: {}", e)))?;
            let path = entry.path();

            if path.extension().map_or(false, |ext| ext == "json") {
                if let Ok(record) = self.load_record_from_file(&path).await {
                    if record.player_id == player_id {
                        records.push(record);
                    }
                }
            }
        }

        // Sort by timestamp (newest first)
        records.sort_by_key(|record| std::cmp::Reverse(record.timestamp));

        Ok(records)
    }

    async fn load_record_from_file(&self, file_path: &Path) -> GameResult<GameRecord> {
        let file = File::open(file_path)
            .map_err(|e| GameError::StorageError(format!("Failed to open record file: {}", e)))?;
        let reader = BufReader::new(file);

        let record: GameRecord = serde_json::from_reader(reader)
            .map_err(|e| GameError::DeserializationError(format!("Failed to deserialize record: {}", e)))?;

        Ok(record)
    }

    async fn load_all_records(&self) -> GameResult<Vec<GameRecord>> {
        let records_dir = self.config.data_dir.join("records");

        if !records_dir.exists() {
            return Ok(Vec::new());
        }

        let entries = fs::read_dir(&records_dir)
            .map_err(|e| GameError::StorageError(format!("Failed to read records directory: {}", e)))?;

        let mut records = Vec::new();

        for entry in entries {
            let entry = entry
                .map_err(|e| GameError::StorageError(format!("Failed to read directory entry: {}", e)))?;
            let path = entry.path();

            if path.extension().map_or(false, |ext| ext == "json") {
                match self.load_record_from_file(&path).await {
                    Ok(record) => records.push(record),
                    Err(e) => {
                        warn!("Failed to load record from {:?}: {}", path, e);
                        continue;
                    }
                }
            }
        }

        Ok(records)
    }

    fn calculate_stats_from_records(&self, player_id: &str, records: &[GameRecord]) -> PlayerStats {
        let mut stats = PlayerStats::new(player_id.to_string());

        for record in records {
            stats.update_with_game(record);
        }

        stats
    }

    async fn update_cache_with_record(&self, record: &GameRecord) {
        let mut cache = self.cache.write().await;

        // Update player's record cache
        if let Some(cached_records) = cache.records.get_mut(&record.player_id) {
            cached_records.data.insert(0, record.clone());
            // Keep only recent records in cache
            if cached_records.data.len() > 50 {
                cached_records.data.truncate(50);
            }
            cached_records.timestamp = SystemTime::now();
        }

        // Update player stats cache
        if let Some(cached_stats) = cache.stats.get_mut(&record.player_id) {
            cached_stats.data.update_with_game(record);
            cached_stats.timestamp = SystemTime::now();
        } else {
            let mut new_stats = PlayerStats::new(record.player_id.clone());
            new_stats.update_with_game(record);
            cache.cache_stats(new_stats);
        }
    }

    async fn calculate_data_size(&self) -> GameResult<u64> {
        let records_dir = self.config.data_dir.join("records");

        if !records_dir.exists() {
            return Ok(0);
        }

        let mut total_size = 0u64;

        let entries = fs::read_dir(&records_dir)
            .map_err(|e| GameError::StorageError(format!("Failed to read records directory: {}", e)))?;

        for entry in entries {
            let entry = entry
                .map_err(|e| GameError::StorageError(format!("Failed to read directory entry: {}", e)))?;

            if let Ok(metadata) = entry.metadata() {
                total_size += metadata.len();
            }
        }

        Ok(total_size)
    }

    async fn count_total_records(&self) -> GameResult<usize> {
        let records_dir = self.config.data_dir.join("records");

        if !records_dir.exists() {
            return Ok(0);
        }

        let entries = fs::read_dir(&records_dir)
            .map_err(|e| GameError::StorageError(format!("Failed to read records directory: {}", e)))?;

        let count = entries
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry.path().extension().map_or(false, |ext| ext == "json")
            })
            .count();

        Ok(count)
    }

    fn estimate_cache_memory_usage(&self, cache: &GameDataCache) -> usize {
        // Rough estimation of memory usage
        let records_size = cache.records.iter()
            .map(|(key, entry)| key.len() + entry.data.len() * 200) // Estimate ~200 bytes per record
            .sum::<usize>();

        let stats_size = cache.stats.len() * 100; // Estimate ~100 bytes per stats entry

        records_size + stats_size
    }

    async fn cleanup_old_backups(&self) -> GameResult<()> {
        let backups_dir = self.config.data_dir.join("backups");

        if !backups_dir.exists() {
            return Ok(());
        }

        let entries = fs::read_dir(&backups_dir)
            .map_err(|e| GameError::StorageError(format!("Failed to read backups directory: {}", e)))?;

        let mut backup_files: Vec<_> = entries
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_name().to_string_lossy().starts_with("game_data_backup_"))
            .collect();

        if backup_files.len() <= self.config.max_backups {
            return Ok(());
        }

        // Sort by modification time (oldest first)
        backup_files.sort_by_key(|entry| {
            entry.metadata()
                .and_then(|m| m.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH)
        });

        // Remove oldest backups
        let to_remove = backup_files.len() - self.config.max_backups;
        for entry in backup_files.iter().take(to_remove) {
            if let Err(e) = fs::remove_file(entry.path()) {
                warn!("Failed to remove old backup {:?}: {}", entry.path(), e);
            } else {
                info!("Removed old backup: {:?}", entry.path());
            }
        }

        Ok(())
    }
}

/// Data structure for backup files
#[derive(Debug, Serialize, Deserialize)]
struct BackupData {
    records: Vec<GameRecord>,
    stats: HashMap<String, PlayerStats>,
    created_at: u64,
    version: String,
}

/// Storage statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct StorageStats {
    pub total_records: usize,
    pub total_size_bytes: u64,
    pub cache_stats: CacheStats,
    pub data_directory: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CacheStats {
    pub cached_players: usize,
    pub cached_stats: usize,
    pub total_memory_usage: usize,
}

/// Storage manager that coordinates local and blockchain storage
pub struct StorageManager {
    local_storage: LocalStorage,
    blockchain_enabled: bool,
}

impl StorageManager {
    /// Create a new storage manager
    pub fn new(config: StorageConfig) -> GameResult<Self> {
        let local_storage = LocalStorage::new(config)?;

        Ok(Self {
            local_storage,
            blockchain_enabled: false, // Will be enabled when Calimero is connected
        })
    }

    /// Store a game record (both locally and optionally on blockchain)
    pub async fn store_game_record(&self, record: &GameRecord) -> GameResult<Option<String>> {
        // Always store locally first
        self.local_storage.store_game_record(record).await?;

        // Store on blockchain if enabled
        if self.blockchain_enabled {
            // TODO: Store to blockchain via Calimero
            // For now, return a mock transaction hash
            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
            Ok(Some(crate::utils::generate_mock_tx_hash()))
        } else {
            Ok(None)
        }
    }

    /// Retrieve player statistics (from cache, local storage, or blockchain)
    pub async fn get_player_stats(&self, player_id: &str) -> GameResult<PlayerStats> {
        self.local_storage.get_player_stats(player_id).await
    }

    /// Retrieve game history for a player
    pub async fn get_player_records(&self, player_id: &str, limit: Option<usize>) -> GameResult<Vec<GameRecord>> {
        self.local_storage.get_player_records(player_id, limit).await
    }

    /// Get storage statistics
    pub async fn get_stats(&self) -> GameResult<StorageStats> {
        self.local_storage.get_storage_stats().await
    }

    /// Enable blockchain storage
    pub fn enable_blockchain_storage(&mut self) {
        self.blockchain_enabled = true;
        info!("Blockchain storage enabled");
    }

    /// Disable blockchain storage
    pub fn disable_blockchain_storage(&mut self) {
        self.blockchain_enabled = false;
        info!("Blockchain storage disabled");
    }

    /// Create a backup of all data
    pub async fn create_backup(&self) -> GameResult<PathBuf> {
        self.local_storage.create_backup().await
    }

    /// Restore from backup
    pub async fn restore_from_backup(&self, backup_path: &Path) -> GameResult<()> {
        self.local_storage.restore_from_backup(backup_path).await
    }
}

/// Utility functions for storage operations
pub mod utils {
    use super::*;

    /// Clean up storage by removing old records
    pub async fn cleanup_old_records(
        storage: &LocalStorage,
        older_than_days: u64,
    ) -> GameResult<usize> {
        let cutoff_time = SystemTime::now() - Duration::from_secs(older_than_days * 24 * 3600);
        let cutoff_timestamp = cutoff_time
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let records_dir = storage.config.data_dir.join("records");

        if !records_dir.exists() {
            return Ok(0);
        }

        let entries = fs::read_dir(&records_dir)
            .map_err(|e| GameError::StorageError(format!("Failed to read records directory: {}", e)))?;

        let mut removed_count = 0;

        for entry in entries {
            let entry = entry
                .map_err(|e| GameError::StorageError(format!("Failed to read directory entry: {}", e)))?;
            let path = entry.path();

            if path.extension().map_or(false, |ext| ext == "json") {
                if let Ok(record) = storage.load_record_from_file(&path).await {
                    if record.timestamp < cutoff_timestamp {
                        if let Err(e) = fs::remove_file(&path) {
                            warn!("Failed to remove old record file {:?}: {}", path, e);
                        } else {
                            removed_count += 1;
                        }
                    }
                }
            }
        }

        if removed_count > 0 {
            info!("Cleaned up {} old record files", removed_count);
            storage.clear_cache().await;
        }

        Ok(removed_count)
    }

    /// Export records to CSV format
    pub async fn export_records_to_csv(
        storage: &LocalStorage,
        output_path: &Path,
    ) -> GameResult<usize> {
        let records = storage.load_all_records().await?;

        let mut csv_content = String::new();
        csv_content.push_str("game_id,player_id,target_number,attempts,success,duration_seconds,difficulty,timestamp\n");

        for record in &records {
            csv_content.push_str(&format!(
                "{},{},{},{},{},{},{},{}\n",
                record.game_id.simple(),
                record.player_id,
                record.target_number,
                record.attempts,
                record.success,
                record.duration_seconds,
                record.difficulty,
                record.timestamp
            ));
        }

        fs::write(output_path, csv_content)
            .map_err(|e| GameError::StorageError(format!("Failed to write CSV file: {}", e)))?;

        Ok(records.len())
    }

    /// Import records from CSV format
    pub async fn import_records_from_csv(
        storage: &LocalStorage,
        csv_path: &Path,
    ) -> GameResult<usize> {
        let csv_content = fs::read_to_string(csv_path)
            .map_err(|e| GameError::StorageError(format!("Failed to read CSV file: {}", e)))?;

        let mut lines = csv_content.lines();
        let _header = lines.next(); // Skip header

        let mut imported_count = 0;

        for line in lines {
            let fields: Vec<&str> = line.split(',').collect();
            if fields.len() != 8 {
                warn!("Skipping invalid CSV line: {}", line);
                continue;
            }

            let game_id = Uuid::parse_str(fields[0])
                .map_err(|e| GameError::ParseError(format!("Invalid game ID: {}", e)))?;

            let record = GameRecord {
                game_id,
                player_id: fields[1].to_string(),
                target_number: fields[2].parse()
                    .map_err(|e| GameError::ParseError(format!("Invalid target number: {}", e)))?,
                attempts: fields[3].parse()
                    .map_err(|e| GameError::ParseError(format!("Invalid attempts: {}", e)))?,
                guesses: Vec::new(), // CSV doesn't include individual guesses
                success: fields[4].parse()
                    .map_err(|e| GameError::ParseError(format!("Invalid success flag: {}", e)))?,
                duration_seconds: fields[5].parse()
                    .map_err(|e| GameError::ParseError(format!("Invalid duration: {}", e)))?,
                difficulty: fields[6].to_string(),
                timestamp: fields[7].parse()
                    .map_err(|e| GameError::ParseError(format!("Invalid timestamp: {}", e)))?,
            };

            storage.store_game_record(&record).await?;
            imported_count += 1;
        }

        if imported_count > 0 {
            info!("Imported {} records from CSV", imported_count);
            storage.clear_cache().await;
        }

        Ok(imported_count)
    }

    /// Validate storage directory structure
    pub async fn validate_storage_structure(data_dir: &Path) -> GameResult<()> {
        if !data_dir.exists() {
            fs::create_dir_all(data_dir)
                .map_err(|e| GameError::StorageError(format!("Failed to create data directory: {}", e)))?;
        }

        let records_dir = data_dir.join("records");
        if !records_dir.exists() {
            fs::create_dir_all(&records_dir)
                .map_err(|e| GameError::StorageError(format!("Failed to create records directory: {}", e)))?;
        }

        let backups_dir = data_dir.join("backups");
        if !backups_dir.exists() {
            fs::create_dir_all(&backups_dir)
                .map_err(|e| GameError::StorageError(format!("Failed to create backups directory: {}", e)))?;
        }

        Ok(())
    }

    /// Get storage usage summary
    pub async fn get_storage_summary(data_dir: &Path) -> GameResult<StorageSummary> {
        let mut summary = StorageSummary::default();

        if !data_dir.exists() {
            return Ok(summary);
        }

        // Count records
        let records_dir = data_dir.join("records");
        if records_dir.exists() {
            if let Ok(entries) = fs::read_dir(&records_dir) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        if entry.path().extension().map_or(false, |ext| ext == "json") {
                            summary.total_records += 1;
                            if let Ok(metadata) = entry.metadata() {
                                summary.records_size_bytes += metadata.len();
                            }
                        }
                    }
                }
            }
        }

        // Count backups
        let backups_dir = data_dir.join("backups");
        if backups_dir.exists() {
            if let Ok(entries) = fs::read_dir(&backups_dir) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        if entry.file_name().to_string_lossy().starts_with("game_data_backup_") {
                            summary.backup_count += 1;
                            if let Ok(metadata) = entry.metadata() {
                                summary.backups_size_bytes += metadata.len();
                            }
                        }
                    }
                }
            }
        }

        summary.total_size_bytes = summary.records_size_bytes + summary.backups_size_bytes;

        Ok(summary)
    }
}

/// Summary of storage usage
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct StorageSummary {
    pub total_records: usize,
    pub records_size_bytes: u64,
    pub backup_count: usize,
    pub backups_size_bytes: u64,
    pub total_size_bytes: u64,
}

impl StorageSummary {
    /// Format size in human-readable format
    pub fn format_size(&self) -> String {
        format_bytes(self.total_size_bytes)
    }

    /// Format records size
    pub fn format_records_size(&self) -> String {
        format_bytes(self.records_size_bytes)
    }

    /// Format backups size
    pub fn format_backups_size(&self) -> String {
        format_bytes(self.backups_size_bytes)
    }
}

/// Format bytes in human-readable format
fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.2} {}", size, UNITS[unit_index])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempdir::TempDir;

    #[tokio::test]
    async fn test_local_storage_operations() {
        let temp_dir = TempDir::new("game_storage_test").unwrap();
        let config = StorageConfig {
            data_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let storage = LocalStorage::new(config).unwrap();

        // Test storing a record
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

        storage.store_game_record(&record).await.unwrap();

        // Test retrieving records
        let retrieved_records = storage.get_player_records("test_player", None).await.unwrap();
        assert_eq!(retrieved_records.len(), 1);
        assert_eq!(retrieved_records[0].game_id, record.game_id);

        // Test retrieving stats
        let stats = storage.get_player_stats("test_player").await.unwrap();
        assert_eq!(stats.total_games, 1);
        assert_eq!(stats.total_wins, 1);
    }

    #[tokio::test]
    async fn test_storage_manager() {
        let temp_dir = TempDir::new("game_storage_manager_test").unwrap();
        let config = StorageConfig {
            data_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let manager = StorageManager::new(config).unwrap();

        let record = GameRecord {
            game_id: Uuid::new_v4(),
            player_id: "manager_test_player".to_string(),
            target_number: 75,
            attempts: 5,
            guesses: vec![50, 80, 70, 72, 75],
            duration_seconds: 60,
            timestamp: 1234567890,
            success: true,
            difficulty: "normal".to_string(),
        };

        let tx_hash = manager.store_game_record(&record).await.unwrap();
        assert!(tx_hash.is_none()); // Blockchain storage disabled by default

        let stats = manager.get_player_stats("manager_test_player").await.unwrap();
        assert_eq!(stats.total_games, 1);
    }

    #[tokio::test]
    async fn test_cache_operations() {
        let config = StorageConfig::default();
        let mut cache = GameDataCache::new(&config);

        let stats = PlayerStats::new("cache_test_player".to_string());
        cache.cache_stats(stats.clone());

        let retrieved_stats = cache.get_stats("cache_test_player").unwrap();
        assert_eq!(retrieved_stats.player_id, stats.player_id);

        // Test cache expiration
        let expired_config = StorageConfig {
            cache_ttl_seconds: 0, // Immediate expiration
            ..Default::default()
        };
        let mut expired_cache = GameDataCache::new(&expired_config);
        expired_cache.cache_stats(stats);

        // Should return None due to immediate expiration
        tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
        assert!(expired_cache.get_stats("cache_test_player").is_none());
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1536), "1.50 KB");
        assert_eq!(format_bytes(1048576), "1.00 MB");
    }

    #[tokio::test]
    async fn test_backup_and_restore() {
        let temp_dir = TempDir::new("backup_test").unwrap();
        let config = StorageConfig {
            data_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let storage = LocalStorage::new(config).unwrap();

        // Store some test data
        let record = GameRecord {
            game_id: Uuid::new_v4(),
            player_id: "backup_test_player".to_string(),
            target_number: 42,
            attempts: 3,
            guesses: vec![25, 60, 42],
            duration_seconds: 30,
            timestamp: 1234567890,
            success: true,
            difficulty: "normal".to_string(),
        };

        storage.store_game_record(&record).await.unwrap();

        // Create backup
        let backup_path = storage.create_backup().await.unwrap();
        assert!(backup_path.exists());

        // Clear data and restore
        let records_dir = storage.config.data_dir.join("records");
        if records_dir.exists() {
            fs::remove_dir_all(&records_dir).unwrap();
        }
        storage.clear_cache().await;

        // Restore from backup
        storage.restore_from_backup(&backup_path).await.unwrap();

        // Verify data was restored
        let restored_records = storage.get_player_records("backup_test_player", None).await.unwrap();
        assert_eq!(restored_records.len(), 1);
        assert_eq!(restored_records[0].game_id, record.game_id);
    }
