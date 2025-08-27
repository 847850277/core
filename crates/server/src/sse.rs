use axum::extract::{Query, State};
use axum::response::sse::{Event, KeepAlive};
use axum::response::{IntoResponse, Response, Sse};
use calimero_node_primitives::client::NodeClient;
use calimero_primitives::context::ContextId;
use calimero_primitives::events::NodeEvent;
use futures_util::{stream::BoxStream, Stream, StreamExt};
use serde::Deserialize;
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

#[derive(Deserialize)]
pub struct SseEventsQuery {
    pub context_id: Option<String>,
}

pub async fn handle_sse_events(
    State(node_client): State<Arc<NodeClient>>,
    Query(params): Query<SseEventsQuery>,
) -> Response {
    tracing::info!("SSE connection established");
    
    // 解析 context_id 过滤器
    let context_filter = if let Some(context_id_str) = params.context_id {
        tracing::info!("Filtering events for context_id: {}", context_id_str);
        match hex::decode(&context_id_str) {
            Ok(bytes) if bytes.len() == 32 => {
                let mut context_id = [0u8; 32];
                context_id.copy_from_slice(&bytes);
                Some(ContextId::from(context_id))
            }
            _ => {
                tracing::warn!("Invalid context_id format: {}", context_id_str);
                // 无效的 context_id 格式，返回错误响应
                return Response::builder()
                    .status(400)
                    .header("content-type", "text/plain")
                    .body("Invalid context_id format: expected 32-byte hex string".into())
                    .unwrap();
            }
        }
    } else {
        tracing::info!("No context_id filter, streaming all events");
        None
    };

    // 从 node_client 获取真实的事件流
    let events = node_client.receive_events();
    let stream = create_real_sse_event_stream(events, context_filter);

    Sse::new(stream)
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(30)))
        .into_response()
}

fn create_real_sse_event_stream<S>(
    events: S,
    context_filter: Option<ContextId>,
) -> BoxStream<'static, Result<Event, Infallible>>
where
    S: Stream<Item = NodeEvent> + Send + 'static,
{
    tracing::info!("Creating SSE event stream with filter: {:?}", context_filter);
    
    let stream = events
        .filter_map(move |event| {
            let context_filter = context_filter;
            async move {
                tracing::debug!("Received event: {:?}", event);
                // 参考 ws.rs 中 handle_node_events 的过滤逻辑
                match event {
                    NodeEvent::Context(context_event) => {
                        // 如果有 context_filter，只有匹配的 context_id 才通过
                        if let Some(filter_context_id) = context_filter {
                            if context_event.context_id == filter_context_id {
                                tracing::info!("Event matches filter, forwarding");
                                Some(NodeEvent::Context(context_event))
                            } else {
                                tracing::debug!("Event filtered out");
                                None
                            }
                        } else {
                            // 没有过滤器，所有 context 事件都通过
                            tracing::info!("No filter, forwarding event");
                            Some(NodeEvent::Context(context_event))
                        }
                    }
                }
            }
        })
        .map(|event| {
            tracing::info!("Converting event to SSE format");
            // 根据事件类型确定 SSE 事件类型
            let event_type = match &event {
                NodeEvent::Context(context_event) => {
                    match &context_event.payload {
                        calimero_primitives::events::ContextEventPayload::StateMutation(_) => "state_mutation",
                        calimero_primitives::events::ContextEventPayload::ExecutionEvent(_) => "execution_event",
                    }
                }
            };

            // 序列化事件数据，与 WebSocket 中的格式保持一致
            let data = serde_json::to_string(&event).unwrap_or_else(|_| "{}".to_string());

            // 生成唯一的事件 ID（使用时间戳纳秒）
            let event_id = format!("{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0));

            tracing::info!("SSE event created: type={}, id={}", event_type, event_id);

            // 返回格式化的 SSE 事件
            Ok(Event::default()
                .event(event_type)
                .data(data)
                .id(event_id))
        });

    Box::pin(stream)
}