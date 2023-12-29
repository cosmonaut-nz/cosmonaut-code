//! The provider specific work for Google Gemini service
//! Note: this does not cover usage of the private Gemini API, which is is provided by the the Vertex AI provider.

use crate::provider::api::ProviderCompletionResponse;
use crate::provider::api::ProviderResponseConverter;
use crate::provider::prompts::PromptData;
use crate::provider::{APIProvider, RequestType};
use crate::settings::{ProviderSettings, Settings};

use google_generative_ai_rs::v1::api::Client;
use google_generative_ai_rs::v1::api::PostResult;
use google_generative_ai_rs::v1::errors::GoogleAPIError;
use google_generative_ai_rs::v1::gemini::request::{Content, Part, Request};
use google_generative_ai_rs::v1::gemini::Role;
use serde_json::json;

use super::data::GeminiResponseConverter;

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

        let prompt_msgs: Vec<serde_json::Value> = prompt_data
            .messages
            .iter()
            .map(|message| json!([{"text": message.content}]))
            .collect();

        let client = Client::new(settings.sensitive.api_key.use_key(|key| key.to_owned()));

        let request = Request {
            contents: vec![Content {
                role: Role::User,
                parts: prompt_msgs
                    .iter()
                    .map(|message| Part {
                        text: Some(message.to_string()),
                        inline_data: None,
                        file_data: None,
                        video_metadata: None,
                    })
                    .collect(),
            }],
            tools: vec![],
            safety_settings: vec![],
            generation_config: None,
        };

        let post_result: PostResult = client
            .post(provider.api_timeout.unwrap_or(30), &request)
            .await?;

        let response = post_result.rest().ok_or_else(|| {
            Box::new(GoogleAPIError {
                message: "Unknown error from Google API response".to_string(),
                code: None,
            }) as Box<dyn std::error::Error>
        })?;

        let converter = GeminiResponseConverter::new(self.model.clone());

        Ok(converter.to_generic_provider_response(&response))
    }
}
