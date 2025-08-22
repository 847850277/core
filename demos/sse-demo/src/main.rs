use axum::{
    extract::{Query, State},
    http::{
        header::{CACHE_CONTROL, CONTENT_TYPE},
        HeaderMap, HeaderValue, StatusCode,
    },
    response::{
        sse::{Event, KeepAlive, Sse},
        Html, IntoResponse,
    },
    routing::get,
    Router,
};
use eyre::Result;
use futures_util::Stream;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    convert::Infallible,
    sync::Arc,
    time::Duration,
};
use tokio::{
    sync::{broadcast, RwLock},
    time::interval,
};
use tokio_stream::wrappers::BroadcastStream;
use tracing::{debug, info, warn};
use uuid::Uuid;

// =============================================================================
// Types and Data Structures
// =============================================================================

/// Represents different types of events that can be sent to clients
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum NodeEvent {
    /// Context-related events (similar to the original WebSocket implementation)
    Context {
        context_id: String,
        event_type: String,
        data: serde_json::Value,
        timestamp: u64,
    },
    /// System events
    System {
        message: String,
        level: String,
        timestamp: u64,
    },
    /// Heartbeat events
    Heartbeat { timestamp: u64 },
}

/// Server application state
#[derive(Debug)]
pub struct AppState {
    /// Event broadcaster - sends events to all connected clients
    event_sender: broadcast::Sender<NodeEvent>,
    /// Track subscriptions per connection
    subscriptions: RwLock<HashMap<String, HashSet<String>>>,
    /// Connection counter for demo purposes
    connection_count: RwLock<u32>,
}

/// Query parameters for SSE endpoint
#[derive(Debug, Deserialize)]
pub struct SseParams {
    /// Connection ID (optional, will be generated if not provided)
    connection_id: Option<String>,
    /// Context IDs to subscribe to (comma-separated)
    contexts: Option<String>,
}

// =============================================================================
// SSE Handler
// =============================================================================

/// Main SSE endpoint handler
pub async fn sse_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SseParams>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, StatusCode> {
    // Generate or use provided connection ID
    let connection_id = params
        .connection_id
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    // Parse context subscriptions
    let contexts: HashSet<String> = params
        .contexts
        .map(|contexts| {
            contexts
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();

    // Store subscription information
    {
        let mut subscriptions = state.subscriptions.write().await;
        subscriptions.insert(connection_id.clone(), contexts.clone());
    }

    // Increment connection counter
    {
        let mut count = state.connection_count.write().await;
        *count += 1;
        info!(
            connection_id = %connection_id,
            contexts = ?contexts,
            total_connections = *count,
            "New SSE connection established"
        );
    }

    // Create event stream
    let event_stream = create_event_stream(state.clone(), connection_id.clone(), contexts).await;

    Ok(Sse::new(event_stream)
        .keep_alive(
            KeepAlive::new()
                .interval(Duration::from_secs(10))
                .text("keep-alive"),
        ))
}

/// Creates an event stream for a specific connection
async fn create_event_stream(
    state: Arc<AppState>,
    connection_id: String,
    subscribed_contexts: HashSet<String>,
) -> impl Stream<Item = Result<Event, Infallible>> {
    // Subscribe to the broadcast channel
    let receiver = state.event_sender.subscribe();
    let broadcast_stream = BroadcastStream::new(receiver);

    // Create stream that filters events based on subscriptions  
    futures_util::StreamExt::filter_map(
        futures_util::StreamExt::then(broadcast_stream, move |result| {
            let subscribed_contexts = subscribed_contexts.clone();
            let connection_id = connection_id.clone();
            
            async move {
                match result {
                    Ok(event) => {
                        // Filter events based on subscription
                        let should_send = match &event {
                            NodeEvent::Context { context_id, .. } => {
                                subscribed_contexts.is_empty() || subscribed_contexts.contains(context_id)
                            }
                            NodeEvent::System { .. } | NodeEvent::Heartbeat { .. } => true,
                        };

                        if should_send {
                            debug!(
                                connection_id = %connection_id,
                                event_type = ?event,
                                "Sending event to client"
                            );

                            // Serialize event to JSON
                            match serde_json::to_string(&event) {
                                Ok(json_data) => {
                                    Some(Ok(Event::default()
                                        .event(match &event {
                                            NodeEvent::Context { .. } => "context",
                                            NodeEvent::System { .. } => "system", 
                                            NodeEvent::Heartbeat { .. } => "heartbeat",
                                        })
                                        .data(json_data)))
                                }
                                Err(e) => {
                                    warn!(
                                        connection_id = %connection_id,
                                        error = %e,
                                        "Failed to serialize event"
                                    );
                                    None
                                }
                            }
                        } else {
                            // Event filtered out
                            None
                        }
                    }
                    Err(err) => {
                        // Handle different error types based on the actual BroadcastStreamRecvError
                        warn!(
                            connection_id = %connection_id,
                            error = ?err,
                            "Stream error occurred"
                        );
                        // Send an error notification
                        Some(Ok(Event::default()
                            .event("error")
                            .data(format!("{{\"type\":\"stream_error\",\"message\":\"{}\"}}", err))))
                    }
                }
            }
        }),
        |option| async move { option }
    )
}

// =============================================================================
// Event Generation (Simulates Node Events)
// =============================================================================

/// Spawns a background task that generates demo events
pub async fn spawn_event_generator(state: Arc<AppState>) {
    tokio::spawn(async move {
        let mut heartbeat_interval = interval(Duration::from_secs(30));
        let mut context_event_interval = interval(Duration::from_secs(5));
        let mut system_event_interval = interval(Duration::from_secs(15));

        let contexts = vec!["context-1", "context-2", "context-3"];
        let mut counter = 0u64;

        loop {
            tokio::select! {
                _ = heartbeat_interval.tick() => {
                    let event = NodeEvent::Heartbeat {
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs(),
                    };
                    
                    if let Err(e) = state.event_sender.send(event) {
                        debug!("No active SSE connections: {}", e);
                    }
                }
                
                _ = context_event_interval.tick() => {
                    counter += 1;
                    let context_id = contexts[counter as usize % contexts.len()];
                    
                    let event = NodeEvent::Context {
                        context_id: context_id.to_string(),
                        event_type: "execution".to_string(),
                        data: serde_json::json!({
                            "method": "update_state",
                            "result": {
                                "success": true,
                                "value": counter,
                                "message": format!("Executed operation #{}", counter)
                            }
                        }),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs(),
                    };
                    
                    info!("Generated context event for {}: {}", context_id, counter);
                    if let Err(e) = state.event_sender.send(event) {
                        debug!("No active SSE connections: {}", e);
                    }
                }
                
                _ = system_event_interval.tick() => {
                    let event = NodeEvent::System {
                        message: format!("System status check #{}", counter),
                        level: "info".to_string(),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs(),
                    };
                    
                    if let Err(e) = state.event_sender.send(event) {
                        debug!("No active SSE connections: {}", e);
                    }
                }
            }
        }
    });
}

// =============================================================================
// HTTP Endpoints
// =============================================================================

/// Serves the demo HTML page
pub async fn serve_demo_page() -> impl IntoResponse {
    Html(include_str!("../static/index.html"))
}

/// API endpoint to trigger custom events (for testing)
pub async fn trigger_event(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let context_id = params
        .get("context")
        .unwrap_or(&"test-context".to_string())
        .clone();
    
    let message = params
        .get("message")
        .unwrap_or(&"Manual trigger".to_string())
        .clone();

    let event = NodeEvent::Context {
        context_id,
        event_type: "manual".to_string(),
        data: serde_json::json!({
            "trigger": "api",
            "message": message,
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        }),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    };

    match state.event_sender.send(event) {
        Ok(_) => {
            let mut headers = HeaderMap::new();
            headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
            (StatusCode::OK, headers, r#"{"status":"sent"}"#)
        }
        Err(_) => {
            let mut headers = HeaderMap::new();
            headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
            (StatusCode::INTERNAL_SERVER_ERROR, headers, r#"{"status":"error","message":"No receivers"}"#)
        }
    }
}

/// Get connection statistics
pub async fn get_stats(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let connection_count = *state.connection_count.read().await;
    let subscriptions = state.subscriptions.read().await;
    
    let stats = serde_json::json!({
        "active_connections": connection_count,
        "subscription_count": subscriptions.len(),
        "subscriptions": subscriptions.clone()
    });

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    
    (StatusCode::OK, headers, stats.to_string())
}

// =============================================================================
// Main Application Setup
// =============================================================================

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("sse_demo=debug,tower_http=debug")
        .init();

    info!("Starting SSE Demo Server");

    // Create application state
    let (event_sender, _) = broadcast::channel(1000);
    let state = Arc::new(AppState {
        event_sender,
        subscriptions: RwLock::new(HashMap::new()),
        connection_count: RwLock::new(0),
    });

    // Start event generator
    spawn_event_generator(state.clone()).await;

    // Build router
    let app = Router::new()
        .route("/", get(serve_demo_page))
        .route("/events", get(sse_handler))
        .route("/trigger", get(trigger_event))
        .route("/stats", get(get_stats))
        .with_state(state)
        .layer(
            tower_http::cors::CorsLayer::new()
                .allow_origin(tower_http::cors::Any)
                .allow_methods([axum::http::Method::GET])
                .allow_headers([axum::http::header::CONTENT_TYPE]),
        );

    // Start server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8888").await?;
        info!("SSE Server listening on http://127.0.0.1:8888");
    info!("Visit the URL in your browser to see the demo");
    info!("Available endpoints:");
    info!("  GET /            - Demo HTML page");
    info!("  GET /events      - SSE endpoint (subscribe with ?contexts=context-1,context-2)");
    info!("  GET /trigger     - Trigger custom event (?context=test&message=hello)");
    info!("  GET /stats       - Connection statistics");

    axum::serve(listener, app).await?;

    Ok(())
}
