use eyre::Result;
use futures_util::StreamExt;
use reqwest::Client;
use reqwest_eventsource::{Event, EventSource};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

// =============================================================================
// Types matching the server
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum NodeEvent {
    Context {
        context_id: String,
        event_type: String,
        data: serde_json::Value,
        timestamp: u64,
    },
    System {
        message: String,
        level: String,
        timestamp: u64,
    },
    Heartbeat { timestamp: u64 },
}

// =============================================================================
// SSE Client Implementation
// =============================================================================

pub struct SseClient {
    base_url: String,
    client: Client,
}

impl SseClient {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            client: Client::new(),
        }
    }

    /// Connect to SSE endpoint and start listening for events
    pub async fn subscribe(&self, contexts: Option<Vec<String>>) -> Result<()> {
        let mut url = format!("{}/events", self.base_url);
        
        // Add context subscription parameters
        if let Some(contexts) = contexts {
            let contexts_param = contexts.join(",");
            url = format!("{}?contexts={}", url, contexts_param);
        }

        info!("Connecting to SSE endpoint: {}", url);

        // Create EventSource from the URL
        let mut event_source = EventSource::new(
            self.client
                .get(&url)
                .timeout(Duration::from_secs(30))
        )?;

        info!("Connected to SSE stream, listening for events...");
        info!("Press Ctrl+C to stop");

        // Listen for events
        while let Some(event) = event_source.next().await {
            match event {
                Ok(Event::Open) => {
                    info!("🔗 SSE connection opened");
                }
                Ok(Event::Message(message)) => {
                    match message.event.as_str() {
                        "context" => {
                            match serde_json::from_str::<NodeEvent>(&message.data) {
                                Ok(NodeEvent::Context { context_id, event_type, data, timestamp }) => {
                                    info!(
                                        "📦 Context Event [{}] type={} timestamp={}", 
                                        context_id, event_type, timestamp
                                    );
                                    debug!("   Data: {}", serde_json::to_string_pretty(&data)?);
                                }
                                Ok(event) => {
                                    warn!("Unexpected event type in context message: {:?}", event);
                                }
                                Err(e) => {
                                    error!("Failed to parse context event: {} | Raw: {}", e, message.data);
                                }
                            }
                        }
                        "system" => {
                            match serde_json::from_str::<NodeEvent>(&message.data) {
                                Ok(NodeEvent::System { message: msg, level, timestamp }) => {
                                    info!("🖥️  System Event [{}] {}: {}", timestamp, level, msg);
                                }
                                Ok(event) => {
                                    warn!("Unexpected event type in system message: {:?}", event);
                                }
                                Err(e) => {
                                    error!("Failed to parse system event: {} | Raw: {}", e, message.data);
                                }
                            }
                        }
                        "heartbeat" => {
                            match serde_json::from_str::<NodeEvent>(&message.data) {
                                Ok(NodeEvent::Heartbeat { timestamp }) => {
                                    debug!("💓 Heartbeat at {}", timestamp);
                                }
                                Ok(event) => {
                                    warn!("Unexpected event type in heartbeat message: {:?}", event);
                                }
                                Err(e) => {
                                    error!("Failed to parse heartbeat: {} | Raw: {}", e, message.data);
                                }
                            }
                        }
                        "error" => {
                            error!("❌ Server Error: {}", message.data);
                        }
                        "" => {
                            // Keep-alive message
                            debug!("💤 Keep-alive message");
                        }
                        other => {
                            info!("❓ Unknown event type '{}': {}", other, message.data);
                        }
                    }
                }
                Err(e) => {
                    error!("❌ SSE Error: {}", e);
                    // Try to reconnect after a delay
                    warn!("🔄 Attempting to reconnect in 5 seconds...");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    break;
                }
            }
        }

        warn!("🔌 SSE connection closed");
        Ok(())
    }

    /// Trigger a custom event for testing
    pub async fn trigger_event(&self, context: Option<&str>, message: Option<&str>) -> Result<()> {
        let mut url = format!("{}/trigger", self.base_url);
        
        let mut params = Vec::new();
        if let Some(context) = context {
            params.push(format!("context={}", context));
        }
        if let Some(message) = message {
            params.push(format!("message={}", urlencoding::encode(message)));
        }
        
        if !params.is_empty() {
            url = format!("{}?{}", url, params.join("&"));
        }

        let response = timeout(
            Duration::from_secs(10),
            self.client.get(&url).send()
        ).await??;

        if response.status().is_success() {
            info!("✅ Event triggered successfully");
        } else {
            error!("❌ Failed to trigger event: {}", response.status());
        }

        Ok(())
    }

    /// Get server statistics
    pub async fn get_stats(&self) -> Result<()> {
        let url = format!("{}/stats", self.base_url);
        
        let response = timeout(
            Duration::from_secs(10),
            self.client.get(&url).send()
        ).await??;

        if response.status().is_success() {
            let stats: serde_json::Value = response.json().await?;
            info!("📊 Server Statistics:");
            println!("{}", serde_json::to_string_pretty(&stats)?);
        } else {
            error!("❌ Failed to get stats: {}", response.status());
        }

        Ok(())
    }
}

// =============================================================================
// Main CLI Application
// =============================================================================

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("sse_client=debug")
        .init();

    let args: Vec<String> = std::env::args().collect();
    
    let server_url = "http://127.0.0.1:8888".to_string();
    let client = SseClient::new(server_url);

    if args.len() > 1 {
        match args[1].as_str() {
            "listen" => {
                // Parse contexts from command line args
                let contexts = if args.len() > 2 {
                    Some(args[2].split(',').map(|s| s.trim().to_string()).collect())
                } else {
                    None
                };

                info!("Starting SSE client...");
                if let Some(ref contexts) = contexts {
                    info!("Subscribing to contexts: {:?}", contexts);
                } else {
                    info!("Subscribing to all events");
                }

                // Start listening with automatic reconnection
                loop {
                    if let Err(e) = client.subscribe(contexts.clone()).await {
                        error!("Connection failed: {}", e);
                        info!("Retrying in 5 seconds...");
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                }
            }
            "trigger" => {
                let context = args.get(2).map(|s| s.as_str());
                let message = args.get(3).map(|s| s.as_str());
                client.trigger_event(context, message).await?;
            }
            "stats" => {
                client.get_stats().await?;
            }
            _ => {
                print_usage();
            }
        }
    } else {
        print_usage();
    }

    Ok(())
}

fn print_usage() {
    println!("SSE Demo Client");
    println!("Usage:");
    println!("  cargo run --bin sse-client listen [contexts]");
    println!("    - Listen for SSE events. Optionally specify comma-separated context IDs");
    println!("    - Examples:");
    println!("      cargo run --bin sse-client listen");
    println!("      cargo run --bin sse-client listen context-1,context-2");
    println!();
    println!("  cargo run --bin sse-client trigger [context] [message]");
    println!("    - Trigger a custom event");
    println!("    - Examples:");
    println!("      cargo run --bin sse-client trigger");
    println!("      cargo run --bin sse-client trigger test-context 'Hello World'");
    println!();
    println!("  cargo run --bin sse-client stats");
    println!("    - Get server connection statistics");
}
