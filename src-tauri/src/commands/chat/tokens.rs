use crate::domain::chat::TokenCountResponse;
use crate::domain::errors::AppError;
use agent_rs_lib::agent::memory::tokenizer;

/// Helper function to perform the actual token counting using `agent_rs_lib` BPE tokenizer.
///
/// > [!NOTE]
/// > **Approximation Notice:** This function uses the `cl100k_base` BPE tokenizer vocabulary (accurate for OpenAI models).
/// > For other providers like Anthropic and Gemini, this count serves as a close approximation.
pub fn calculate_tokens(prompt: &str, response: Option<&str>) -> (usize, usize) {
    let prompt_tokens = tokenizer::count_string_tokens(prompt);
    let response_tokens = response.map_or(0, tokenizer::count_string_tokens);
    (prompt_tokens, response_tokens)
}

/// Counts tokens in a prompt and an optional response using `agent_rs_lib` BPE tokenizer.
///
/// > [!NOTE]
/// > **Approximation Notice:** This command uses the OpenAI `cl100k_base` tokenizer vocabulary.
/// > Counts for Anthropic/Gemini responses are close approximations rather than precise provider billing values.
#[tauri::command]
pub fn count_tokens(
    prompt: String,
    response: Option<String>,
) -> Result<TokenCountResponse, AppError> {
    let (prompt_tokens, response_tokens) = calculate_tokens(&prompt, response.as_deref());
    Ok(TokenCountResponse {
        prompt_tokens,
        response_tokens,
        total_tokens: prompt_tokens + response_tokens,
    })
}
