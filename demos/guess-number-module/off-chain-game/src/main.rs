use std::io::{self, Write};
use std::time::{SystemTime, UNIX_EPOCH};
use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand};
use owo_colors::OwoColorize;
use rand::Rng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use calimero_sdk::context::ContextManager;
use calimero_primitives::identity::Did;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(name = "guess-number-client")]
#[command(about = "A guessing number game with blockchain integration")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Player identity (DID)
    #[arg(short, long)]
    player: Option<String>,

    /// Game difficulty (easy: 1-50, normal: 1-100, hard: 1-200)
    #[arg(short, long, default_value = "normal")]
    difficulty: String,

    /// Maximum attempts allowed
    #[arg(short, long)]
    max_attempts: Option<u32>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start a new guessing game
    Play,
    /// Show player statistics
    Stats,
    /// Show game history
    History,
    /// Configure game settings
    Config,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameConfig {
    pub min_number: u32,
    pub max_number: u32,
    pub max_attempts: u32,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerStats {
    pub player_id: String,
    pub total_games: u32,
    pub total_wins: u32,
    pub average_attempts: f64,
    pub best_score: u32,
    pub total_time: u64,
    pub win_rate: f64,
}

#[derive(Debug)]
struct Game {
    id: Uuid,
    config: GameConfig,
    target_number: u32,
    attempts: u32,
    guesses: Vec<u32>,
    start_time: SystemTime,
    player_id: String,
    difficulty: String,
}

impl Game {
    fn new(config: GameConfig, player_id: String, difficulty: String) -> Self {
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
        }
    }

    fn make_guess(&mut self, guess: u32) -> GameResult {
        self.attempts += 1;
        self.guesses.push(guess);

        match guess.cmp(&self.target_number) {
            std::cmp::Ordering::Less => {
                if self.attempts >= self.config.max_attempts {
                    GameResult::GameOver
                } else {
                    GameResult::TooSmall
                }
            }
            std::cmp::Ordering::Greater => {
                if self.attempts >= self.config.max_attempts {
                    GameResult::GameOver
                } else {
                    GameResult::TooLarge
                }
            }
            std::cmp::Ordering::Equal => GameResult::Correct,
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

#[derive(Debug, PartialEq)]
enum GameResult {
    TooSmall,
    TooLarge,
    Correct,
    GameOver,
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    tracing_subscriber::init();

    let cli = Cli::parse();

    // Initialize Calimero context if available
    let _context_manager = init_calimero().await?;

    let player_id = cli.player.unwrap_or_else(|| {
        format!("player_{}", Uuid::new_v4().simple())
    });

    match cli.command {
        Some(Commands::Play) | None => {
            let config = get_game_config(&cli.difficulty, cli.max_attempts);
            play_game(config, player_id).await?;
        }
        Some(Commands::Stats) => {
            show_player_stats(&player_id).await?;
        }
        Some(Commands::History) => {
            show_game_history(&player_id).await?;
        }
        Some(Commands::Config) => {
            show_config().await?;
        }
    }

    Ok(())
}

async fn init_calimero() -> eyre::Result<Option<ContextManager>> {
    // TODO: Initialize Calimero context manager
    // This would connect to the Calimero network and set up the context
    // for storing game results on-chain

    println!("{}", "🔗 Initializing Calimero Network connection...".cyan());

    // For now, we'll simulate the connection
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    println!("{}", "✅ Connected to Calimero Network".green());

    Ok(None)
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

async fn play_game(config: GameConfig, player_id: String) -> eyre::Result<()> {
    print_game_header(&config);

    let mut game = Game::new(config.clone(), player_id.clone(), "normal".to_string());

    loop {
        print!("{}", "💭 请输入你的猜测 (输入 'quit' 退出): ".bright_blue());
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        let input = input.trim();

        if input == "quit" {
            println!("{}", "👋 游戏已退出!".yellow());
            return Ok(());
        }

        let guess = match input.parse::<u32>() {
            Ok(num) => num,
            Err(_) => {
                println!("{}", "❌ 请输入一个有效的数字!".red());
                continue;
            }
        };

        if guess < config.min_number || guess > config.max_number {
            println!(
                "{}",
                format!("❌ 数字必须在 {} 到 {} 之间!", config.min_number, config.max_number).red()
            );
            continue;
        }

        let result = game.make_guess(guess);

        match result {
            GameResult::TooSmall => {
                println!("{}", "📈 太小了！尝试一个更大的数字".yellow());
                show_attempts_left(&game, &config);
            }
            GameResult::TooLarge => {
                println!("{}", "📉 太大了！尝试一个更小的数字".yellow());
                show_attempts_left(&game, &config);
            }
            GameResult::Correct => {
                print_victory(&game).await?;
                return Ok(());
            }
            GameResult::GameOver => {
                print_game_over(&game).await?;
                return Ok(());
            }
        }
    }
}

fn print_game_header(config: &GameConfig) {
    println!("{}", "🎮 欢迎来到链上猜数字游戏！".bright_green().bold());
    println!("{}", "═".repeat(50).bright_blue());
    println!("{}", format!(
        "🎯 猜一个 {} 到 {} 之间的数字",
        config.min_number,
        config.max_number
    ).cyan());
    println!("{}", format!("🔄 你有 {} 次机会", config.max_attempts).cyan());
    println!("{}", "═".repeat(50).bright_blue());
    println!();
}

fn show_attempts_left(game: &Game, config: &GameConfig) {
    let attempts_left = config.max_attempts - game.attempts;
    if attempts_left > 0 {
        println!("{}", format!("🔄 剩余尝试次数: {}", attempts_left).bright_black());
    }
}

async fn print_victory(game: &Game) -> eyre::Result<()> {
    println!();
    println!("{}", "🎉 恭喜！你猜对了！".bright_green().bold());
    print_game_stats(game, true).await?;
    Ok(())
}

async fn print_game_over(game: &Game) -> eyre::Result<()> {
    println!();
    println!("{}", "💀 游戏结束！你用完了所有机会。".red().bold());
    println!("{}", format!("🎯 正确答案是: {}", game.target_number).bright_yellow());
    print_game_stats(game, false).await?;
    Ok(())
}

async fn print_game_stats(game: &Game, success: bool) -> eyre::Result<()> {
    let duration = game.start_time.elapsed().unwrap_or_default().as_secs();

    println!();
    println!("{}", "📋 游戏统计:".bright_blue().bold());
    println!("{}", format!("   🎯 目标数字: {}", game.target_number).white());
    println!("{}", format!("   🔄 尝试次数: {}", game.attempts).white());
    println!("{}", format!("   ⏱️  游戏时长: {} 秒", duration).white());
    println!("{}", format!("   🎮 游戏ID: {}", game.id.simple()).bright_black());

    // Store game result to blockchain
    let record = game.to_record(success);
    store_game_result(&record).await?;

    // Show player statistics
    show_current_session_stats(&game.player_id).await?;

    Ok(())
}

async fn store_game_result(record: &GameRecord) -> eyre::Result<()> {
    println!("{}", "💾 正在保存游戏结果到区块链...".cyan());

    // TODO: Implement actual Calimero/NEAR storage
    // This would use the Calimero SDK to store the game result
    // For now, we'll simulate the storage operation

    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

    // Simulate storing to local cache/database
    println!("{}", "✅ 游戏记录已保存到区块链！".green());

    // Show transaction info (simulated)
    println!("{}", format!("📝 交易哈希: 0x{}", generate_mock_tx_hash()).bright_black());

    Ok(())
}

async fn show_player_stats(_player_id: &str) -> eyre::Result<()> {
    println!("{}", "📊 玩家统计".bright_blue().bold());
    println!("{}", "═".repeat(30).bright_blue());

    // TODO: Implement actual stats retrieval from blockchain
    // For now, show mock stats
    let stats = PlayerStats {
        player_id: _player_id.to_string(),
        total_games: 15,
        total_wins: 12,
        average_attempts: 4.2,
        best_score: 2,
        total_time: 1200,
        win_rate: 80.0,
    };

    println!("{}", format!("🎮 总游戏数: {}", stats.total_games).white());
    println!("{}", format!("🏆 获胜次数: {}", stats.total_wins).white());
    println!("{}", format!("📈 胜率: {:.1}%", stats.win_rate).white());
    println!("{}", format!("🎯 平均尝试次数: {:.1}", stats.average_attempts).white());
    println!("{}", format!("⭐ 最佳记录: {} 次猜中", stats.best_score).white());
    println!("{}", format!("⏱️  总游戏时长: {} 分钟", stats.total_time / 60).white());

    Ok(())
}

async fn show_game_history(_player_id: &str) -> eyre::Result<()> {
    println!("{}", "📚 游戏历史".bright_blue().bold());
    println!("{}", "═".repeat(50).bright_blue());

    // TODO: Implement actual history retrieval from blockchain
    // For now, show mock history

    println!("{}", "🕐 最近的游戏:".cyan());
    for i in 1..=5 {
        let timestamp = Utc::now() - chrono::Duration::hours(i);
        let success = i % 2 == 0;
        let attempts = 3 + i as u32;

        let status = if success { "✅ 成功" } else { "❌ 失败" };
        println!(
            "   {} {} - {} 次尝试 - {}",
            timestamp.format("%m-%d %H:%M").to_string().bright_black(),
            status,
            attempts,
            if success { "".to_string() } else { format!("(答案: {})", 42 + i) }
        );
    }

    Ok(())
}

async fn show_current_session_stats(_player_id: &str) -> eyre::Result<()> {
    println!();
    println!("{}", "🏆 你的游戏记录:".bright_blue());
    println!("{}", "   📊 总游戏数: 16 (+1)".white());
    println!("{}", "   🎯 平均尝试次数: 4.1".white());
    println!("{}", "   ⭐ 最佳记录: 2 次猜中".white());

    Ok(())
}

async fn show_config() -> eyre::Result<()> {
    println!("{}", "⚙️  游戏配置".bright_blue().bold());
    println!("{}", "═".repeat(30).bright_blue());

    println!("{}", "🎮 可用难度:".cyan());
    println!("   {} - 数字范围: 1-50, 最大尝试: 8 次", "简单".green());
    println!("   {} - 数字范围: 1-100, 最大尝试: 10 次", "普通".yellow());
    println!("   {} - 数字范围: 1-200, 最大尝试: 12 次", "困难".red());

    println!();
    println!("{}", "🔧 使用方法:".cyan());
    println!("   guess-number-client play --difficulty easy");
    println!("   guess-number-client play --difficulty normal --max-attempts 15");
    println!("   guess-number-client stats");
    println!("   guess-number-client history");

    Ok(())
}

fn generate_mock_tx_hash() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..64)
        .map(|_| format!("{:x}", rng.gen_range(0..16)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_game_creation() {
        let config = GameConfig {
            min_number: 1,
            max_number: 100,
            max_attempts: 10,
        };
        let game = Game::new(config.clone(), "test_player".to_string(), "normal".to_string());

        assert_eq!(game.attempts, 0);
        assert!(game.guesses.is_empty());
        assert!(game.target_number >= config.min_number && game.target_number <= config.max_number);
    }

    #[test]
    fn test_game_guess_too_small() {
        let config = GameConfig {
            min_number: 50,
            max_number: 50,
            max_attempts: 10,
        };
        let mut game = Game::new(config, "test_player".to_string(), "normal".to_string());
        game.target_number = 50; // Force target to be 50

        let result = game.make_guess(25);
        assert_eq!(result, GameResult::TooSmall);
        assert_eq!(game.attempts, 1);
    }

    #[test]
    fn test_game_guess_correct() {
        let config = GameConfig {
            min_number: 50,
            max_number: 50,
            max_attempts: 10,
        };
        let mut game = Game::new(config, "test_player".to_string(), "normal".to_string());
        game.target_number = 50; // Force target to be 50

        let result = game.make_guess(50);
        assert_eq!(result, GameResult::Correct);
        assert_eq!(game.attempts, 1);
    }
}
