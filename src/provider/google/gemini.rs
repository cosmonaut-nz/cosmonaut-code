//! The provider specific work for Google Gemini service
//! Note: this does not cover usage of the private Gemini API, which is is provided by the the Vertex AI provider.
use super::data::{GeminiResponse, GeminiResponseConverter};
use crate::provider::api::{ProviderCompletionResponse, ProviderResponseConverter};
use crate::provider::prompts::PromptData;
use crate::provider::{APIProvider, RequestType};
use crate::settings::{ProviderSettings, Settings};

use log::info;
use reqwest::Client;
use serde_json::json;
use std::time::Duration;
use url::Url;

/// The Google Gemini public API provider works on the the following URL structure:
/// - The API URL base - 'https://generativelanguage.googleapis.com/v1'
/// - The 'models' extension - '/models', i.e., 'gemini-pro'
/// - action: 'generateContent'
/// - example: 'https://generativelanguage.googleapis.com/v1/models/gemini-pro:generateContent'
///
/// For public API use, the API key is required.
pub(crate) struct GeminiProvider {
    pub(crate) model: String,
}

#[async_trait::async_trait]
impl APIProvider for GeminiProvider {
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

        let api_url = self.get_api_url(settings, provider)?;

        let prompt_msgs: Vec<serde_json::Value> = prompt_data
            .messages
            .iter()
            .map(|message| json!([{"text": message.content}]))
            .collect();

        let response: Result<reqwest::Response, reqwest::Error> = client
            .post(&api_url)
            .header(reqwest::header::USER_AGENT, env!("CARGO_CRATE_NAME"))
            .header("Content-Type", "application/json")
            .json(&json!({
                "contents": {"role": "user", "parts": prompt_msgs},
            }))
            .send()
            .await;

        match response {
            Ok(res) => match res.status() {
                reqwest::StatusCode::OK => match res.json::<GeminiResponse>().await {
                    Ok(mut data) => {
                        data.model = Some(self.model.clone());
                        Ok(GeminiResponseConverter.to_generic_provider_response(&data))
                    }
                    Err(e) => Err(format!(
                        "Failed to deserialize Google response into GoogleCompletionResponse: {}",
                        e
                    )
                    .into()),
                },
                reqwest::StatusCode::UNAUTHORIZED => {
                    return Err(format!("Authorization error. Code: {:?}", res.status()).into());
                }
                reqwest::StatusCode::BAD_REQUEST => {
                    return Err(format!(
                        "API request format not correctly formed. Code: {:?}",
                        res.status()
                    )
                    .into());
                }
                reqwest::StatusCode::FORBIDDEN => {
                    let status = res.status();
                    info!("Forbidden. Check API permissions. {:#?}", res.text().await);
                    return Err(
                        format!("Forbidden. Check API permissions. Code: {:?}", status).into(),
                    );
                }
                _ => {
                    return Err(
                        format!("An unexpected HTTP error code: . {:?}", res.status()).into(),
                    );
                }
            },
            Err(e) => {
                if e.is_timeout() {
                    Err("Google API server timed out".into())
                } else if e.is_status() {
                    Err(format!("Google API server returned error: {}", e.status().unwrap()).into())
                } else {
                    Err(format!("Google API request failed: {}", e).into())
                }
            }
        }
    }
}
impl GeminiProvider {
    /// creates a valid URL for this provider
    /// provider.api_url = https://generativelanguage.googleapis.com/v1/models/{model}:generateContent?key={key}
    fn get_api_url(
        &self,
        settings: &Settings,
        provider: &ProviderSettings,
    ) -> Result<String, Box<dyn std::error::Error>> {
        // Replace '{model}' placeholder directly in the URL string
        let url_with_model = provider.api_url.replace("{model}", &self.model);

        // Parse the URL after replacing the model
        let mut api_url = Url::parse(&url_with_model)?;

        // Append the API key as a query parameter
        api_url.query_pairs_mut().append_pair(
            "key",
            settings
                .sensitive
                .api_key
                .use_key(|key| key.to_owned())
                .as_str(),
        );

        Ok(api_url.to_string())
    }
}
