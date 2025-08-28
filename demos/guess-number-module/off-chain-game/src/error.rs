//! Error types for the guessing number game

use thiserror::Error;
use uuid::Uuid;

/// Main error type for the guessing number game
#[derive(Error, Debug)]
pub enum GameError {
    /// Game-related errors
    #[error("Game with ID {0} not found")]
    GameNotFound(Uuid),

    #[error("Game with ID {0} is already finished")]
    GameAlreadyFinished(Uuid),

    #[error("Invalid guess: {guess} is outside the valid range {min}-{max}")]
    InvalidGuess { guess: u32, min: u32, max: u32 },

    #[error("Invalid difficulty level: {0}. Valid options are: easy, normal, hard")]
    InvalidDifficulty(String),

    #[error("Invalid player ID: {0}. Player ID must be 1-64 characters long and contain only alphanumeric characters, underscores, or hyphens")]
    InvalidPlayerId(String),

    #[error("Maximum concurrent games reached for player {0}")]
    TooManyConcurrentGames(String),

    #[error("Game timeout: game {0} has been inactive for too long")]
    GameTimeout(Uuid),

    /// Network and Calimero integration errors
    #[error("Failed to connect to Calimero Network: {0}")]
    CalimeroConnectionError(String),

    #[error("Failed to initialize Calimero context: {0}")]
    CalimeroContextError(String),

    #[error("Failed to store game result to blockchain: {0}")]
    BlockchainStorageError(String),

    #[error("Failed to retrieve data from blockchain: {0}")]
    BlockchainRetrievalError(String),

    /// Storage and persistence errors
    #[error("Local storage error: {0}")]
    StorageError(String),

    #[error("Failed to serialize game data: {0}")]
    SerializationError(String),

    #[error("Failed to deserialize game data: {0}")]
    DeserializationError(String),

    /// Configuration errors
    #[error("Invalid game configuration: {0}")]
    InvalidConfiguration(String),

    #[error("Configuration file not found at path: {0}")]
    ConfigurationNotFound(String),

    /// Network and HTTP errors
    #[error("HTTP request failed: {0}")]
    HttpError(String),

    #[error("Invalid API request: {0}")]
    InvalidApiRequest(String),

    #[error("Server internal error: {0}")]
    ServerError(String),

    /// Input/Output errors
    #[error("Input/Output error: {0}")]
    IoError(String),

    #[error("Failed to parse input: {0}")]
    ParseError(String),

    /// Authentication and authorization errors
    #[error("Authentication failed: {0}")]
    AuthenticationError(String),

    #[error("Insufficient permissions for operation: {0}")]
    PermissionError(String),

    #[error("Invalid or expired session")]
    SessionError,

    /// Rate limiting errors
    #[error("Rate limit exceeded for player {0}")]
    RateLimitExceeded(String),

    #[error("Too many failed attempts for player {0}")]
    TooManyFailedAttempts(String),

    /// Generic errors
    #[error("Unexpected error: {0}")]
    UnexpectedError(String),
}

impl GameError {
    /// Check if this error is recoverable (game can continue)
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            GameError::InvalidGuess { .. }
                | GameError::InvalidApiRequest(_)
                | GameError::ParseError(_)
                | GameError::RateLimitExceeded(_)
        )
    }

    /// Check if this error should trigger a retry
    pub fn should_retry(&self) -> bool {
        matches!(
            self,
            GameError::CalimeroConnectionError(_)
                | GameError::BlockchainStorageError(_)
                | GameError::HttpError(_)
                | GameError::StorageError(_)
        )
    }

    /// Get error category for logging/monitoring
    pub fn category(&self) -> &'static str {
        match self {
            GameError::GameNotFound(_)
            | GameError::GameAlreadyFinished(_)
            | GameError::InvalidGuess { .. }
            | GameError::GameTimeout(_)
            | GameError::TooManyConcurrentGames(_) => "game",

            GameError::InvalidDifficulty(_)
            | GameError::InvalidPlayerId(_)
            | GameError::InvalidConfiguration(_)
            | GameError::ConfigurationNotFound(_) => "config",

            GameError::CalimeroConnectionError(_)
            | GameError::CalimeroContextError(_)
            | GameError::BlockchainStorageError(_)
            | GameError::BlockchainRetrievalError(_) => "blockchain",

            GameError::StorageError(_)
            | GameError::SerializationError(_)
            | GameError::DeserializationError(_) => "storage",

            GameError::HttpError(_)
            | GameError::InvalidApiRequest(_)
            | GameError::ServerError(_) => "network",

            GameError::IoError(_) | GameError::ParseError(_) => "io",

            GameError::AuthenticationError(_)
            | GameError::PermissionError(_)
            | GameError::SessionError => "auth",

            GameError::RateLimitExceeded(_) | GameError::TooManyFailedAttempts(_) => "rate_limit",

            GameError::UnexpectedError(_) => "unknown",
        }
    }
}

/// Result type alias for game operations
pub type GameResult<T> = Result<T, GameError>;

/// Convert from standard library errors
impl From<std::io::Error> for GameError {
    fn from(err: std::io::Error) -> Self {
        GameError::IoError(err.to_string())
    }
}

impl From<serde_json::Error> for GameError {
    fn from(err: serde_json::Error) -> Self {
        GameError::SerializationError(err.to_string())
    }
}

impl From<reqwest::Error> for GameError {
    fn from(err: reqwest::Error) -> Self {
        GameError::HttpError(err.to_string())
    }
}

impl From<uuid::Error> for GameError {
    fn from(err: uuid::Error) -> Self {
        GameError::ParseError(format!("UUID parse error: {}", err))
    }
}

/// Error response for HTTP API
#[derive(serde::Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    pub category: String,
    pub recoverable: bool,
    pub timestamp: String,
}

impl From<GameError> for ErrorResponse {
    fn from(err: GameError) -> Self {
        Self {
            error: err.category().to_string(),
            message: err.to_string(),
            category: err.category().to_string(),
            recoverable: err.is_recoverable(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}

/// Helper macro for creating configuration errors
#[macro_export]
macro_rules! config_error {
    ($msg:expr) => {
        GameError::InvalidConfiguration($msg.to_string())
    };
    ($fmt:expr, $($arg:tt)*) => {
        GameError::InvalidConfiguration(format!($fmt, $($arg)*))
    };
}

/// Helper macro for creating unexpected errors
#[macro_export]
macro_rules! unexpected_error {
    ($msg:expr) => {
        GameError::UnexpectedError($msg.to_string())
    };
    ($fmt:expr, $($arg:tt)*) => {
        GameError::UnexpectedError(format!($fmt, $($arg)*))
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_categories() {
        let game_error = GameError::GameNotFound(Uuid::new_v4());
        assert_eq!(game_error.category(), "game");

        let config_error = GameError::InvalidDifficulty("invalid".to_string());
        assert_eq!(config_error.category(), "config");
    }

    #[test]
    fn test_error_recoverability() {
        let recoverable_error = GameError::InvalidGuess {
            guess: 150,
            min: 1,
            max: 100,
        };
        assert!(recoverable_error.is_recoverable());

        let non_recoverable_error = GameError::GameNotFound(Uuid::new_v4());
        assert!(!non_recoverable_error.is_recoverable());
    }

    #[test]
    fn test_error_retry_logic() {
        let should_retry = GameError::CalimeroConnectionError("timeout".to_string());
        assert!(should_retry.should_retry());

        let no_retry = GameError::InvalidDifficulty("invalid".to_string());
        assert!(!no_retry.should_retry());
    }

    #[test]
    fn test_error_response_conversion() {
        let error = GameError::InvalidGuess {
            guess: 150,
            min: 1,
            max: 100,
        };
        let response: ErrorResponse = error.into();

        assert_eq!(response.category, "game");
        assert!(response.recoverable);
        assert!(response.message.contains("150"));
    }

    #[test]
    fn test_error_macros() {
        let config_err = config_error!("Invalid setting");
        assert!(matches!(config_err, GameError::InvalidConfiguration(_)));

        let unexpected_err = unexpected_error!("Something went wrong with {}", "test");
        assert!(matches!(unexpected_err, GameError::UnexpectedError(_)));
    }
}
