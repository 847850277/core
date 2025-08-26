use crate::SseEventsQuery;
use axum::extract::{Query, State};
use axum::response::sse::{Event, KeepAlive};
use axum::response::{IntoResponse, Response, Sse};
use calimero_node_primitives::client::NodeClient;
use calimero_primitives::events::NodeEvent;
use futures_util::{stream::BoxStream, StreamExt};
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;

pub async fn handle_sse_events(
    State(node_client): State<Arc<NodeClient>>,
    Query(params): Query<SseEventsQuery>,
) -> Response {
    // 创建一个虚拟的广播通道用于演示
    // 在真实实现中，这应该来自 node_client 的事件流
    let (tx, _) = broadcast::channel::<NodeEvent>(100);
    let stream = create_sse_event_stream(tx.subscribe(), params.context_id).await;

    Sse::new(stream)
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(30)))
        .into_response()
}

async fn create_sse_event_stream(
    mut receiver: broadcast::Receiver<NodeEvent>,
    context_id_filter: Option<String>,
) -> BoxStream<'static, Result<Event, Infallible>> {
    let stream = BroadcastStream::new(receiver);

    let stream = stream
        .filter_map(|result| async move { result.ok() })
        .filter_map(move |event| {
            let context_filter = context_id_filter.clone();
            async move {
                match &event {
                    NodeEvent::Context(context_event) => {
                        if let Some(ref filter_id) = context_filter {
                            let context_id_str = hex::encode(context_event.context_id.as_ref());
                            if context_id_str != *filter_id {
                                return None;
                            }
                        }
                        Some(event)
                    }
                }
            }
        })
        .map(|event| {
            let event_type = match &event {
                NodeEvent::Context(context_event) => {
                    match &context_event.payload {
                        calimero_primitives::events::ContextEventPayload::StateMutation(_) => "state_mutation",
                        calimero_primitives::events::ContextEventPayload::ExecutionEvent(_) => "execution_event",
                    }
                }
            };

            let data = serde_json::to_string(&event).unwrap_or_else(|_| "{}".to_string());

            Ok(Event::default()
                .event(event_type)
                .data(data)
                .id(format!(
                    "{}",
                    chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
                )))
        });

    Box::pin(stream)
}