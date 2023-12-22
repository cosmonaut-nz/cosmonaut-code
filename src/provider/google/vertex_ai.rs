//! The provider specific work for Google
//!
use self::data::GeminiStreamedResponse;
use crate::provider::api::{ProviderCompletionResponse, ProviderResponseConverter};
use crate::provider::google::vertex_ai::data::GeminiStreamedResponseConverter;
use crate::provider::prompts::PromptData;
use crate::provider::{APIProvider, RequestType};
use crate::settings::{ProviderSettings, Settings};

use gcp_auth::AuthenticationManager;
use log::info;
use reqwest::Client;
use serde_json::json;
use std::time::Duration;
use url::Url;

/// The Google private API provider works on the the following URL structure:
/// - The API URL base - 'https://{region}-aiplatform.googleapis.com/v1'
/// - The 'projects' extension - '/projects', i.e., '[YOUR_PROJECT_ID]'
/// - The 'locations' extension - '/locations', i.e., 'us-central1'
/// - The 'publishers' extension - '/publishers', i.e., 'google'
/// - The 'models' extension - '/models', i.e., 'gemini-pro'
/// - action: 'streamGenerateContent', this is key, as it varies from the public API in that it streams the reponse as it generates it
/// - example: 'https://{region}-aiplatform.googleapis.com/v1/projects/{project_id}/locations/{region}/publishers/google/models/{model}:streamGenerateContent'
///
/// For private API use, application default credentials (ADC) are required. See [`gcp_auth::AuthenticationManager`] for more details.
pub(crate) struct VertexAiProvider {
    pub(crate) model: String,
}

#[async_trait::async_trait]
impl APIProvider for VertexAiProvider {
    async fn ask_request_of_provider(
        &self,
        _request_type: &RequestType,
        settings: &Settings,
        prompt_data: &PromptData,
    ) -> Result<ProviderCompletionResponse, Box<dyn std::error::Error>> {
        let provider: &ProviderSettings = settings.get_active_provider()?;

        // set up GCP application default credentials (ADC) authn
        let authentication_manager = AuthenticationManager::new().await?;
        let scopes = &["https://www.googleapis.com/auth/cloud-platform"];
        let token = authentication_manager.get_token(scopes).await?;

        let client: Client = Client::builder()
            .timeout(Duration::from_secs(provider.api_timeout.unwrap_or(30)))
            .build()?;

        let url = self.get_api_url(settings, provider)?;

        let prompt_msgs: Vec<serde_json::Value> = prompt_data
            .messages
            .iter()
            .map(|message| json!([{"text": message.content}]))
            .collect();

        let response: Result<reqwest::Response, reqwest::Error> = client
            .post(&url)
            .header(reqwest::header::USER_AGENT, env!("CARGO_CRATE_NAME"))
            .bearer_auth(&token.as_str().to_string()) // for private API only
            .header("Content-Type", "application/json")
            .json(&json!({
                "contents": {"role": "user", "parts": prompt_msgs},
            }))
            .send()
            .await;

        // TODO work out the format of the reponse
        match response {
            Ok(res) => match res.status() {
                reqwest::StatusCode::OK => {
                    // Going to shift the code to handle in chunks. These 'should' match to a 'GeminiResponse' struct.
                    // As each is successfully deserialized, we'll add it to the GeminiStreamedResponse struct
                    let streamed_response: GeminiStreamedResponse = GeminiStreamedResponse {
                        model: Some(self.model.clone()),
                        streamed_candidates: vec![],
                        usage_metadata: None,
                    };
                    // TODO work out how to handle the streamed response
                    // let mut stream = res.bytes_stream();

                    Ok(GeminiStreamedResponseConverter
                        .to_generic_provider_response(&streamed_response))
                }
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
                    return Err(format!("An unexpected HTTP error code: {:?}", res.status()).into());
                }
            },
            Err(e) => {
                if e.is_timeout() {
                    Err("VertexAI API server timed out".into())
                } else if e.is_status() {
                    Err(format!(
                        "VertexAI API server returned error: {}",
                        e.status().unwrap()
                    )
                    .into())
                } else {
                    Err(format!("VertexAI API request failed: {}", e).into())
                }
            }
        }
    }
}
impl VertexAiProvider {
    /// creates a valid URL for this provider
    /// https://{region}-aiplatform.googleapis.com/v1/projects/{project_id}/locations/{region}/publishers/google/models/{model}:streamGenerateContent
    fn get_api_url(
        &self,
        _settings: &Settings,
        provider: &ProviderSettings,
    ) -> Result<String, Box<dyn std::error::Error>> {
        //
        let region = "us-central1";
        let project_id = "mickclarke138";

        // Replace '{region}' && '{project_id}' && '{model}' placeholders in the URL string
        // TODO work out how to get the region and project_id from the settings
        let url_updated = provider
            .api_url
            .replace("{model}", &self.model)
            .replace("{region}", region)
            .replace("{project_id}", project_id);

        // Parse the URL after replacing the model
        let api_url = Url::parse(&url_updated)?;

        Ok(api_url.to_string())
    }
}

/// The data structures for the Google API response
pub(super) mod data {
    use serde::Deserialize;

    use crate::provider::{
        api::{
            ProviderCompletionResponse, ProviderResponseChoice, ProviderResponseConverter,
            ProviderResponseMessage,
        },
        google::data::GeminiResponse,
    };

    /// The streamGenerateContent response
    #[derive(Debug, Deserialize)]
    pub struct GeminiStreamedResponse {
        pub model: Option<String>,
        #[serde(rename = "candidates")]
        pub streamed_candidates: Vec<GeminiResponse>,
        #[serde(rename = "usageMetadata")]
        pub usage_metadata: Option<UsageMetadata>,
    }

    #[derive(Debug, Deserialize)]
    pub struct UsageMetadata {
        #[serde(rename = "candidatesTokenCount")]
        pub candidates_token_count: u64,
        #[serde(rename = "promptTokenCount")]
        pub prompt_token_count: u64,
        #[serde(rename = "totalTokenCount")]
        pub total_token_count: u64,
    }

    pub(crate) struct GeminiStreamedResponseConverter;
    impl ProviderResponseConverter<GeminiStreamedResponse> for GeminiStreamedResponseConverter {
        fn to_generic_provider_response(
            &self,
            google_response: &GeminiStreamedResponse,
        ) -> ProviderCompletionResponse {
            // Iterate through the streamed_candidates to get the inner candidates
            let gemini_candidates = &google_response.streamed_candidates;

            let mut messages: Vec<ProviderResponseMessage> = vec![];
            for gemini_completion_response in gemini_candidates {
                for candidate in &gemini_completion_response.candidates {
                    for part in &candidate.content.parts {
                        messages.push(ProviderResponseMessage {
                            content: part.text.to_string(),
                        });
                    }
                }
            }

            ProviderCompletionResponse {
                id: "".to_string(),
                model: google_response
                    .model
                    .clone()
                    .unwrap_or("none set".to_string()),
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
}
