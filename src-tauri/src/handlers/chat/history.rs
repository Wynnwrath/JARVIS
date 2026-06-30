use rig_core::message::{AssistantContent, Message, UserContent};

pub fn assistant_message_text(
    content: &rig_core::OneOrMany<rig_core::message::AssistantContent>,
) -> String {
    content
        .iter()
        .filter_map(|item| match item {
            AssistantContent::Text(t) => Some(t.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn deduplicate_consecutive_assistant_messages(history: Vec<Message>) -> Vec<Message> {
    let mut result: Vec<Message> = Vec::new();
    for msg in history {
        if let Message::Assistant { content, .. } = &msg {
            if let Some(Message::Assistant {
                content: prev_content,
                ..
            }) = result.last()
            {
                let prev_text = assistant_message_text(prev_content);
                let curr_text = assistant_message_text(content);
                if prev_text == curr_text {
                    let prev_has_tools = prev_content
                        .iter()
                        .any(|item| matches!(item, AssistantContent::ToolCall(_)));
                    let curr_has_tools = content
                        .iter()
                        .any(|item| matches!(item, AssistantContent::ToolCall(_)));
                    if curr_has_tools != prev_has_tools {
                        if curr_has_tools {
                            result.pop();
                            result.push(msg);
                        }
                    } else if prev_has_tools && curr_has_tools {
                        result.push(msg);
                    }
                    continue;
                }
            }
        }
        result.push(msg);
    }
    result
}

pub fn prepare_prompt_with_attachments(input: &str, attachments: Option<&[String]>) -> String {
    let mut prompt = String::new();
    if let Some(paths) = attachments {
        for path in paths {
            prompt.push_str(&format!(
                "[Attached Document: {}]\nUse the 'read_document' tool to read this file if you need to access its contents.\n\n",
                path
            ));
        }
    }
    prompt.push_str(input);
    prompt
}

pub fn update_history_with_clean_user_message(
    history: &mut [Message],
    input: &str,
    attachments: Option<&[String]>,
) {
    let Some(paths) = attachments else {
        return;
    };
    if paths.is_empty() {
        return;
    }

    for msg in history.iter_mut().rev() {
        if let Message::User { content } = msg {
            let is_attachment_msg = match content.first_ref() {
                UserContent::Text(text_content) => {
                    text_content.text.starts_with("[Attached Document:")
                }
                _ => false,
            };

            if is_attachment_msg {
                let mut clean_text = String::new();
                for path in paths {
                    clean_text.push_str(&format!("[Attached: {}]\n", path));
                }
                clean_text.push_str(input);
                *msg = Message::user(&clean_text);
                break;
            }
        }
    }
}
