//! NEAR Smart Contract for Guessing Number Game
//!
//! This contract stores game results from the off-chain guessing number game,
//! maintains player statistics, and provides leaderboard functionality.
//!
//! Key features:
//! - Store individual game records
//! - Aggregate player statistics
//! - Global leaderboard
//! - Historical data queries
//! - Admin functions for contract management

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LookupMap, UnorderedMap, Vector};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{
    env, json_types::U128, near_bindgen, AccountId, Balance, BorshStorageKey, PanicOnDefault,
    Promise,
};
use std::collections::HashMap;

/// Storage keys for different collections
#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey {
    GameRecords,
    PlayerStats,
    PlayerGameRecords,
    Leaderboard,
    AdminList,
}

/// Individual game record stored on-chain
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct GameRecord {
    /// Unique game identifier
    pub game_id: String,
    /// Player account ID
    pub player_id: AccountId,
    /// The target number that was generated
    pub target_number: u32,
    /// Number of attempts made
    pub attempts: u32,
    /// List of all guesses made by the player
    pub guesses: Vec<u32>,
    /// Game duration in seconds
    pub duration_seconds: u64,
    /// Unix timestamp when the game was completed
    pub timestamp: u64,
    /// Whether the player won the game
    pub success: bool,
    /// Game difficulty level
    pub difficulty: String,
    /// Optional: Score calculated for this game
    pub score: u32,
}

/// Aggregated statistics for a player
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct PlayerStats {
    /// Player account ID
    pub player_id: AccountId,
    /// Total number of games played
    pub total_games: u32,
    /// Total number of games won
    pub total_wins: u32,
    /// Average number of attempts across all games
    pub average_attempts: f64,
    /// Best score (minimum attempts in a winning game)
    pub best_score: u32,
    /// Total time spent playing (in seconds)
    pub total_time: u64,
    /// Win rate as a percentage
    pub win_rate: f64,
    /// Last game played timestamp
    pub last_played: u64,
    /// Total score accumulated across all games
    pub total_score: u64,
}

/// Leaderboard entry for ranking players
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct LeaderboardEntry {
    /// Rank position (1-based)
    pub rank: u32,
    /// Player account ID
    pub player_id: AccountId,
    /// Total games played
    pub total_games: u32,
    /// Win rate percentage
    pub win_rate: f64,
    /// Average attempts per game
    pub average_attempts: f64,
    /// Best score achieved
    pub best_score: u32,
    /// Total accumulated score
    pub total_score: u64,
}

/// Contract metadata and statistics
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct ContractStats {
    /// Total number of games recorded
    pub total_games: u64,
    /// Total number of unique players
    pub total_players: u64,
    /// Total time spent by all players
    pub total_play_time: u64,
    /// Contract version
    pub version: String,
    /// Contract owner
    pub owner: AccountId,
    /// Storage usage in bytes
    pub storage_usage: u64,
}

/// Events emitted by the contract
#[derive(Serialize)]
#[serde(crate = "near_sdk::serde", tag = "event", content = "data")]
pub enum GameEvent {
    /// New game record stored
    GameRecorded {
        game_id: String,
        player_id: AccountId,
        success: bool,
        attempts: u32,
        difficulty: String,
    },
    /// Player achieved a new best score
    NewBestScore {
        player_id: AccountId,
        new_best: u32,
        previous_best: u32,
    },
    /// Player stats updated
    StatsUpdated {
        player_id: AccountId,
        total_games: u32,
        win_rate: f64,
    },
}

/// Main contract structure
#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct GuessNumberContract {
    /// Contract owner
    pub owner_id: AccountId,
    /// Storage for individual game records
    pub game_records: UnorderedMap<String, GameRecord>,
    /// Storage for aggregated player statistics
    pub player_stats: LookupMap<AccountId, PlayerStats>,
    /// Storage for player's game history (player_id -> list of game_ids)
    pub player_game_records: LookupMap<AccountId, Vector<String>>,
    /// Cached leaderboard data
    pub leaderboard_cache: Vector<LeaderboardEntry>,
    /// Last time leaderboard was updated
    pub leaderboard_last_update: u64,
    /// Admins who can perform certain operations
    pub admins: Vector<AccountId>,
    /// Total number of games recorded
    pub total_games: u64,
    /// Total number of unique players
    pub total_players: u64,
    /// Contract version
    pub version: String,
    /// Storage cost per game record (in yoctoNEAR)
    pub storage_cost_per_record: Balance,
}

#[near_bindgen]
impl GuessNumberContract {
    /// Initialize the contract with owner
    #[init]
    pub fn new(owner_id: AccountId) -> Self {
        Self {
            owner_id: owner_id.clone(),
            game_records: UnorderedMap::new(StorageKey::GameRecords),
            player_stats: LookupMap::new(StorageKey::PlayerStats),
            player_game_records: LookupMap::new(StorageKey::PlayerGameRecords),
            leaderboard_cache: Vector::new(StorageKey::Leaderboard),
            leaderboard_last_update: 0,
            admins: Vector::new(StorageKey::AdminList),
            total_games: 0,
            total_players: 0,
            version: "1.0.0".to_string(),
            storage_cost_per_record: 10_000_000_000_000_000_000_000, // 0.01 NEAR
        }
    }

    /// Store a new game record
    #[payable]
    pub fn store_game_record(&mut self, record: GameRecord) -> String {
        // Validate the caller has paid enough for storage
        let storage_start = env::storage_usage();

        // Validate game record
        self.validate_game_record(&record);

        // Check if game already exists
        assert!(
            !self.game_records.contains_key(&record.game_id),
            "Game record already exists"
        );

        let game_id = record.game_id.clone();
        let player_id = record.player_id.clone();
        let success = record.success;
        let attempts = record.attempts;
        let difficulty = record.difficulty.clone();

        // Store the game record
        self.game_records.insert(&game_id, &record);

        // Update player's game history
        let mut player_games = self
            .player_game_records
            .get(&player_id)
            .unwrap_or_else(|| Vector::new(format!("pg_{}", player_id).as_bytes()));
        player_games.push(&game_id);
        self.player_game_records.insert(&player_id, &player_games);

        // Update or create player statistics
        self.update_player_stats(&record);

        // Update contract statistics
        self.total_games += 1;

        // Calculate storage cost and validate payment
        let storage_end = env::storage_usage();
        let storage_used = storage_end - storage_start;
        let required_cost = Balance::from(storage_used) * env::storage_byte_cost();

        assert!(
            env::attached_deposit() >= required_cost,
            "Insufficient payment for storage. Required: {}, Provided: {}",
            required_cost,
            env::attached_deposit()
        );

        // Refund excess payment
        let refund = env::attached_deposit() - required_cost;
        if refund > 0 {
            Promise::new(env::predecessor_account_id()).transfer(refund);
        }

        // Emit event
        self.emit_event(GameEvent::GameRecorded {
            game_id: game_id.clone(),
            player_id,
            success,
            attempts,
            difficulty,
        });

        // Invalidate leaderboard cache if it's getting old
        let current_time = env::block_timestamp() / 1_000_000_000;
        if current_time - self.leaderboard_last_update > 3600 {
            // Cache is older than 1 hour, clear it
            self.leaderboard_cache.clear();
            self.leaderboard_last_update = 0;
        }

        format!("Game record stored successfully: {}", game_id)
    }

    /// Get player statistics
    pub fn get_player_stats(&self, player_id: AccountId) -> Option<PlayerStats> {
        self.player_stats.get(&player_id)
    }

    /// Get player's game history with pagination
    pub fn get_player_games(
        &self,
        player_id: AccountId,
        from_index: Option<u32>,
        limit: Option<u32>,
    ) -> Vec<GameRecord> {
        let from_index = from_index.unwrap_or(0);
        let limit = limit.unwrap_or(20).min(100); // Max 100 records per call

        if let Some(player_games) = self.player_game_records.get(&player_id) {
            let mut results = Vec::new();
            let total = player_games.len() as u32;

            for i in from_index..std::cmp::min(from_index + limit, total) {
                if let Some(game_id) = player_games.get(total - 1 - i) {
                    // Return in reverse order (newest first)
                    if let Some(record) = self.game_records.get(&game_id) {
                        results.push(record);
                    }
                }
            }

            results
        } else {
            Vec::new()
        }
    }

    /// Get global leaderboard
    pub fn get_leaderboard(&mut self, limit: Option<u32>) -> Vec<LeaderboardEntry> {
        let limit = limit.unwrap_or(10).min(100);
        let current_time = env::block_timestamp() / 1_000_000_000;

        // Check if we need to rebuild the leaderboard cache
        if self.leaderboard_cache.is_empty() || current_time - self.leaderboard_last_update > 1800 {
            self.rebuild_leaderboard();
            self.leaderboard_last_update = current_time;
        }

        let mut results = Vec::new();
        let cache_len = self.leaderboard_cache.len() as u32;

        for i in 0..std::cmp::min(limit, cache_len) {
            if let Some(entry) = self.leaderboard_cache.get(i) {
                results.push(entry);
            }
        }

        results
    }

    /// Search for players by partial account ID
    pub fn search_players(&self, query: String) -> Vec<AccountId> {
        let mut results = Vec::new();
        let query_lower = query.to_lowercase();

        for player_id in self.player_stats.keys() {
            if player_id.as_str().to_lowercase().contains(&query_lower) {
                results.push(player_id);

                if results.len() >= 20 {
                    break; // Limit results to prevent gas issues
                }
            }
        }

        results
    }

    /// Get contract statistics
    pub fn get_contract_stats(&self) -> ContractStats {
        ContractStats {
            total_games: self.total_games,
            total_players: self.total_players,
            total_play_time: self.calculate_total_play_time(),
            version: self.version.clone(),
            owner: self.owner_id.clone(),
            storage_usage: env::storage_usage(),
        }
    }

    /// Get a specific game record
    pub fn get_game_record(&self, game_id: String) -> Option<GameRecord> {
        self.game_records.get(&game_id)
    }

    /// Get multiple game records by IDs
    pub fn get_game_records(&self, game_ids: Vec<String>) -> Vec<Option<GameRecord>> {
        game_ids
            .into_iter()
            .map(|id| self.game_records.get(&id))
            .collect()
    }

    /// Get recent games across all players
    pub fn get_recent_games(&self, limit: Option<u32>) -> Vec<GameRecord> {
        let limit = limit.unwrap_or(20).min(100);
        let mut results = Vec::new();

        // This is not the most efficient way, but works for demo purposes
        // In production, you might want to maintain a separate "recent games" index
        let mut all_records: Vec<GameRecord> = self.game_records.values().collect();
        all_records.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        for record in all_records.into_iter().take(limit as usize) {
            results.push(record);
        }

        results
    }

    // Admin Functions

    /// Add an admin (only owner can call)
    pub fn add_admin(&mut self, admin_id: AccountId) {
        self.assert_owner();

        // Check if already an admin
        for i in 0..self.admins.len() {
            if let Some(existing_admin) = self.admins.get(i) {
                if existing_admin == admin_id {
                    env::panic_str("Account is already an admin");
                }
            }
        }

        self.admins.push(&admin_id);
    }

    /// Remove an admin (only owner can call)
    pub fn remove_admin(&mut self, admin_id: AccountId) {
        self.assert_owner();

        let mut index_to_remove = None;
        for i in 0..self.admins.len() {
            if let Some(existing_admin) = self.admins.get(i) {
                if existing_admin == admin_id {
                    index_to_remove = Some(i);
                    break;
                }
            }
        }

        if let Some(index) = index_to_remove {
            self.admins.swap_remove(index);
        } else {
            env::panic_str("Account is not an admin");
        }
    }

    /// Update storage cost (only owner can call)
    pub fn set_storage_cost(&mut self, cost: U128) {
        self.assert_owner();
        self.storage_cost_per_record = cost.0;
    }

    /// Force rebuild leaderboard (only admin can call)
    pub fn rebuild_leaderboard_admin(&mut self) {
        self.assert_admin();
        self.rebuild_leaderboard();
        self.leaderboard_last_update = env::block_timestamp() / 1_000_000_000;
    }

    /// Clean up old game records (only admin can call)
    pub fn cleanup_old_records(&mut self, older_than_timestamp: u64, limit: u32) -> u32 {
        self.assert_admin();

        let mut removed_count = 0;
        let mut records_to_remove = Vec::new();

        for (game_id, record) in self.game_records.iter() {
            if record.timestamp < older_than_timestamp {
                records_to_remove.push(game_id);
                removed_count += 1;

                if removed_count >= limit {
                    break;
                }
            }
        }

        // Remove the identified records
        for game_id in records_to_remove {
            self.game_records.remove(&game_id);
        }

        removed_count
    }

    // Private helper functions

    fn validate_game_record(&self, record: &GameRecord) {
        assert!(!record.game_id.is_empty(), "Game ID cannot be empty");
        assert!(record.attempts > 0, "Attempts must be greater than 0");
        assert!(record.attempts <= 50, "Too many attempts");
        assert!(
            record.guesses.len() == record.attempts as usize,
            "Guesses length must match attempts"
        );
        assert!(
            record.target_number >= 1,
            "Target number must be at least 1"
        );
        assert!(record.target_number <= 1000, "Target number too large");
        assert!(!record.difficulty.is_empty(), "Difficulty cannot be empty");

        // Validate difficulty levels
        match record.difficulty.as_str() {
            "easy" | "normal" | "hard" => {}
            _ => env::panic_str("Invalid difficulty level"),
        }

        // Validate guesses are within reasonable range
        for &guess in &record.guesses {
            assert!(
                guess >= 1 && guess <= 1000,
                "Invalid guess value: {}",
                guess
            );
        }
    }

    fn update_player_stats(&mut self, record: &GameRecord) {
        let mut stats = self
            .player_stats
            .get(&record.player_id)
            .unwrap_or_else(|| PlayerStats {
                player_id: record.player_id.clone(),
                total_games: 0,
                total_wins: 0,
                average_attempts: 0.0,
                best_score: u32::MAX,
                total_time: 0,
                win_rate: 0.0,
                last_played: 0,
                total_score: 0,
            });

        let is_new_player = stats.total_games == 0;

        // Update stats
        stats.total_games += 1;
        stats.total_time += record.duration_seconds;
        stats.last_played = record.timestamp;
        stats.total_score += record.score as u64;

        if record.success {
            stats.total_wins += 1;
            if stats.best_score == u32::MAX || record.attempts < stats.best_score {
                let previous_best = stats.best_score;
                stats.best_score = record.attempts;

                // Emit new best score event
                if previous_best != u32::MAX {
                    self.emit_event(GameEvent::NewBestScore {
                        player_id: record.player_id.clone(),
                        new_best: record.attempts,
                        previous_best,
                    });
                }
            }
        }

        // Recalculate win rate and average attempts
        stats.win_rate = (stats.total_wins as f64 / stats.total_games as f64) * 100.0;

        // Calculate average attempts using running average
        stats.average_attempts = (stats.average_attempts * (stats.total_games - 1) as f64
            + record.attempts as f64)
            / stats.total_games as f64;

        // Store updated stats
        self.player_stats.insert(&record.player_id, &stats);

        if is_new_player {
            self.total_players += 1;
        }

        // Emit stats updated event
        self.emit_event(GameEvent::StatsUpdated {
            player_id: record.player_id.clone(),
            total_games: stats.total_games,
            win_rate: stats.win_rate,
        });
    }

    fn rebuild_leaderboard(&mut self) {
        self.leaderboard_cache.clear();

        let mut entries: Vec<LeaderboardEntry> = self
            .player_stats
            .values()
            .enumerate()
            .map(|(_, stats)| LeaderboardEntry {
                rank: 0, // Will be set after sorting
                player_id: stats.player_id,
                total_games: stats.total_games,
                win_rate: stats.win_rate,
                average_attempts: stats.average_attempts,
                best_score: if stats.best_score == u32::MAX {
                    0
                } else {
                    stats.best_score
                },
                total_score: stats.total_score,
            })
            .collect();

        // Sort by win rate (descending), then by average attempts (ascending)
        entries.sort_by(|a, b| {
            b.win_rate
                .partial_cmp(&a.win_rate)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    a.average_attempts
                        .partial_cmp(&b.average_attempts)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .then_with(|| b.total_score.cmp(&a.total_score))
        });

        // Assign ranks and store in cache
        for (index, mut entry) in entries.into_iter().enumerate() {
            entry.rank = (index + 1) as u32;
            self.leaderboard_cache.push(&entry);
        }
    }

    fn calculate_total_play_time(&self) -> u64 {
        self.player_stats
            .values()
            .map(|stats| stats.total_time)
            .sum()
    }

    fn assert_owner(&self) {
        assert_eq!(
            env::predecessor_account_id(),
            self.owner_id,
            "Only contract owner can call this method"
        );
    }

    fn assert_admin(&self) {
        let caller = env::predecessor_account_id();

        // Owner is always considered an admin
        if caller == self.owner_id {
            return;
        }

        // Check if caller is in admin list
        for i in 0..self.admins.len() {
            if let Some(admin) = self.admins.get(i) {
                if admin == caller {
                    return;
                }
            }
        }

        env::panic_str("Only contract owner or admin can call this method");
    }

    fn emit_event(&self, event: GameEvent) {
        let event_json = near_sdk::serde_json::to_string(&event)
            .unwrap_or_else(|_| "Failed to serialize event".to_string());

        env::log_str(&format!("EVENT_JSON:{}", event_json));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::{testing_env, AccountId};

    fn get_context(predecessor: AccountId) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder.predecessor_account_id(predecessor);
        builder
    }

    #[test]
    fn test_contract_initialization() {
        let owner = accounts(0);
        let context = get_context(owner.clone());
        testing_env!(context.build());

        let contract = GuessNumberContract::new(owner.clone());

        assert_eq!(contract.owner_id, owner);
        assert_eq!(contract.total_games, 0);
        assert_eq!(contract.total_players, 0);
        assert_eq!(contract.version, "1.0.0");
    }

    #[test]
    fn test_store_game_record() {
        let owner = accounts(0);
        let player = accounts(1);

        let mut context = get_context(owner.clone());
        context.attached_deposit(1_000_000_000_000_000_000_000_000); // 1 NEAR
        testing_env!(context.build());

        let mut contract = GuessNumberContract::new(owner);

        let record = GameRecord {
            game_id: "test_game_1".to_string(),
            player_id: player.clone(),
            target_number: 42,
            attempts: 3,
            guesses: vec![25, 60, 42],
            duration_seconds: 30,
            timestamp: 1234567890,
            success: true,
            difficulty: "normal".to_string(),
            score: 850,
        };

        let result = contract.store_game_record(record.clone());
        assert!(result.contains("Game record stored successfully"));

        // Verify the record was stored
        let stored_record = contract.get_game_record("test_game_1".to_string());
        assert!(stored_record.is_some());

        let stored = stored_record.unwrap();
        assert_eq!(stored.game_id, "test_game_1");
        assert_eq!(stored.player_id, player);
        assert_eq!(stored.success, true);

        // Verify player stats were updated
        let stats = contract.get_player_stats(player).unwrap();
        assert_eq!(stats.total_games, 1);
        assert_eq!(stats.total_wins, 1);
        assert_eq!(stats.win_rate, 100.0);
        assert_eq!(stats.best_score, 3);
    }

    #[test]
    fn test_player_stats_aggregation() {
        let owner = accounts(0);
        let player = accounts(1);

        let mut context = get_context(owner.clone());
        context.attached_deposit(2_000_000_000_000_000_000_000_000); // 2 NEAR
        testing_env!(context.build());

        let mut contract = GuessNumberContract::new(owner);

        // First game - win
        let record1 = GameRecord {
            game_id: "game_1".to_string(),
            player_id: player.clone(),
            target_number: 50,
            attempts: 3,
            guesses: vec![25, 75, 50],
            duration_seconds: 45,
            timestamp: 1234567890,
            success: true,
            difficulty: "normal".to_string(),
            score: 900,
        };

        // Second game - loss
        let record2 = GameRecord {
            game_id: "game_2".to_string(),
            player_id: player.clone(),
            target_number: 25,
            attempts: 10,
            guesses: vec![50, 60, 70, 80, 90, 40, 30, 20, 10, 5],
            duration_seconds: 120,
            timestamp: 1234567950,
            success: false,
            difficulty: "normal".to_string(),
            score: 0,
        };

        contract.store_game_record(record1);
        contract.store_game_record(record2);

        let stats = contract.get_player_stats(player).unwrap();
        assert_eq!(stats.total_games, 2);
        assert_eq!(stats.total_wins, 1);
        assert_eq!(stats.win_rate, 50.0);
        assert_eq!(stats.best_score, 3);
        assert_eq!(stats.average_attempts, 6.5); // (3 + 10) / 2
        assert_eq!(stats.total_time, 165); // 45 + 120
        assert_eq!(stats.total_score, 900);
    }

    #[test]
    fn test_leaderboard() {
        let owner = accounts(0);
        let player1 = accounts(1);
        let player2 = accounts(2);

        let mut context = get_context(owner.clone());
        context.attached_deposit(3_000_000_000_000_000_000_000_000); // 3 NEAR
        testing_env!(context.build());

        let mut contract = GuessNumberContract::new(owner);

        // Player1 - better stats
        let record1 = GameRecord {
            game_id: "game_1".to_string(),
            player_id: player1.clone(),
            target_number: 50,
            attempts: 2,
            guesses: vec![25, 50],
            duration_seconds: 30,
            timestamp: 1234567890,
            success: true,
            difficulty: "normal".to_string(),
            score: 950,
        };

        // Player2 - worse stats
        let record2 = GameRecord {
            game_id: "game_2".to_string(),
            player_id: player2.clone(),
            target_number: 75,
            attempts: 5,
            guesses: vec![50, 60, 70, 80, 75],
            duration_seconds: 90,
            timestamp: 1234567950,
            success: true,
            difficulty: "normal".to_string(),
            score: 700,
        };

        contract.store_game_record(record1);
        contract.store_game_record(record2);

        let leaderboard = contract.get_leaderboard(None);
        assert_eq!(leaderboard.len(), 2);

        // Player1 should be ranked higher (better win rate and fewer attempts)
        assert_eq!(leaderboard[0].player_id, player1);
        assert_eq!(leaderboard[0].rank, 1);
        assert_eq!(leaderboard[1].player_id, player2);
        assert_eq!(leaderboard[1].rank, 2);
    }

    #[test]
    #[should_panic(expected = "Game record already exists")]
    fn test_duplicate_game_id() {
        let owner = accounts(0);
        let player = accounts(1);

        let mut context = get_context(owner.clone());
        context.attached_deposit(2_000_000_000_000_000_000_000_000);
        testing_env!(context.build());

        let mut contract = GuessNumberContract::new(owner);

        let record = GameRecord {
            game_id: "duplicate_game".to_string(),
            player_id: player.clone(),
            target_number: 42,
            attempts: 3,
            guesses: vec![25, 60, 42],
            duration_seconds: 30,
            timestamp: 1234567890,
            success: true,
            difficulty: "normal".to_string(),
            score: 850,
        };

        contract.store_game_record(record.clone());
        contract.store_game_record(record); // Should panic
    }

    #[test]
    #[should_panic(expected = "Invalid difficulty level")]
    fn test_invalid_difficulty() {
        let owner = accounts(0);
        let player = accounts(1);

        let mut context = get_context(owner.clone());
        context.attached_deposit(1_000_000_000_000_000_000_000_000);
        testing_env!(context.build());

        let mut contract = GuessNumberContract::new(owner);

        let record = GameRecord {
            game_id: "invalid_difficulty_game".to_string(),
            player_id: player,
            target_number: 42,
            attempts: 3,
            guesses: vec![25, 60, 42],
            duration_seconds: 30,
            timestamp: 1234567890,
            success: true,
            difficulty: "impossible".to_string(), // Invalid difficulty
            score: 850,
        };

        contract.store_game_record(record);
    }

    #[test]
    fn test_admin_functions() {
        let owner = accounts(0);
        let admin = accounts(1);
        let player = accounts(2);

        let context = get_context(owner.clone());
        testing_env!(context.build());

        let mut contract = GuessNumberContract::new(owner.clone());

        // Add admin
        contract.add_admin(admin.clone());

        // Test admin can rebuild leaderboard
        let context = get_context(admin.clone());
        testing_env!(context.build());
        contract.rebuild_leaderboard_admin();

        // Remove admin
        let context = get_context(owner.clone());
        testing_env!(context.build());
        contract.remove_admin(admin.clone());
    }

    #[test]
    #[should_panic(expected = "Only contract owner can call this method")]
    fn test_non_owner_add_admin() {
        let owner = accounts(0);
        let non_owner = accounts(1);
        let admin_candidate = accounts(2);

        let context = get_context(owner.clone());
        testing_env!(context.build());

        let mut contract = GuessNumberContract::new(owner);

        // Try to add admin from non-owner account
        let context = get_context(non_owner);
        testing_env!(context.build());

        contract.add_admin(admin_candidate); // Should panic
    }
}
