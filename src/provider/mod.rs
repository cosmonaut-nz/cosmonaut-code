//!
//!  Handles the access to the LLM with utility functions for specified actions
//!
// TODO: Heavy refactor. Move to the 'openai-api-rs' crate
//       ```
//          [dependencies]
//              openai-api-rs = "2.1.0"
//       ```
//
use crate::chat_prompts::PromptData;
use crate::settings::{Provider, Settings};
use log::debug;
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use std::time::Duration;

#[derive(Debug, Deserialize)]
pub struct ModelResponse {
    pub choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
pub struct Choice {
    pub message: Message,
}

#[derive(Debug, Deserialize)]
pub struct Message {
    pub content: String,
}

/// Sends text contents to an LLM agent to evaluate according to the prompt passed to it.
///
/// # Parameters
///
/// * `config` - A ['Config'] that contains information for the LLM
/// * `prompt` - a preconfigured prompt text to ask the LLM to do a task
/// * `contents` - the body of the output_file to be reviewed
///
/// # Returns
///
/// * A response from the LLM ['ModelResponse']
///
pub async fn review_code_file(
    settings: &Settings,
    provider: &Provider,
    prompt_data: PromptData,
) -> Result<ModelResponse, Box<dyn std::error::Error>> {
    // We set a timeout on the client
    let client: Client = Client::builder()
        .timeout(Duration::from_secs(provider.api_timeout))
        .build()?;
    let response: Result<reqwest::Response, reqwest::Error> = client
        .post(&provider.api_url)
        .header(
            "Authorization",
            format!("Bearer {}", settings.sensitive.api_key.get()),
        )
        .header("Content-Type", "application/json")
        .json(&json!({
            "model": provider.model,
            "messages": prompt_data.messages,
            "max_tokens": provider.max_tokens,
            "n": 1,
            "stop": null
        }))
        .send()
        .await;
    match response {
        Ok(res) => {
            debug!("Response status: {}", res.status());
            if res.status().is_success() {
                match res.json::<ModelResponse>().await {
                    Ok(data) => Ok(data),
                    Err(e) => Err(format!("Failed to deserialize response: {}", e).into()),
                }
            } else {
                Err(format!("Server returned error: {}", res.status()).into())
            }
        }
        Err(e) => {
            if e.is_timeout() {
                Err(format!(
                    "Network request timed out after {} seconds",
                    provider.api_timeout
                )
                .into())
            } else if e.is_status() {
                Err(format!("Server returned error: {}", e.status().unwrap()).into())
            } else {
                Err(format!("Network request failed: {}", e).into())
            }
        }
    }
}
