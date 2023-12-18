//!
use super::api::{
    ProviderCompletionResponse, ProviderResponseChoice, ProviderResponseConverter,
    ProviderResponseMessage,
};
use super::{APIProvider, RequestType};
use crate::provider::prompts::PromptData;
use crate::settings::{ProviderSettings, Settings};
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;

pub(super) struct LMStudioProvider {}

#[async_trait::async_trait]
impl APIProvider for LMStudioProvider {
    async fn ask_request_of_provider(
        &self,
        _request_type: &RequestType,
        settings: &Settings,
        prompt_data: &PromptData,
    ) -> Result<ProviderCompletionResponse, Box<dyn std::error::Error>> {
        let provider: &ProviderSettings = settings.get_active_provider()?;

        // TODO set a timeout on the client
        // let client: Client = Client::builder().timeout(settings).build()?;
        let client: Client = Client::builder().build()?;

        let response: Result<reqwest::Response, reqwest::Error> = client
            .post(provider.api_url.clone())
            .header("Content-Type", "application/json")
            .json(&json!({
                "messages": prompt_data.messages,
                "max_tokens": -1,
                "temperature": 0.7,
                "stream": false,
            }))
            .send()
            .await;
        match response {
            Ok(res) => {
                if res.status().is_success() {
                    match res.json::<LMStudioCompletionResponse>().await {
                        Ok(data) => {
                            Ok(LMStudioResponseConverter.to_generic_provider_response(&data))
                        }
                        Err(e) => Err(format!("Failed to deserialize response: {}", e).into()),
                    }
                } else {
                    Err(format!("Server returned error: {}", res.status()).into())
                }
            }
            Err(e) => {
                if e.is_timeout() {
                    Err("Network request timed out".into())
                } else if e.is_status() {
                    Err(format!("Server returned error: {}", e.status().unwrap()).into())
                } else {
                    Err(format!("Network request failed: {}", e).into())
                }
            }
        }
    }
}
#[derive(Debug, Deserialize)]
pub struct LMStudioCompletionResponse {
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
// Implementation of ProviderResponseConverter for LM Studio.
pub(crate) struct LMStudioResponseConverter;

impl ProviderResponseConverter<LMStudioCompletionResponse> for LMStudioResponseConverter {
    fn to_generic_provider_response(
        &self,
        response: &LMStudioCompletionResponse,
    ) -> ProviderCompletionResponse {
        ProviderCompletionResponse {
            id: String::new(),
            model: String::new(),
            choices: response
                .choices
                .iter()
                .map(convert_chat_choice_to_provider_choice)
                .collect(),
        }
    }
}
fn convert_chat_choice_to_provider_choice(chat_choice: &Choice) -> ProviderResponseChoice {
    ProviderResponseChoice {
        message: convert_chat_message_to_provider_message(&chat_choice.message),
    }
}
fn convert_chat_message_to_provider_message(chat_message: &Message) -> ProviderResponseMessage {
    ProviderResponseMessage {
        content: chat_message.content.clone(),
    }
}
