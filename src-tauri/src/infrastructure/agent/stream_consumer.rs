use crate::domain::chat::StreamEvent;
use crate::domain::errors::AppError;
use agent_rs_lib::agent::agents::ContextManagedChatStream;
use futures::StreamExt;
use rig_core::streaming::{StreamedAssistantContent, ToolCallDeltaContent};

pub async fn consume_chat_stream<S, R>(
    mut stream: ContextManagedChatStream<S, R>,
    rx: tokio::sync::oneshot::Receiver<Vec<rig_core::message::Message>>,
    channel: &tauri::ipc::Channel<StreamEvent>,
) -> Result<Vec<rig_core::message::Message>, AppError>
where
    S: futures::Stream<
            Item = Result<rig_core::agent::MultiTurnStreamItem<R>, rig_core::agent::StreamingError>,
        > + Unpin,
    R: Unpin,
{
    let mut aborted = false;
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(rig_core::agent::MultiTurnStreamItem::StreamAssistantItem(
                StreamedAssistantContent::Text(text),
            )) => {
                if channel
                    .send(StreamEvent::Text {
                        delta: text.to_string(),
                    })
                    .is_err()
                {
                    aborted = true;
                    break;
                }
            }
            Ok(rig_core::agent::MultiTurnStreamItem::StreamAssistantItem(
                StreamedAssistantContent::Reasoning(r),
            )) => {
                let text = r.display_text();
                let id = r.id.clone().unwrap_or_default();
                if channel
                    .send(StreamEvent::Reasoning {
                        id,
                        delta: text,
                        is_final: true,
                    })
                    .is_err()
                {
                    aborted = true;
                    break;
                }
            }
            Ok(rig_core::agent::MultiTurnStreamItem::StreamAssistantItem(
                StreamedAssistantContent::ReasoningDelta { id, reasoning },
            )) => {
                if channel
                    .send(StreamEvent::Reasoning {
                        id: id.unwrap_or_default(),
                        delta: reasoning,
                        is_final: false,
                    })
                    .is_err()
                {
                    aborted = true;
                    break;
                }
            }
            Ok(rig_core::agent::MultiTurnStreamItem::StreamAssistantItem(
                StreamedAssistantContent::ToolCall {
                    tool_call,
                    internal_call_id,
                },
            )) => {
                let id = internal_call_id;
                if channel
                    .send(StreamEvent::ToolCallStart {
                        id: id.clone(),
                        name: tool_call.function.name.clone(),
                    })
                    .is_err()
                {
                    aborted = true;
                    break;
                }
                let args_str = tool_call.function.arguments.to_string();
                if channel
                    .send(StreamEvent::ToolCallEnd { id, args: args_str })
                    .is_err()
                {
                    aborted = true;
                    break;
                }
            }
            Ok(rig_core::agent::MultiTurnStreamItem::StreamAssistantItem(
                StreamedAssistantContent::ToolCallDelta {
                    internal_call_id,
                    content,
                    ..
                },
            )) => {
                let id = internal_call_id;
                match content {
                    ToolCallDeltaContent::Name(name) => {
                        if channel
                            .send(StreamEvent::ToolCallStart { id, name })
                            .is_err()
                        {
                            aborted = true;
                            break;
                        }
                    }
                    ToolCallDeltaContent::Delta(args_delta) => {
                        if channel
                            .send(StreamEvent::ToolCallDelta { id, args_delta })
                            .is_err()
                        {
                            aborted = true;
                            break;
                        }
                    }
                }
            }
            Ok(rig_core::agent::MultiTurnStreamItem::FinalResponse(resp)) => {
                tracing::debug!("Stream final response: {:?}", resp);
            }
            Err(e) => {
                return Err(AppError::SystemError(e.to_string()));
            }
            _other => {
                tracing::warn!("unhandled MultiTurnStreamItem variant from agent_rs stream");
            }
        }
    }

    if aborted {
        return Err(AppError::SystemError(
            "Stream aborted: channel closed".to_string(),
        ));
    }

    drop(stream);

    let raw_history = rx.await.map_err(|e| AppError::SystemError(e.to_string()))?;
    let updated_history = agent_rs_lib::agent::strip_reasoning_from_history(raw_history);
    Ok(updated_history)
}
