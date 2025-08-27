use std::borrow::Cow;
use std::process::Stdio;

use calimero_primitives::alias::Alias;
use calimero_primitives::context::ContextId;
use calimero_primitives::events::NodeEvent;
use clap::Parser;
use comfy_table::{Cell, Color, Table};
use eyre::{OptionExt, Result};
use reqwest_eventsource::{Event, EventSource};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use futures_util::StreamExt;

use crate::cli::Environment;
use crate::common::resolve_alias;
use crate::output::{ErrorLine, InfoLine, Report};

pub const EXAMPLES: &str = r#"
  # Watch events from default context using SSE
  $ meroctl context watch

  # Watch events and show notification
  $ meroctl context watch -x notify-send "New event"

  # Watch events and log to file (first 10 events)
  $ meroctl context watch -x sh -c "echo 'Event received' >> events.log" -n 10

  # Watch events and run custom script with arguments
  $ meroctl context watch -x ./my-script.sh --arg1 value1
"#;

#[derive(Debug, Parser)]
#[command(after_help = EXAMPLES)]
#[command(about = "Watch events from a context using SSE")]
pub struct WatchSseCommand {
    /// ContextId to stream events from
    #[arg(
        value_name = "CONTEXT",
        help = "Context to stream events from",
        default_value = "default"
    )]
    pub context: Alias<ContextId>,

    /// Command to execute when an event is received (can specify multiple args)
    #[arg(short = 'x', long, value_name = "COMMAND", num_args = 1..)]
    pub exec: Option<Vec<String>>,

    /// Maximum number of events to process before exiting
    #[arg(short = 'n', long, value_name = "COUNT")]
    pub count: Option<usize>,
}

#[derive(Serialize, Deserialize, Debug)]
struct SseEventInfo {
    event_type: String,
    event_id: String,
    data: NodeEvent,
}

impl Report for SseEventInfo {
    fn report(&self) {
        let mut table = Table::new();
        let _ = table.set_header(vec![Cell::new("SSE Event").fg(Color::Green)]);

        let _ = table.add_row(vec![format!("Event Type: {}", self.event_type)]);
        let _ = table.add_row(vec![format!("Event ID: {}", self.event_id)]);
        
        // 格式化事件数据
        match &self.data {
            NodeEvent::Context(ctx_event) => {
                let _ = table.add_row(vec![format!("Context ID: {}", ctx_event.context_id)]);
                match &ctx_event.payload {
                    calimero_primitives::events::ContextEventPayload::StateMutation(state_mut) => {
                        let _ = table.add_row(vec![format!("Event: State Mutation")]);
                        let _ = table.add_row(vec![format!("New Root: {}", state_mut.new_root)]);
                    }
                    calimero_primitives::events::ContextEventPayload::ExecutionEvent(exec_event) => {
                        let _ = table.add_row(vec![format!("Event: Execution Event")]);
                        let _ = table.add_row(vec![format!("Events Count: {}", exec_event.events.len())]);
                        for (i, event) in exec_event.events.iter().enumerate() {
                            let data_str = String::from_utf8_lossy(&event.data);
                            let _ = table.add_row(vec![format!("  Event {}: {} - {}", i + 1, event.kind, data_str)]);
                        }
                    }
                }
            }
        }

        println!("{table}");
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct ExecutionOutput<'a> {
    #[serde(borrow)]
    cmd: Cow<'a, [String]>,
    status: Option<i32>,
    stdout: String,
    stderr: String,
}

impl Report for ExecutionOutput<'_> {
    fn report(&self) {
        let mut table = Table::new();
        let _ = table.add_row(vec![format!("Command: {}", self.cmd.join(" "))]);
        let _ = table.add_row(vec![format!("Status: {:?}", self.status)]);
        let _ = table.add_row(vec![format!("Stdout: {}", self.stdout)]);
        let _ = table.add_row(vec![format!("Stderr: {}", self.stderr)]);

        println!("{table}");
    }
}

impl WatchSseCommand {
    pub async fn run(self, environment: &Environment) -> Result<()> {
        let connection = environment.connection()?;

        let resolve_response = resolve_alias(connection, self.context, None).await?;

        let context_id = resolve_response
            .value()
            .cloned()
            .ok_or_eyre("Failed to resolve context: no value found")?;

        // 构建SSE URL - 现在 SSE 在独立的 /events 路径
        let mut sse_url = connection.api_url.clone();
        sse_url.set_path("events");
        
        // 如果有具体的 context_id，将 Base58 编码的 ContextId 转换为十六进制格式
        if context_id.to_string() != "default" {
            let context_id_hex = hex::encode(context_id.as_ref());
            sse_url.set_query(Some(&format!("context_id={}", context_id_hex)));
        }
        // 如果是 default，不设置 context_id 参数，监听所有事件

        environment
            .output
            .write(&InfoLine(&format!("Connecting to SSE endpoint: {}", sse_url)));

        // 创建SSE客户端
        let mut es = EventSource::get(sse_url.as_str());

        environment
            .output
            .write(&InfoLine(&format!("Subscribed to context {}", context_id)));

        if let Some(cmd) = &self.exec {
            environment.output.write(&InfoLine(&format!(
                "Will execute command: {}",
                cmd.join(" ")
            )));
        }

        environment
            .output
            .write(&InfoLine("Streaming events (press Ctrl+C to stop):"));

        let mut event_count = 0;
        while let Some(event) = es.next().await {
            match event {
                Ok(Event::Open) => {
                    environment
                        .output
                        .write(&InfoLine("🔗 SSE connection opened"));
                }
                Ok(Event::Message(message)) => {
                    // 创建事件信息结构
                    let event_info = SseEventInfo {
                        event_type: message.event.clone(),
                        event_id: message.id.clone(),
                        data: match serde_json::from_str::<NodeEvent>(&message.data) {
                            Ok(event) => event,
                            Err(e) => {
                                environment
                                    .output
                                    .write(&ErrorLine(&format!("Failed to parse event data: {}", e)));
                                continue;
                            }
                        },
                    };

                    environment.output.write(&event_info);

                    if let Some(cmd) = &self.exec {
                        if let Some(max_count) = self.count {
                            if event_count >= max_count {
                                break;
                            }
                        }

                        let mut child = Command::new(&cmd[0])
                            .args(&cmd[1..])
                            .stdin(Stdio::piped())
                            .spawn()?;

                        let stdin = child.stdin.take();

                        let event_data = message.data.clone();
                        let stdin = tokio::spawn(async move {
                            let Some(mut stdin) = stdin else {
                                return Ok(());
                            };

                            stdin.write_all(event_data.as_bytes()).await
                        });

                        let output = child
                            .wait_with_output()
                            .await
                            .map_err(|e| eyre::eyre!("Failed to execute command: {}", e))?;

                        stdin.await??;

                        let outcome = ExecutionOutput {
                            cmd: cmd.into(),
                            status: output.status.code(),
                            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                        };

                        environment.output.write(&outcome);
                    }

                    event_count += 1;

                    if let Some(max_count) = self.count {
                        if event_count >= max_count {
                            break;
                        }
                    }
                }
                Err(err) => {
                    environment
                        .output
                        .write(&ErrorLine(&format!("SSE error: {}", err)));
                    break;
                }
            }
        }

        Ok(())
    }
}
