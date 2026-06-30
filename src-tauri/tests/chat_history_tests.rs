use jarvis_lib::handlers::chat::history::{
    assistant_message_text, deduplicate_consecutive_assistant_messages,
    prepare_prompt_with_attachments, update_history_with_clean_user_message,
};
use rig_core::completion::message::{Text, ToolCall, ToolFunction};
use rig_core::message::{AssistantContent, Message, UserContent};
use rig_core::OneOrMany;

fn user_msg(text: &str) -> Message {
    Message::User {
        content: OneOrMany::one(UserContent::Text(Text::new(text.to_string()))),
    }
}

fn assistant_msg(text: &str) -> Message {
    Message::Assistant {
        content: OneOrMany::one(AssistantContent::Text(Text::new(text.to_string()))),
        id: None,
    }
}

fn assistant_msg_with_tool(text: &str, tool_name: &str) -> Message {
    let items = vec![
        AssistantContent::Text(Text::new(text.to_string())),
        AssistantContent::ToolCall(ToolCall::new(
            "call_1".to_string(),
            ToolFunction::new(tool_name.to_string(), serde_json::json!({})),
        )),
    ];
    let content: OneOrMany<AssistantContent> = OneOrMany::many(items).unwrap();
    Message::Assistant { content, id: None }
}

// ── assistant_message_text ──────────────────────────────────────

#[test]
fn assistant_message_text_single_block() {
    let content: OneOrMany<AssistantContent> =
        OneOrMany::one(AssistantContent::Text(Text::new("hello".to_string())));
    assert_eq!(assistant_message_text(&content), "hello");
}

#[test]
fn assistant_message_text_multiple_text_blocks() {
    let content: OneOrMany<AssistantContent> = OneOrMany::many(vec![
        AssistantContent::Text(Text::new("hello".to_string())),
        AssistantContent::Text(Text::new("world".to_string())),
    ])
    .unwrap();
    assert_eq!(assistant_message_text(&content), "hello\nworld");
}

#[test]
fn assistant_message_text_mixed_content_filters_non_text() {
    let content: OneOrMany<AssistantContent> = OneOrMany::many(vec![
        AssistantContent::Text(Text::new("answer".to_string())),
        AssistantContent::ToolCall(ToolCall::new(
            "call_1".to_string(),
            ToolFunction::new("search".to_string(), serde_json::json!({})),
        )),
    ])
    .unwrap();
    assert_eq!(assistant_message_text(&content), "answer");
}

// ── deduplicate_consecutive_assistant_messages ──────────────────

#[test]
fn dedup_empty_history() {
    let result = deduplicate_consecutive_assistant_messages(vec![]);
    assert!(result.is_empty());
}

#[test]
fn dedup_no_duplicates() {
    let history = vec![user_msg("hi"), assistant_msg("a"), user_msg("bye")];
    let result = deduplicate_consecutive_assistant_messages(history);
    assert_eq!(result.len(), 3);
}

#[test]
fn dedup_two_consecutive_identical_text_only() {
    let history = vec![assistant_msg("dup"), assistant_msg("dup")];
    let result = deduplicate_consecutive_assistant_messages(history);
    assert_eq!(result.len(), 1);
}

#[test]
fn dedup_three_consecutive_identical() {
    let history = vec![
        assistant_msg("dup"),
        assistant_msg("dup"),
        assistant_msg("dup"),
    ];
    let result = deduplicate_consecutive_assistant_messages(history);
    assert_eq!(result.len(), 1);
}

#[test]
fn dedup_non_consecutive_duplicates_kept() {
    let history = vec![assistant_msg("a"), user_msg("q"), assistant_msg("a")];
    let result = deduplicate_consecutive_assistant_messages(history);
    assert_eq!(result.len(), 3);
}

#[test]
fn dedup_user_message_interleaving() {
    let history = vec![
        user_msg("q1"),
        assistant_msg("a1"),
        user_msg("q2"),
        assistant_msg("a1"),
    ];
    let result = deduplicate_consecutive_assistant_messages(history);
    assert_eq!(result.len(), 4);
}

#[test]
fn dedup_all_assistant_identical() {
    let history = vec![
        assistant_msg("x"),
        assistant_msg("x"),
        assistant_msg("x"),
        assistant_msg("x"),
    ];
    let result = deduplicate_consecutive_assistant_messages(history);
    assert_eq!(result.len(), 1);
}

#[test]
fn dedup_with_tools_replaces_text_only() {
    let history = vec![
        assistant_msg("answer"),
        assistant_msg_with_tool("answer", "search"),
    ];
    let result = deduplicate_consecutive_assistant_messages(history);
    assert_eq!(result.len(), 1);
    // The surviving message should be the one with the tool call.
    if let Message::Assistant { content, .. } = &result[0] {
        assert!(content
            .iter()
            .any(|item| matches!(item, AssistantContent::ToolCall(_))));
    } else {
        panic!("expected assistant message");
    }
}

#[test]
fn dedup_both_with_tools_not_deduped() {
    let history = vec![
        assistant_msg_with_tool("answer", "search"),
        assistant_msg_with_tool("answer", "read"),
    ];
    let result = deduplicate_consecutive_assistant_messages(history);
    assert_eq!(result.len(), 2);
}

// ── update_history_with_clean_user_message ──────────────────────

#[test]
fn clean_user_msg_replaces_attachment_metadata() {
    let mut history = vec![
        user_msg("[Attached Document: /path/file.txt]\nOriginal prompt"),
        assistant_msg("response"),
    ];
    update_history_with_clean_user_message(
        &mut history,
        "New prompt",
        Some(&["/path/file.txt".to_string()]),
    );

    if let Message::User { content } = &history[0] {
        let text = match content.first_ref() {
            UserContent::Text(t) => t.text.as_str(),
            _ => panic!("expected text"),
        };
        assert!(text.starts_with("[Attached: /path/file.txt]"));
        assert!(text.contains("New prompt"));
        assert!(!text.contains("[Attached Document:"));
    } else {
        panic!("expected user message");
    }
}

#[test]
fn clean_user_msg_noop_without_attachments() {
    let mut history = vec![user_msg("hello")];
    update_history_with_clean_user_message(&mut history, "hello", None);
    if let Message::User { content } = &history[0] {
        let text = match content.first_ref() {
            UserContent::Text(t) => t.text.as_str(),
            _ => panic!("expected text"),
        };
        assert_eq!(text, "hello");
    }
}

#[test]
fn clean_user_msg_noop_with_empty_attachments() {
    let mut history = vec![user_msg("hello")];
    update_history_with_clean_user_message(&mut history, "hello", Some(&[]));
    if let Message::User { content } = &history[0] {
        let text = match content.first_ref() {
            UserContent::Text(t) => t.text.as_str(),
            _ => panic!("expected text"),
        };
        assert_eq!(text, "hello");
    }
}

// ── prepare_prompt_with_attachments ─────────────────────────────

#[test]
fn prepare_prompt_no_attachments() {
    let result = prepare_prompt_with_attachments("hello", None);
    assert_eq!(result, "hello");
}

#[test]
fn prepare_prompt_with_attachment_paths() {
    let result = prepare_prompt_with_attachments(
        "hello",
        Some(&["/path/a.txt".to_string(), "/path/b.txt".to_string()]),
    );
    assert!(result.contains("[Attached Document: /path/a.txt]"));
    assert!(result.contains("[Attached Document: /path/b.txt]"));
    assert!(result.ends_with("hello"));
}
