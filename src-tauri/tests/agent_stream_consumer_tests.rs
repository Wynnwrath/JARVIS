use agent_rs_lib::agent::agents::ContextManagedChatStream;
use futures::stream;
use jarvis_lib::domain::chat::StreamEvent;
use jarvis_lib::domain::errors::AppError;
use jarvis_lib::infrastructure::agent::consume_chat_stream;
use rig_core::agent::MultiTurnStreamItem;
use rig_core::completion::message::{Text, ToolCall, ToolFunction};
use rig_core::streaming::StreamedAssistantContent;
use std::sync::{Arc, Mutex};

type MockStream = stream::Iter<
    std::vec::IntoIter<Result<MultiTurnStreamItem<()>, rig_core::agent::StreamingError>>,
>;

fn make_test_channel() -> (
    tauri::ipc::Channel<StreamEvent>,
    Arc<Mutex<Vec<StreamEvent>>>,
) {
    let events: Arc<Mutex<Vec<StreamEvent>>> = Arc::new(Mutex::new(Vec::new()));
    let events_clone = Arc::clone(&events);
    let channel = tauri::ipc::Channel::new(move |response| {
        let body = match response {
            tauri::ipc::InvokeResponseBody::Json(s) => s.into_bytes(),
            tauri::ipc::InvokeResponseBody::Raw(v) => v,
        };
        if let Ok(event) = serde_json::from_slice::<StreamEvent>(&body) {
            events_clone.lock().unwrap().push(event);
        }
        Ok(())
    });
    (channel, events)
}

fn make_stream(
    items: Vec<Result<MultiTurnStreamItem<()>, rig_core::agent::StreamingError>>,
) -> ContextManagedChatStream<MockStream, ()> {
    let (tx, _rx) = tokio::sync::oneshot::channel();
    ContextManagedChatStream::new(stream::iter(items), tx, vec![])
}

fn make_history_rx(
    history: Vec<rig_core::message::Message>,
) -> tokio::sync::oneshot::Receiver<Vec<rig_core::message::Message>> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let _ = tx.send(history);
    rx
}

#[tokio::test]
async fn text_delta_sends_text_event() {
    let (channel, events) = make_test_channel();
    let stream = make_stream(vec![Ok(MultiTurnStreamItem::StreamAssistantItem(
        StreamedAssistantContent::Text(Text::new("hello")),
    ))]);
    let rx = make_history_rx(vec![]);

    let result = consume_chat_stream(stream, rx, &channel).await;
    assert!(result.is_ok());

    let captured = events.lock().unwrap();
    assert_eq!(captured.len(), 1);
    match &captured[0] {
        StreamEvent::Text { delta } => assert_eq!(delta, "hello"),
        other => panic!("expected Text event, got {:?}", other),
    }
}

#[tokio::test]
async fn tool_call_sends_start_and_end() {
    let (channel, events) = make_test_channel();
    let tool_call = ToolCall::new(
        "tc1".to_string(),
        ToolFunction::new("read_file".to_string(), serde_json::json!({"path": "/foo"})),
    );
    let stream = make_stream(vec![Ok(MultiTurnStreamItem::StreamAssistantItem(
        StreamedAssistantContent::ToolCall {
            tool_call,
            internal_call_id: "ic1".to_string(),
        },
    ))]);
    let rx = make_history_rx(vec![]);

    let result = consume_chat_stream(stream, rx, &channel).await;
    assert!(result.is_ok());

    let captured = events.lock().unwrap();
    assert_eq!(captured.len(), 2);
    match &captured[0] {
        StreamEvent::ToolCallStart { id, name } => {
            assert_eq!(id, "ic1");
            assert_eq!(name, "read_file");
        }
        other => panic!("expected ToolCallStart, got {:?}", other),
    }
    match &captured[1] {
        StreamEvent::ToolCallEnd { id, args } => {
            assert_eq!(id, "ic1");
            assert!(args.contains("read_file") || args.contains("/foo"));
        }
        other => panic!("expected ToolCallEnd, got {:?}", other),
    }
}

#[tokio::test]
async fn final_response_logs_and_completes() {
    let (channel, _events) = make_test_channel();
    let final_resp = rig_core::agent::FinalResponse::empty();
    let stream = make_stream(vec![Ok(MultiTurnStreamItem::FinalResponse(final_resp))]);
    let rx = make_history_rx(vec![]);

    let result = consume_chat_stream(stream, rx, &channel).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn stream_error_returns_system_error() {
    let (channel, _events) = make_test_channel();
    let err = rig_core::agent::StreamingError::Completion(
        rig_core::completion::CompletionError::RequestError(Box::new(std::io::Error::other(
            "test error",
        ))),
    );
    let stream = make_stream(vec![Err(err)]);
    let rx = make_history_rx(vec![]);

    let result = consume_chat_stream(stream, rx, &channel).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        AppError::SystemError(msg) => assert!(msg.contains("test error")),
        other => panic!("expected SystemError, got {:?}", other),
    }
}
