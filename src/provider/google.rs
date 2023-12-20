//! The provider specific work for Google
//!
use super::api::{
    ProviderCompletionResponse, ProviderResponseChoice, ProviderResponseConverter,
    ProviderResponseMessage,
};
use super::{APIProvider, RequestType};
use crate::provider::prompts::PromptData;
use crate::settings::{ProviderSettings, Settings};

use std::time::Duration;

use reqwest::Client;
use serde::Deserialize;
use serde_json::json;

/// What are the essential parts of a Google API request?
/// - The API URL base - 'https://generativelanguage.googleapis.com/v1'
/// - The 'models' extension - '/models', i.e., 'gemini-pro'
/// - action: 'generateContent'
/// - overall: 'https://generativelanguage.googleapis.com/v1/models/gemini-pro:generateContent'
pub(super) struct GoogleProvider {
    pub(super) model: String,
}

#[async_trait::async_trait]
impl APIProvider for GoogleProvider {
    async fn ask_request_of_provider(
        &self,
        _request_type: &RequestType,
        settings: &Settings,
        prompt_data: &PromptData,
    ) -> Result<ProviderCompletionResponse, Box<dyn std::error::Error>> {
        let provider: &ProviderSettings = settings.get_active_provider()?;

        let client: Client = Client::builder()
            .timeout(Duration::from_secs(provider.api_timeout.unwrap_or(30)))
            .build()?;

        let key = settings.sensitive.api_key.use_key(|key| key.to_string());

        // TODO fix the API handling
        let google_url = format!(
            "{}/models/{}:generateContent?key={}",
            provider.api_url.clone(),
            self.model.clone(),
            key
        );

        let prompt_msgs: Vec<serde_json::Value> = prompt_data
            .messages
            .iter()
            .map(|message| json!([{"text": message.content}]))
            .collect();

        let response: Result<reqwest::Response, reqwest::Error> = client
            .post(&google_url)
            .header(reqwest::header::USER_AGENT, env!("CARGO_CRATE_NAME"))
            .header("Content-Type", "application/json")
            .json(&json!({
                "contents": {"role": "user", "parts": prompt_msgs},
            }))
            .send()
            .await;

        match response {
            Ok(res) => match res.status() {
                reqwest::StatusCode::OK => match res.json::<GoogleCompletionResponse>().await {
                    Ok(data) => Ok(GoogleResponseConverter.to_generic_provider_response(&data)),
                    Err(e) => Err(format!("Failed to deserialize response: {}", e).into()),
                },
                reqwest::StatusCode::UNAUTHORIZED => {
                    return Err(format!("Authorization error. Code: {:?}", res.status()).into());
                }
                reqwest::StatusCode::BAD_REQUEST => {
                    return Err(
                        format!("Request not formed correctly. Code: {:?}", res.status()).into(),
                    );
                }
                reqwest::StatusCode::FORBIDDEN => {
                    return Err(format!(
                        "Forbidden. Check API permissions. Code: {:?}",
                        res.status()
                    )
                    .into());
                }
                _ => {
                    return Err(
                        format!("An unexpected HTTP error code: . {:?}", res.status()).into(),
                    );
                }
            },
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

// The structs that represent the Google API response
/// Top-level response from Google
#[derive(Debug, Deserialize)]
pub struct GoogleCompletionResponse {
    pub candidates: Vec<Candidate>,
    #[serde(rename = "promptFeedback")]
    pub prompt_feedback: PromptFeedback,
}
/// Google has the concept of a candidate. Similar to an OpenAI 'choice', but at the top-level
#[derive(Debug, Deserialize)]
pub struct Candidate {
    pub content: Content,
    #[serde(rename = "finishReason")]
    pub finish_reason: String,
    pub index: i32,
    #[serde(rename = "safetyRatings")]
    pub safety_ratings: Vec<SafetyRating>,
}
/// The Content for a Candidate is further broken down into Parts
#[derive(Debug, Deserialize)]
pub struct Content {
    pub parts: Vec<Part>,
    pub role: GoogleRole,
}
#[derive(Debug, Deserialize)]
pub struct Part {
    pub text: String,
}
/// Feedback on the prompt
#[derive(Debug, Deserialize)]
pub struct PromptFeedback {
    #[serde(rename = "safetyRatings")]
    pub safety_ratings: Vec<SafetyRating>,
}

/// The Google safety rating
#[derive(Debug, Deserialize)]
pub struct SafetyRating {
    pub category: String,
    pub probability: String,
}
/// A Google role is only ever 'user' or 'agent'
#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GoogleRole {
    User,
    Model,
}
// Implementation of ProviderResponseConverter for LM Studio.
pub(crate) struct GoogleResponseConverter;
impl ProviderResponseConverter<GoogleCompletionResponse> for GoogleResponseConverter {
    fn to_generic_provider_response(
        &self,
        google_response: &GoogleCompletionResponse,
    ) -> ProviderCompletionResponse {
        let mut messages: Vec<ProviderResponseMessage> = vec![];
        for candidate in &google_response.candidates {
            for part in &candidate.content.parts {
                messages.push(ProviderResponseMessage {
                    content: part.text.to_string(),
                });
            }
        }
        ProviderCompletionResponse {
            id: "".to_string(),
            model: "".to_string(),
            choices: vec![ProviderResponseChoice {
                message: ProviderResponseMessage {
                    content: messages
                        .iter()
                        .map(|m| m.content.clone())
                        .collect::<Vec<String>>()
                        .join("\n"),
                },
            }],
        }
    }
}
