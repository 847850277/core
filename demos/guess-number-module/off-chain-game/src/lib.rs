//! Guessing Number Game Library
//!
//! This library provides the core functionality for the guessing number game
//! with Calimero Network integration for blockchain storage.

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

pub mod calimero;
pub mod error;
pub mod game;
pub mod storage;

pub use error::GameError;

/// Game configuration defining the rules and constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameConfig {
    pub min_number: u32,
    pub max_number: u32,
    pub max_attempts: u32,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            min_number: 1,
            max_number: 100,
            max_attempts: 10,
        }
    }
}

impl GameConfig {
    /// Create a new game configuration for different difficulty levels
    pub fn for_difficulty(difficulty: &str, max_attempts: Option<u32>) -> Self {
        match difficulty {
            "easy" => Self {
                min_number: 1,
                max_number: 50,
                max_attempts: max_attempts.unwrap_or(8),
            },
            "hard" => Self {
                min_number: 1,
                max_number: 200,
                max_attempts: max_attempts.unwrap_or(12),
            },
            _ => Self {
                min_number: 1,
                max_number: 100,
                max_attempts: max_attempts.unwrap_or(10),
            },
        }
    }

    /// Validate if a guess is within the valid range
    pub fn is_valid_guess(&self, guess: u32) -> bool {
        guess >= self.min_number && guess <= self.max_number
    }
}

/// A complete game record that can be stored on-chain
#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

impl GameRecord {
    /// Create a new game record
    pub fn new(
        game_id: Uuid,
        player_id: String,
        target_number: u32,
        attempts: u32,
        guesses: Vec<u32>,
        duration_seconds: u64,
        success: bool,
        difficulty: String,
    ) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            game_id,
            player_id,
            target_number,
            attempts,
            guesses,
            duration_seconds,
            timestamp,
            success,
            difficulty,
        }
    }

    /// Get the score for this game (lower is better, 0 if unsuccessful)
    pub fn score(&self) -> u32 {
        if self.success {
            // Score is inversely related to attempts (fewer attempts = higher score)
            self.attempts
        } else {
            0
        }
    }
}

/// Player statistics aggregated from multiple games
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerStats {
    pub player_id: String,
    pub total_games: u32,
    pub total_wins: u32,
    pub average_attempts: f64,
    pub best_score: u32, // Minimum attempts in a successful game
    pub total_time: u64, // Total time spent in seconds
    pub win_rate: f64,   // Percentage of won games
}

impl PlayerStats {
    pub fn new(player_id: String) -> Self {
        Self {
            player_id,
            total_games: 0,
            total_wins: 0,
            average_attempts: 0.0,
            best_score: u32::MAX,
            total_time: 0,
            win_rate: 0.0,
        }
    }

    /// Update statistics with a new game record
    pub fn update_with_game(&mut self, record: &GameRecord) {
        self.total_games += 1;
        self.total_time += record.duration_seconds;

        if record.success {
            self.total_wins += 1;
            if self.best_score == u32::MAX || record.attempts < self.best_score {
                self.best_score = record.attempts;
            }
        }

        self.win_rate = (self.total_wins as f64 / self.total_games as f64) * 100.0;

        // Recalculate average attempts (this would be more efficient if we tracked total attempts)
        // For simplicity, we'll approximate this
        if self.total_games == 1 {
            self.average_attempts = record.attempts as f64;
        } else {
            // Simple running average approximation
            self.average_attempts = (self.average_attempts * (self.total_games - 1) as f64
                + record.attempts as f64)
                / self.total_games as f64;
        }
    }
}

/// The result of a single guess attempt
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GuessResult {
    TooSmall,
    TooLarge,
    Correct,
    GameOver,
}

impl GuessResult {
    pub fn to_string(&self) -> &'static str {
        match self {
            GuessResult::TooSmall => "too_small",
            GuessResult::TooLarge => "too_large",
            GuessResult::Correct => "correct",
            GuessResult::GameOver => "game_over",
        }
    }

    pub fn is_game_ending(&self) -> bool {
        matches!(self, GuessResult::Correct | GuessResult::GameOver)
    }
}

/// Game difficulty levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Difficulty {
    Easy,
    Normal,
    Hard,
}

impl std::str::FromStr for Difficulty {
    type Err = GameError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "easy" => Ok(Difficulty::Easy),
            "normal" => Ok(Difficulty::Normal),
            "hard" => Ok(Difficulty::Hard),
            _ => Err(GameError::InvalidDifficulty(s.to_string())),
        }
    }
}

impl std::fmt::Display for Difficulty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Difficulty::Easy => write!(f, "easy"),
            Difficulty::Normal => write!(f, "normal"),
            Difficulty::Hard => write!(f, "hard"),
        }
    }
}

/// Leaderboard entry combining player stats with ranking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderboardEntry {
    pub rank: u32,
    pub player_id: String,
    pub total_games: u32,
    pub win_rate: f64,
    pub average_attempts: f64,
    pub best_score: u32,
}

impl From<(u32, &PlayerStats)> for LeaderboardEntry {
    fn from((rank, stats): (u32, &PlayerStats)) -> Self {
        Self {
            rank,
            player_id: stats.player_id.clone(),
            total_games: stats.total_games,
            win_rate: stats.win_rate,
            average_attempts: stats.average_attempts,
            best_score: if stats.best_score == u32::MAX {
                0
            } else {
                stats.best_score
            },
        }
    }
}

/// Constants for the game
pub mod constants {
    /// Default server address
    pub const DEFAULT_SERVER_ADDR: &str = "127.0.0.1:8080";

    /// Maximum number of concurrent games per player
    pub const MAX_CONCURRENT_GAMES: usize = 5;

    /// Game timeout in seconds
    pub const GAME_TIMEOUT_SECONDS: u64 = 300; // 5 minutes

    /// Maximum history records to keep in memory
    pub const MAX_HISTORY_RECORDS: usize = 1000;

    /// Calimero context name for the game
    pub const CALIMERO_CONTEXT_NAME: &str = "guess-number-game";
}

/// Utility functions
pub mod utils {
    use super::*;

    /// Generate a mock transaction hash for demonstration
    pub fn generate_mock_tx_hash() -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        (0..64)
            .map(|_| format!("{:x}", rng.gen_range(0..16)))
            .collect()
    }

    /// Format duration in a human-readable way
    pub fn format_duration(seconds: u64) -> String {
        if seconds < 60 {
            format!("{} 秒", seconds)
        } else if seconds < 3600 {
            format!("{} 分 {} 秒", seconds / 60, seconds % 60)
        } else {
            format!("{} 时 {} 分", seconds / 3600, (seconds % 3600) / 60)
        }
    }

    /// Validate player ID format
    pub fn is_valid_player_id(player_id: &str) -> bool {
        !player_id.is_empty()
            && player_id.len() <= 64
            && player_id
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    }

    /// Generate a random player ID
    pub fn generate_player_id() -> String {
        format!("player_{}", Uuid::new_v4().simple())
    }

    /// Calculate score based on attempts, difficulty, and time
    pub fn calculate_game_score(record: &GameRecord, _config: &GameConfig) -> u32 {
        if !record.success {
            return 0;
        }

        let base_score = 1000;
        let attempt_penalty = (record.attempts * 50).min(800);
        let time_bonus = if record.duration_seconds < 60 { 100 } else { 0 };
        let difficulty_multiplier = match record.difficulty.as_str() {
            "easy" => 1,
            "hard" => 3,
            _ => 2, // normal
        };

        ((base_score - attempt_penalty + time_bonus) * difficulty_multiplier).max(100)
    }
}

#[cfg(test)]
mod tests {
    use super::utils::*;
    use super::*;

    #[test]
    fn test_game_config_creation() {
        let config = GameConfig::for_difficulty("easy", None);
        assert_eq!(config.min_number, 1);
        assert_eq!(config.max_number, 50);
        assert_eq!(config.max_attempts, 8);

        let config = GameConfig::for_difficulty("hard", Some(15));
        assert_eq!(config.max_attempts, 15);
    }

    #[test]
    fn test_game_config_validation() {
        let config = GameConfig::default();
        assert!(config.is_valid_guess(50));
        assert!(!config.is_valid_guess(0));
        assert!(!config.is_valid_guess(101));
    }

    #[test]
    fn test_player_stats_update() {
        let mut stats = PlayerStats::new("test_player".to_string());

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

        stats.update_with_game(&record);

        assert_eq!(stats.total_games, 1);
        assert_eq!(stats.total_wins, 1);
        assert_eq!(stats.win_rate, 100.0);
        assert_eq!(stats.best_score, 3);
        assert_eq!(stats.average_attempts, 3.0);
    }

    #[test]
    fn test_guess_result() {
        assert_eq!(GuessResult::Correct.to_string(), "correct");
        assert!(GuessResult::Correct.is_game_ending());
        assert!(!GuessResult::TooSmall.is_game_ending());
    }

    #[test]
    fn test_difficulty_parsing() {
        assert_eq!("easy".parse::<Difficulty>().unwrap(), Difficulty::Easy);
        assert_eq!("NORMAL".parse::<Difficulty>().unwrap(), Difficulty::Normal);
        assert!("invalid".parse::<Difficulty>().is_err());
    }

    #[test]
    fn test_player_id_validation() {
        assert!(is_valid_player_id("player123"));
        assert!(is_valid_player_id("user_test-1"));
        assert!(!is_valid_player_id(""));
        assert!(!is_valid_player_id("player with spaces"));
        assert!(!is_valid_player_id(&"x".repeat(65))); // Too long
    }

    #[test]
    fn test_duration_formatting() {
        assert_eq!(format_duration(30), "30 秒");
        assert_eq!(format_duration(90), "1 分 30 秒");
        assert_eq!(format_duration(3660), "1 时 1 分");
    }

    #[test]
    fn test_score_calculation() {
        let config = GameConfig::default();

        // Successful game with few attempts
        let record = GameRecord::new(
            Uuid::new_v4(),
            "player".to_string(),
            42,
            2,
            vec![25, 42],
            30,
            true,
            "normal".to_string(),
        );

        let score = calculate_game_score(&record, &config);
        assert!(score > 0);

        // Unsuccessful game
        let mut failed_record = record.clone();
        failed_record.success = false;
        let failed_score = calculate_game_score(&failed_record, &config);
        assert_eq!(failed_score, 0);
    }
}
