use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, Json},
    routing::{get, post},
    Router,
};
use owo_colors::OwoColorize;
use rand::Rng;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
};
use tracing::{info, warn};
use uuid::Uuid;

// Re-use types from main.rs
use super::{GameConfig, GameRecord, PlayerStats};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateGameRequest {
    pub player_id: String,
    pub difficulty: Option<String>,
    pub max_attempts: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuessRequest {
    pub guess: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuessResponse {
    pub result: String,
    pub message: String,
    pub attempts: u32,
    pub max_attempts: u32,
    pub game_over: bool,
    pub success: bool,
    pub target_number: Option<u32>, // Only revealed when game is over
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameStatus {
    pub game_id: Uuid,
    pub player_id: String,
    pub attempts: u32,
    pub max_attempts: u32,
    pub difficulty: String,
    pub active: bool,
    pub guesses: Vec<u32>,
}

#[derive(Debug, Clone)]
struct ActiveGame {
    id: Uuid,
    player_id: String,
    target_number: u32,
    attempts: u32,
    guesses: Vec<u32>,
    config: GameConfig,
    start_time: SystemTime,
    difficulty: String,
    active: bool,
}

impl ActiveGame {
    fn new(player_id: String, config: GameConfig, difficulty: String) -> Self {
        let mut rng = rand::thread_rng();
        let target_number = rng.gen_range(config.min_number..=config.max_number);

        Self {
            id: Uuid::new_v4(),
            player_id,
            target_number,
            attempts: 0,
            guesses: Vec::new(),
            config,
            start_time: SystemTime::now(),
            difficulty,
            active: true,
        }
    }

    fn make_guess(&mut self, guess: u32) -> GuessResponse {
        if !self.active {
            return GuessResponse {
                result: "game_over".to_string(),
                message: "游戏已结束".to_string(),
                attempts: self.attempts,
                max_attempts: self.config.max_attempts,
                game_over: true,
                success: false,
                target_number: Some(self.target_number),
            };
        }

        self.attempts += 1;
        self.guesses.push(guess);

        match guess.cmp(&self.target_number) {
            std::cmp::Ordering::Less => {
                if self.attempts >= self.config.max_attempts {
                    self.active = false;
                    GuessResponse {
                        result: "game_over".to_string(),
                        message: format!("游戏结束！正确答案是 {}。太小了，但你已经用完了所有机会。", self.target_number),
                        attempts: self.attempts,
                        max_attempts: self.config.max_attempts,
                        game_over: true,
                        success: false,
                        target_number: Some(self.target_number),
                    }
                } else {
                    GuessResponse {
                        result: "too_small".to_string(),
                        message: format!("太小了！尝试一个更大的数字。剩余 {} 次机会。",
                            self.config.max_attempts - self.attempts),
                        attempts: self.attempts,
                        max_attempts: self.config.max_attempts,
                        game_over: false,
                        success: false,
                        target_number: None,
                    }
                }
            }
            std::cmp::Ordering::Greater => {
                if self.attempts >= self.config.max_attempts {
                    self.active = false;
                    GuessResponse {
                        result: "game_over".to_string(),
                        message: format!("游戏结束！正确答案是 {}。太大了，但你已经用完了所有机会。", self.target_number),
                        attempts: self.attempts,
                        max_attempts: self.config.max_attempts,
                        game_over: true,
                        success: false,
                        target_number: Some(self.target_number),
                    }
                } else {
                    GuessResponse {
                        result: "too_large".to_string(),
                        message: format!("太大了！尝试一个更小的数字。剩余 {} 次机会。",
                            self.config.max_attempts - self.attempts),
                        attempts: self.attempts,
                        max_attempts: self.config.max_attempts,
                        game_over: false,
                        success: false,
                        target_number: None,
                    }
                }
            }
            std::cmp::Ordering::Equal => {
                self.active = false;
                GuessResponse {
                    result: "correct".to_string(),
                    message: format!("🎉 恭喜！你用 {} 次尝试猜对了数字 {}！", self.attempts, self.target_number),
                    attempts: self.attempts,
                    max_attempts: self.config.max_attempts,
                    game_over: true,
                    success: true,
                    target_number: Some(self.target_number),
                }
            }
        }
    }

    fn to_record(&self, success: bool) -> GameRecord {
        let duration = self.start_time.elapsed().unwrap_or_default().as_secs();
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
            success,
            difficulty: self.difficulty.clone(),
        }
    }
}

#[derive(Clone)]
struct AppState {
    games: Arc<RwLock<HashMap<Uuid, ActiveGame>>>,
    game_records: Arc<RwLock<Vec<GameRecord>>>,
    player_stats: Arc<RwLock<HashMap<String, PlayerStats>>>,
}

impl AppState {
    fn new() -> Self {
        Self {
            games: Arc::new(RwLock::new(HashMap::new())),
            game_records: Arc::new(RwLock::new(Vec::new())),
            player_stats: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::init();

    println!("{}", "🚀 Starting Guess Number Game Server...".bright_green().bold());

    let state = AppState::new();

    let app = Router::new()
        // Web interface routes
        .route("/", get(serve_index))
        .route("/game", get(serve_game_page))

        // API routes
        .route("/api/game/start", post(create_game))
        .route("/api/game/:game_id/guess", post(make_guess))
        .route("/api/game/:game_id/status", get(get_game_status))
        .route("/api/game/:game_id/finish", post(finish_game))

        // Statistics routes
        .route("/api/stats/player/:player_id", get(get_player_stats))
        .route("/api/stats/leaderboard", get(get_leaderboard))
        .route("/api/history/:player_id", get(get_game_history))

        // Health check
        .route("/health", get(health_check))

        // Static files
        .nest_service("/static", ServeDir::new("static"))

        .layer(
            ServiceBuilder::new()
                .layer(CorsLayer::new()
                    .allow_origin(Any)
                    .allow_methods(Any)
                    .allow_headers(Any))
        )
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;

    println!("{}", "🌐 Server running at: http://127.0.0.1:8080".bright_blue());
    println!("{}", "📊 Game statistics: http://127.0.0.1:8080/api/stats/leaderboard".cyan());
    println!("{}", "🏥 Health check: http://127.0.0.1:8080/health".cyan());

    axum::serve(listener, app).await?;

    Ok(())
}

async fn serve_index() -> Html<String> {
    Html(include_str!("../static/index.html").to_string())
}

async fn serve_game_page() -> Html<String> {
    Html(include_str!("../static/game.html").to_string())
}

async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "guess-number-game",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "version": env!("CARGO_PKG_VERSION")
    }))
}

async fn create_game(
    State(state): State<AppState>,
    Json(request): Json<CreateGameRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let difficulty = request.difficulty.unwrap_or_else(|| "normal".to_string());
    let config = get_game_config(&difficulty, request.max_attempts);

    let game = ActiveGame::new(request.player_id.clone(), config, difficulty);
    let game_id = game.id;

    let mut games = state.games.write().await;
    games.insert(game_id, game);

    info!("Created new game {} for player {}", game_id, request.player_id);

    Ok(Json(serde_json::json!({
        "success": true,
        "game_id": game_id,
        "message": "游戏已创建",
        "range": {
            "min": games.get(&game_id).unwrap().config.min_number,
            "max": games.get(&game_id).unwrap().config.max_number
        },
        "max_attempts": games.get(&game_id).unwrap().config.max_attempts
    })))
}

async fn make_guess(
    Path(game_id): Path<Uuid>,
    State(state): State<AppState>,
    Json(request): Json<GuessRequest>,
) -> Result<Json<GuessResponse>, StatusCode> {
    let mut games = state.games.write().await;

    match games.get_mut(&game_id) {
        Some(game) => {
            let response = game.make_guess(request.guess);

            // If game is over, store the result
            if response.game_over {
                let record = game.to_record(response.success);
                store_game_result(&state, &record).await;
            }

            info!("Player {} guessed {} in game {}: {}",
                game.player_id, request.guess, game_id, response.result);

            Ok(Json(response))
        }
        None => {
            warn!("Game {} not found", game_id);
            Err(StatusCode::NOT_FOUND)
        }
    }
}

async fn get_game_status(
    Path(game_id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Json<GameStatus>, StatusCode> {
    let games = state.games.read().await;

    match games.get(&game_id) {
        Some(game) => {
            Ok(Json(GameStatus {
                game_id: game.id,
                player_id: game.player_id.clone(),
                attempts: game.attempts,
                max_attempts: game.config.max_attempts,
                difficulty: game.difficulty.clone(),
                active: game.active,
                guesses: game.guesses.clone(),
            }))
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}

async fn finish_game(
    Path(game_id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut games = state.games.write().await;

    match games.get_mut(&game_id) {
        Some(game) => {
            if game.active {
                game.active = false;
                let record = game.to_record(false); // Force finish as unsuccessful
                store_game_result(&state, &record).await;
            }

            info!("Game {} finished by player {}", game_id, game.player_id);

            Ok(Json(serde_json::json!({
                "success": true,
                "message": "游戏已结束",
                "target_number": game.target_number
            })))
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}

async fn get_player_stats(
    Path(player_id): Path<String>,
    State(state): State<AppState>,
) -> Json<PlayerStats> {
    let stats = state.player_stats.read().await;

    match stats.get(&player_id) {
        Some(player_stats) => Json(player_stats.clone()),
        None => Json(PlayerStats {
            player_id: player_id.clone(),
            total_games: 0,
            total_wins: 0,
            average_attempts: 0.0,
            best_score: 0,
            total_time: 0,
            win_rate: 0.0,
        }),
    }
}

async fn get_leaderboard(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let limit: usize = params.get("limit").and_then(|s| s.parse().ok()).unwrap_or(10);
    let stats = state.player_stats.read().await;

    let mut leaderboard: Vec<_> = stats.values().cloned().collect();
    leaderboard.sort_by(|a, b| {
        b.win_rate.partial_cmp(&a.win_rate)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.average_attempts.partial_cmp(&b.average_attempts).unwrap_or(std::cmp::Ordering::Equal))
    });

    leaderboard.truncate(limit);

    Json(serde_json::json!({
        "leaderboard": leaderboard,
        "total_players": stats.len(),
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

async fn get_game_history(
    Path(player_id): Path<String>,
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Json<Vec<GameRecord>> {
    let limit: usize = params.get("limit").and_then(|s| s.parse().ok()).unwrap_or(20);
    let records = state.game_records.read().await;

    let mut player_records: Vec<_> = records
        .iter()
        .filter(|record| record.player_id == player_id)
        .cloned()
        .collect();

    player_records.sort_by_key(|record| std::cmp::Reverse(record.timestamp));
    player_records.truncate(limit);

    Json(player_records)
}

async fn store_game_result(state: &AppState, record: &GameRecord) {
    // Store game record
    let mut records = state.game_records.write().await;
    records.push(record.clone());

    // Update player statistics
    let mut stats = state.player_stats.write().await;
    let player_stats = stats.entry(record.player_id.clone()).or_insert(PlayerStats {
        player_id: record.player_id.clone(),
        total_games: 0,
        total_wins: 0,
        average_attempts: 0.0,
        best_score: u32::MAX,
        total_time: 0,
        win_rate: 0.0,
    });

    player_stats.total_games += 1;
    player_stats.total_time += record.duration_seconds;

    if record.success {
        player_stats.total_wins += 1;
        if player_stats.best_score == u32::MAX || record.attempts < player_stats.best_score {
            player_stats.best_score = record.attempts;
        }
    }

    player_stats.win_rate = (player_stats.total_wins as f64 / player_stats.total_games as f64) * 100.0;

    // Calculate average attempts from recent games
    let player_records: Vec<_> = records
        .iter()
        .filter(|r| r.player_id == record.player_id)
        .collect();

    if !player_records.is_empty() {
        let total_attempts: u32 = player_records.iter().map(|r| r.attempts).sum();
        player_stats.average_attempts = total_attempts as f64 / player_records.len() as f64;
    }

    info!("Stored game result for player {}: {} attempts, success: {}",
        record.player_id, record.attempts, record.success);

    // TODO: Store to Calimero/NEAR blockchain
    tokio::spawn(async move {
        // Simulate blockchain storage delay
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        info!("Game result stored to blockchain: {}", record.game_id);
    });
}

fn get_game_config(difficulty: &str, max_attempts: Option<u32>) -> GameConfig {
    match difficulty {
        "easy" => GameConfig {
            min_number: 1,
            max_number: 50,
            max_attempts: max_attempts.unwrap_or(8),
        },
        "hard" => GameConfig {
            min_number: 1,
            max_number: 200,
            max_attempts: max_attempts.unwrap_or(12),
        },
        _ => GameConfig {
            min_number: 1,
            max_number: 100,
            max_attempts: max_attempts.unwrap_or(10),
        },
    }
}
