use std::sync::Arc;

use axum::extract::{Query, State};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::Response;
use axum::response::IntoResponse;
use axum::routing::{get, post, Router};
use axum::{Extension, Json};
use calimero_context_primitives::client::ContextClient;
use calimero_node_primitives::client::NodeClient;
use calimero_primitives::events::NodeEvent;
use calimero_server_primitives::jsonrpc::{
    Request as PrimitiveRequest, RequestPayload, Response as PrimitiveResponse, ResponseBody,
    ResponseBodyError, ResponseBodyResult, ServerResponseError,
};
use chrono;
use futures_util::{stream::BoxStream, Stream, StreamExt};
use serde::{Deserialize, Serialize};
use std::{convert::Infallible, time::Duration};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tracing::{debug, info};

use crate::config::ServerConfig;

mod execute;

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[non_exhaustive]
pub struct JsonRpcConfig {
    #[serde(default = "calimero_primitives::common::bool_true")]
    pub enabled: bool,
}

impl JsonRpcConfig {
    #[must_use]
    pub const fn new(enabled: bool) -> Self {
        Self { enabled }
    }
}

pub(crate) struct ServiceState {
    ctx_client: ContextClient,
    node_client: Option<NodeClient>,
}

pub(crate) fn service(
    config: &ServerConfig,
    ctx_client: ContextClient,
    node_client: Option<NodeClient>,
) -> Option<(&'static str, Router)> {
    let path = "/jsonrpc"; // todo! source from config

    for listen in &config.listen {
        info!("JSON RPC server listening on {}/http{{{}}}", listen, path);
    }

    let state = Arc::new(ServiceState { 
        ctx_client,
        node_client: node_client.clone(),
    });
    let rpc_handler = post(handle_request).layer(Extension(Arc::clone(&state)));

    let router = Router::new().route("/", rpc_handler);

    Some((path, router))
}

async fn handle_request(
    Extension(state): Extension<Arc<ServiceState>>,
    Json(request): Json<PrimitiveRequest<serde_json::Value>>,
) -> Json<PrimitiveResponse> {
    debug!(id=?request.id, payload=%request.payload, "Received request");

    let body = match serde_json::from_value(request.payload) {
        Ok(payload) => match payload {
            RequestPayload::Execute(request) => request.handle(state).await.to_res_body(),
        },
        Err(err) => {
            debug!(%err, "Failed to deserialize RequestPayload");

            ResponseBody::Error(ResponseBodyError::ServerError(
                ServerResponseError::ParseError(err.to_string()),
            ))
        }
    };

    if let ResponseBody::Error(err) = &body {
        debug!(id=?request.id, ?err, "request handling failed");
    }

    PrimitiveResponse::new(request.jsonrpc, request.id, body).into()
}

pub(crate) trait Request {
    type Response;
    type Error;

    async fn handle(
        self,
        state: Arc<ServiceState>,
    ) -> Result<Self::Response, RpcError<Self::Error>>;
}

#[derive(Debug)]
#[non_exhaustive]
pub enum RpcError<E> {
    MethodCallError(E),
    InternalError(eyre::Report),
}

impl<E, X: Into<eyre::Report>> From<X> for RpcError<E> {
    fn from(err: X) -> Self {
        RpcError::InternalError(err.into())
    }
}

trait ToResponseBody {
    fn to_res_body(self) -> ResponseBody;
}

impl<T: Serialize, E: Serialize> ToResponseBody for Result<T, RpcError<E>> {
    fn to_res_body(self) -> ResponseBody {
        let err = match self {
            Ok(r) => match serde_json::to_value(r) {
                Ok(v) => return ResponseBody::Result(ResponseBodyResult(v)),
                Err(err) => err.into(),
            },
            Err(RpcError::MethodCallError(err)) => match serde_json::to_value(err) {
                Ok(v) => return ResponseBody::Error(ResponseBodyError::HandlerError(v)),
                Err(err) => err.into(),
            },
            Err(RpcError::InternalError(err)) => err,
        };

        ResponseBody::Error(ResponseBodyError::ServerError(
            ServerResponseError::InternalError { err: Some(err) },
        ))
    }
}

// SSE 相关实现
#[derive(serde::Deserialize)]
struct SseEventsQuery {
    context_id: Option<String>,
}

async fn handle_sse_events(
    State(state): State<Arc<ServiceState>>,
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
