//! Core game logic for the guessing number game
//!
//! This module contains the main game engine that handles:
//! - Game state management
//! - Guess validation and processing
//! - Game lifecycle (start, play, finish)
//! - Integration with Calimero for result storage

use rand::Rng;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use crate::error::{GameError, GameResult};
use crate::{GameConfig, GameRecord, GuessResult};

/// The main game state structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    /// Unique identifier for this game instance
    pub id: Uuid,
    /// Configuration for this game (difficulty, ranges, etc.)
    pub config: GameConfig,
    /// The target number to guess
    target_number: u32,
    /// Number of attempts made so far
    pub attempts: u32,
    /// List of all guesses made by the player
    pub guesses: Vec<u32>,
    /// When the game started
    start_time: SystemTime,
    /// Player identifier
    pub player_id: String,
    /// Game difficulty level
    pub difficulty: String,
    /// Whether the game is still active
    pub active: bool,
    /// Whether the game was won
    pub won: bool,
}

impl Game {
    /// Create a new game instance
    pub fn new(config: GameConfig, player_id: String, difficulty: String) -> Self {
        let mut rng = rand::thread_rng();
        let target_number = rng.gen_range(config.min_number..=config.max_number);

        Self {
            id: Uuid::new_v4(),
            config,
            target_number,
            attempts: 0,
            guesses: Vec::new(),
            start_time: SystemTime::now(),
            player_id,
            difficulty,
            active: true,
            won: false,
        }
    }

    /// Make a guess and return the result
    pub fn make_guess(&mut self, guess: u32) -> GameResult<GuessResult> {
        if !self.active {
            return Err(GameError::GameAlreadyFinished(self.id));
        }

        // Validate guess is within range
        if !self.config.is_valid_guess(guess) {
            return Err(GameError::InvalidGuess {
                guess,
                min: self.config.min_number,
                max: self.config.max_number,
            });
        }

        // Check for duplicate guess (optional enhancement)
        if self.guesses.contains(&guess) {
            // Allow duplicate guesses but maybe warn the user
            // This is not an error, just potentially suboptimal strategy
        }

        self.attempts += 1;
        self.guesses.push(guess);

        let result = match guess.cmp(&self.target_number) {
            Ordering::Less => {
                if self.attempts >= self.config.max_attempts {
                    self.active = false;
                    GuessResult::GameOver
                } else {
                    GuessResult::TooSmall
                }
            }
            Ordering::Greater => {
                if self.attempts >= self.config.max_attempts {
                    self.active = false;
                    GuessResult::GameOver
                } else {
                    GuessResult::TooLarge
                }
            }
            Ordering::Equal => {
                self.active = false;
                self.won = true;
                GuessResult::Correct
            }
        };

        Ok(result)
    }

    /// Get the game's current status
    pub fn status(&self) -> GameStatus {
        GameStatus {
            game_id: self.id,
            player_id: self.player_id.clone(),
            attempts: self.attempts,
            max_attempts: self.config.max_attempts,
            difficulty: self.difficulty.clone(),
            active: self.active,
            won: self.won,
            guesses: self.guesses.clone(),
            remaining_attempts: self.remaining_attempts(),
            duration_seconds: self.duration_seconds(),
        }
    }

    /// Get remaining attempts
    pub fn remaining_attempts(&self) -> u32 {
        if self.attempts >= self.config.max_attempts {
            0
        } else {
            self.config.max_attempts - self.attempts
        }
    }

    /// Get game duration in seconds
    pub fn duration_seconds(&self) -> u64 {
        self.start_time
            .elapsed()
            .unwrap_or(Duration::from_secs(0))
            .as_secs()
    }

    /// Check if game has timed out
    pub fn is_timed_out(&self, timeout_duration: Duration) -> bool {
        self.start_time.elapsed().unwrap_or(Duration::from_secs(0)) > timeout_duration
    }

    /// Force finish the game (for timeouts or manual termination)
    pub fn force_finish(&mut self) {
        self.active = false;
    }

    /// Reveal the target number (only when game is over)
    pub fn reveal_target(&self) -> Option<u32> {
        if !self.active {
            Some(self.target_number)
        } else {
            None
        }
    }

    /// Convert game to a permanent record for storage
    pub fn to_record(&self) -> GameRecord {
        let duration = self.duration_seconds();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        GameRecord {
            game_id: self.id,
            player_id: self.player_id.clone(),
            target_number: self.target_number,
            attempts: self.attempts,
            guesses: self.guesses.clone(),
            duration_seconds: duration,
            timestamp,
            success: self.won,
            difficulty: self.difficulty.clone(),
        }
    }

    /// Get game statistics for analysis
    pub fn get_stats(&self) -> GameStats {
        let duration = self.duration_seconds();
        let efficiency = if self.attempts > 0 {
            self.config.max_attempts as f64 / self.attempts as f64
        } else {
            0.0
        };

        GameStats {
            attempts: self.attempts,
            duration_seconds: duration,
            efficiency_ratio: efficiency,
            guess_pattern: self.analyze_guess_pattern(),
            time_per_attempt: if self.attempts > 0 {
                duration as f64 / self.attempts as f64
            } else {
                0.0
            },
        }
    }

    /// Analyze the pattern of guesses to provide insights
    fn analyze_guess_pattern(&self) -> GuessPattern {
        if self.guesses.len() < 2 {
            return GuessPattern::Insufficient;
        }

        let mut increasing = 0;
        let mut decreasing = 0;
        let mut random_jumps = 0;

        for window in self.guesses.windows(2) {
            match window[1].cmp(&window[0]) {
                Ordering::Greater => increasing += 1,
                Ordering::Less => decreasing += 1,
                Ordering::Equal => {} // Same guess, ignore
            }

            // Check for large jumps (more than 20% of range)
            let jump = (window[1] as i32 - window[0] as i32).abs();
            let range_size = self.config.max_number - self.config.min_number;
            if jump > (range_size as f64 * 0.2) as i32 {
                random_jumps += 1;
            }
        }

        if increasing > decreasing + random_jumps {
            GuessPattern::Methodical
        } else if random_jumps > increasing + decreasing {
            GuessPattern::Random
        } else {
            GuessPattern::Mixed
        }
    }
}

/// Current status of a game
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameStatus {
    pub game_id: Uuid,
    pub player_id: String,
    pub attempts: u32,
    pub max_attempts: u32,
    pub difficulty: String,
    pub active: bool,
    pub won: bool,
    pub guesses: Vec<u32>,
    pub remaining_attempts: u32,
    pub duration_seconds: u64,
}

/// Game statistics for analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameStats {
    pub attempts: u32,
    pub duration_seconds: u64,
    pub efficiency_ratio: f64, // How efficient the player was (max_attempts / actual_attempts)
    pub guess_pattern: GuessPattern,
    pub time_per_attempt: f64, // Average time spent per attempt
}

/// Pattern analysis of player's guessing strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GuessPattern {
    /// Methodical approach (generally moving in one direction)
    Methodical,
    /// Random approach (large jumps between guesses)
    Random,
    /// Mixed approach
    Mixed,
    /// Not enough data to analyze
    Insufficient,
}

/// Game manager to handle multiple games
#[derive(Debug, Clone)]
pub struct GameManager {
    /// Configuration defaults
    pub default_config: GameConfig,
    /// Timeout for inactive games
    pub game_timeout: Duration,
}

impl GameManager {
    /// Create a new game manager
    pub fn new() -> Self {
        Self {
            default_config: GameConfig::default(),
            game_timeout: Duration::from_secs(300), // 5 minutes
        }
    }

    /// Create a new game with specified parameters
    pub fn create_game(
        &self,
        player_id: String,
        difficulty: Option<String>,
        max_attempts: Option<u32>,
    ) -> GameResult<Game> {
        // Validate player ID
        if !crate::utils::is_valid_player_id(&player_id) {
            return Err(GameError::InvalidPlayerId(player_id));
        }

        let difficulty = difficulty.unwrap_or_else(|| "normal".to_string());
        let config = GameConfig::for_difficulty(&difficulty, max_attempts);

        Ok(Game::new(config, player_id, difficulty))
    }

    /// Validate a game configuration
    pub fn validate_config(&self, config: &GameConfig) -> GameResult<()> {
        if config.min_number >= config.max_number {
            return Err(GameError::InvalidConfiguration(
                "Minimum number must be less than maximum number".to_string(),
            ));
        }

        if config.max_attempts == 0 {
            return Err(GameError::InvalidConfiguration(
                "Maximum attempts must be greater than 0".to_string(),
            ));
        }

        if config.max_attempts > 50 {
            return Err(GameError::InvalidConfiguration(
                "Maximum attempts cannot exceed 50".to_string(),
            ));
        }

        let range_size = config.max_number - config.min_number + 1;
        if range_size < 2 {
            return Err(GameError::InvalidConfiguration(
                "Number range must contain at least 2 numbers".to_string(),
            ));
        }

        Ok(())
    }

    /// Clean up timed out games
    pub fn cleanup_timed_out_games(&self, games: &mut std::collections::HashMap<Uuid, Game>) {
        let timeout_threshold = SystemTime::now() - self.game_timeout;

        games.retain(|_, game| {
            if game.start_time < timeout_threshold && game.active {
                false // Remove timed out active games
            } else {
                true // Keep the game
            }
        });
    }

    /// Get recommended strategy hint based on guess history
    pub fn get_strategy_hint(&self, game: &Game) -> Option<String> {
        if game.guesses.len() < 2 {
            return Some("💡 提示：使用二分查找策略可能更高效！".to_string());
        }

        match game.analyze_guess_pattern() {
            GuessPattern::Random => {
                Some("💡 建议：尝试更系统性的方法，比如总是猜测当前范围的中点。".to_string())
            }
            GuessPattern::Methodical => Some("👍 很好！你在使用系统性的方法。".to_string()),
            GuessPattern::Mixed => Some("💡 提示：保持一致的策略可能会更有效。".to_string()),
            GuessPattern::Insufficient => None,
        }
    }
}

impl Default for GameManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Game event for logging and analytics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameEvent {
    pub game_id: Uuid,
    pub player_id: String,
    pub event_type: GameEventType,
    pub timestamp: u64,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameEventType {
    GameStarted,
    GuessMade,
    GameWon,
    GameLost,
    GameAbandoned,
    GameTimeout,
}

impl GameEvent {
    pub fn new(game_id: Uuid, player_id: String, event_type: GameEventType) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            game_id,
            player_id,
            event_type,
            timestamp,
            data: serde_json::json!({}),
        }
    }

    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = data;
        self
    }
}

/// Helper functions for game logic
impl Game {
    /// Get the optimal number of attempts for this game configuration
    pub fn optimal_attempts(&self) -> u32 {
        let range_size = self.config.max_number - self.config.min_number + 1;
        (range_size as f64).log2().ceil() as u32
    }

    /// Calculate the current search range based on previous guesses
    pub fn current_search_range(&self) -> (u32, u32) {
        if self.guesses.is_empty() {
            return (self.config.min_number, self.config.max_number);
        }

        let mut min = self.config.min_number;
        let mut max = self.config.max_number;

        // Analyze guesses to narrow down the range
        for &guess in &self.guesses {
            match guess.cmp(&self.target_number) {
                Ordering::Less => {
                    if guess >= min {
                        min = guess + 1;
                    }
                }
                Ordering::Greater => {
                    if guess <= max {
                        max = guess - 1;
                    }
                }
                Ordering::Equal => {
                    return (guess, guess);
                }
            }
        }

        (min, max)
    }

    /// Get the efficiency rating of the current game
    pub fn efficiency_rating(&self) -> f64 {
        if self.attempts == 0 {
            return 1.0;
        }

        let optimal = self.optimal_attempts() as f64;
        let actual = self.attempts as f64;

        (optimal / actual).min(1.0)
    }

    /// Check if the player is using a good strategy
    pub fn strategy_assessment(&self) -> StrategyAssessment {
        if self.attempts < 2 {
            return StrategyAssessment::TooEarlyToTell;
        }

        let efficiency = self.efficiency_rating();
        let pattern = self.analyze_guess_pattern();

        match (efficiency > 0.8, pattern) {
            (true, GuessPattern::Methodical) => StrategyAssessment::Excellent,
            (true, _) => StrategyAssessment::Good,
            (false, GuessPattern::Methodical) => StrategyAssessment::Fair,
            (false, GuessPattern::Random) => StrategyAssessment::Poor,
            (false, _) => StrategyAssessment::Fair,
        }
    }

    /// Get next optimal guess suggestion (for hint system)
    pub fn suggest_next_guess(&self) -> Option<u32> {
        if !self.active {
            return None;
        }

        let (min, max) = self.current_search_range();
        if min <= max {
            Some((min + max) / 2) // Binary search strategy
        } else {
            None
        }
    }

    /// Validate that the game state is consistent
    pub fn validate_state(&self) -> GameResult<()> {
        // Check basic invariants
        if self.attempts != self.guesses.len() as u32 {
            return Err(GameError::UnexpectedError(
                "Attempts count doesn't match guesses length".to_string(),
            ));
        }

        if self.attempts > self.config.max_attempts {
            return Err(GameError::UnexpectedError(
                "Attempts exceed maximum allowed".to_string(),
            ));
        }

        if !self.config.is_valid_guess(self.target_number) {
            return Err(GameError::UnexpectedError(
                "Target number is outside valid range".to_string(),
            ));
        }

        // Validate all guesses are within range
        for &guess in &self.guesses {
            if !self.config.is_valid_guess(guess) {
                return Err(GameError::UnexpectedError(format!(
                    "Invalid guess {} found in history",
                    guess
                )));
            }
        }

        Ok(())
    }

    /// Calculate score for this game
    pub fn calculate_score(&self) -> u32 {
        if !self.won {
            return 0;
        }

        let base_score = 1000;
        let optimal_attempts = self.optimal_attempts();
        let efficiency_bonus = ((optimal_attempts as f64 / self.attempts as f64) * 500.0) as u32;

        // Time bonus (faster is better)
        let time_bonus = if self.duration_seconds() < 60 {
            200
        } else if self.duration_seconds() < 180 {
            100
        } else {
            0
        };

        // Difficulty multiplier
        let difficulty_multiplier = match self.difficulty.as_str() {
            "easy" => 1,
            "hard" => 3,
            _ => 2, // normal
        };

        (base_score + efficiency_bonus + time_bonus) * difficulty_multiplier
    }
}

/// Strategy assessment levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StrategyAssessment {
    Excellent,
    Good,
    Fair,
    Poor,
    TooEarlyToTell,
}

impl std::fmt::Display for StrategyAssessment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StrategyAssessment::Excellent => write!(f, "优秀"),
            StrategyAssessment::Good => write!(f, "良好"),
            StrategyAssessment::Fair => write!(f, "一般"),
            StrategyAssessment::Poor => write!(f, "需要改进"),
            StrategyAssessment::TooEarlyToTell => write!(f, "数据不足"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_game_creation() {
        let config = GameConfig::default();
        let game = Game::new(
            config.clone(),
            "test_player".to_string(),
            "normal".to_string(),
        );

        assert_eq!(game.attempts, 0);
        assert!(game.guesses.is_empty());
        assert!(game.active);
        assert!(!game.won);
        assert!(game.target_number >= config.min_number && game.target_number <= config.max_number);
    }

    #[test]
    fn test_valid_guess() {
        let config = GameConfig::default();
        let mut game = Game::new(config, "test_player".to_string(), "normal".to_string());

        // Force a known target for testing
        game.target_number = 50;

        let result = game.make_guess(25).unwrap();
        assert_eq!(result, GuessResult::TooSmall);
        assert_eq!(game.attempts, 1);
        assert_eq!(game.guesses.len(), 1);
        assert!(game.active);
    }

    #[test]
    fn test_invalid_guess() {
        let config = GameConfig::default();
        let mut game = Game::new(config, "test_player".to_string(), "normal".to_string());

        let result = game.make_guess(150);
        assert!(matches!(result, Err(GameError::InvalidGuess { .. })));
        assert_eq!(game.attempts, 0); // Should not increment on invalid guess
    }

    #[test]
    fn test_winning_guess() {
        let config = GameConfig::default();
        let mut game = Game::new(config, "test_player".to_string(), "normal".to_string());

        let target = game.target_number;
        let result = game.make_guess(target).unwrap();

        assert_eq!(result, GuessResult::Correct);
        assert!(!game.active);
        assert!(game.won);
    }

    #[test]
    fn test_max_attempts_exceeded() {
        let config = GameConfig {
            min_number: 1,
            max_number: 100,
            max_attempts: 2,
        };
        let mut game = Game::new(config, "test_player".to_string(), "normal".to_string());

        // Force target to something we won't guess
        game.target_number = 50;

        // First guess
        let result1 = game.make_guess(25).unwrap();
        assert_eq!(result1, GuessResult::TooSmall);
        assert!(game.active);

        // Second guess (should end game)
        let result2 = game.make_guess(75).unwrap();
        assert_eq!(result2, GuessResult::GameOver);
        assert!(!game.active);
        assert!(!game.won);
    }

    #[test]
    fn test_game_after_finished() {
        let config = GameConfig::default();
        let mut game = Game::new(config, "test_player".to_string(), "normal".to_string());

        game.force_finish();
        let result = game.make_guess(50);

        assert!(matches!(result, Err(GameError::GameAlreadyFinished(_))));
    }

    #[test]
    fn test_search_range_calculation() {
        let config = GameConfig {
            min_number: 1,
            max_number: 100,
            max_attempts: 10,
        };
        let mut game = Game::new(config, "test_player".to_string(), "normal".to_string());

        // Force target for predictable testing
        game.target_number = 75;

        // Initial range
        assert_eq!(game.current_search_range(), (1, 100));

        // After guessing too low
        game.make_guess(50).unwrap();
        assert_eq!(game.current_search_range(), (51, 100));

        // After guessing too high
        game.make_guess(90).unwrap();
        assert_eq!(game.current_search_range(), (51, 89));
    }

    #[test]
    fn test_optimal_attempts_calculation() {
        let config = GameConfig {
            min_number: 1,
            max_number: 100,
            max_attempts: 10,
        };
        let game = Game::new(config, "test_player".to_string(), "normal".to_string());

        // For range 1-100, optimal is ceil(log2(100)) = 7
        assert_eq!(game.optimal_attempts(), 7);
    }

    #[test]
    fn test_efficiency_rating() {
        let config = GameConfig::default();
        let mut game = Game::new(config, "test_player".to_string(), "normal".to_string());

        // Perfect efficiency (no guesses yet)
        assert_eq!(game.efficiency_rating(), 1.0);

        // Add some attempts
        game.attempts = 7; // Equal to optimal
        assert_eq!(game.efficiency_rating(), 1.0);

        game.attempts = 14; // Twice the optimal
        assert_eq!(game.efficiency_rating(), 0.5);
    }

    #[test]
    fn test_game_manager_validation() {
        let manager = GameManager::new();

        // Valid config
        let valid_config = GameConfig::default();
        assert!(manager.validate_config(&valid_config).is_ok());

        // Invalid config (min >= max)
        let invalid_config = GameConfig {
            min_number: 50,
            max_number: 50,
            max_attempts: 10,
        };
        assert!(manager.validate_config(&invalid_config).is_err());
    }

    #[test]
    fn test_game_record_conversion() {
        let config = GameConfig::default();
        let mut game = Game::new(config, "test_player".to_string(), "normal".to_string());

        game.attempts = 5;
        game.guesses = vec![25, 75, 50, 60, 55];
        game.won = true;

        let record = game.to_record();

        assert_eq!(record.game_id, game.id);
        assert_eq!(record.player_id, game.player_id);
        assert_eq!(record.attempts, 5);
        assert_eq!(record.guesses.len(), 5);
        assert!(record.success);
    }
}
