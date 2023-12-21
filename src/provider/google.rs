//! The provider specific work for Google
//!
use self::data::{GoogleCompletionResponse, GoogleResponseConverter};
use super::api::{ProviderCompletionResponse, ProviderResponseConverter};
use super::{APIProvider, RequestType};
use crate::provider::prompts::PromptData;
use crate::settings::{ProviderSettings, Settings};

// use gcp_auth::AuthenticationManager; // Only used for private API
use log::info;
use reqwest::Client;
use serde_json::json;
use std::time::Duration;

/// The Google public API provider works on the the following URL structure:
/// - The API URL base - 'https://generativelanguage.googleapis.com/v1'
/// - The 'models' extension - '/models', i.e., 'gemini-pro'
/// - action: 'generateContent'
/// - example: 'https://generativelanguage.googleapis.com/v1/models/gemini-pro:generateContent'
///
/// The Google private API provider works on the the following URL structure:
/// - The API URL base - 'https://us-central1-aiplatform.googleapis.com/v1'
/// - The 'projects' extension - '/projects', i.e., '[YOUR_PROJECT_ID]'
/// - The 'locations' extension - '/locations', i.e., 'us-central1'
/// - The 'publishers' extension - '/publishers', i.e., 'google'
/// - The 'models' extension - '/models', i.e., 'gemini-pro'
/// - action: 'streamGenerateContent'
/// - example: 'https://us-central1-aiplatform.googleapis.com/v1/projects/[YOUR_PROJECT_ID]/locations/us-central1/publishers/google/models/gemini-pro:streamGenerateContent'
///
/// For public API use, the API key is required. For private API use, application default credentials (ADC) are required.
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

        // set up GCP application default credentials (ADC) authn
        // Uses URL: 'https://us-central1-aiplatform.googleapis.com/v1/projects/${PROJECT_ID}/locations/us-central1/publishers/google/models/${MODEL_ID}:streamGenerateContent'
        // let authentication_manager = AuthenticationManager::new().await?;
        // let scopes = &["https://www.googleapis.com/auth/cloud-platform"];
        // let token = authentication_manager.get_token(scopes).await?;

        let client: Client = Client::builder()
            .timeout(Duration::from_secs(provider.api_timeout.unwrap_or(30)))
            .build()?;

        // Extract out into a separate crate for the API handling, or if one arises, use that
        let google_url = format!(
            "{}/models/{}:generateContent?key={}",
            provider.api_url.clone(),
            self.model.clone(),
            settings.sensitive.api_key.use_key(|key| key.to_string())
        );
        // If this is a 'private' model, then we need to use the 'publishers' URL
        // Note: the 'publishers' model does not provide the same response as the 'public' URL
        // let google_url =
        // format!("https://us-central1-aiplatform.googleapis.com/v1/projects/{}/locations/us-central1/publishers/google/models/{}:streamGenerateContent",
        // "mickclarke138",
        // self.model.clone());

        let prompt_msgs: Vec<serde_json::Value> = prompt_data
            .messages
            .iter()
            .map(|message| json!([{"text": message.content}]))
            .collect();

        let response: Result<reqwest::Response, reqwest::Error> = client
            .post(&google_url)
            .header(reqwest::header::USER_AGENT, env!("CARGO_CRATE_NAME"))
            // .bearer_auth(&token.as_str().to_string()) // for private API only
            .header("Content-Type", "application/json")
            .json(&json!({
                "contents": {"role": "user", "parts": prompt_msgs},
            }))
            .send()
            .await;

        match response {
            Ok(res) => match res.status() {
                reqwest::StatusCode::OK => match res.json::<GoogleCompletionResponse>().await {
                    Ok(mut data) => {
                        data.model = Some(self.model.clone());
                        Ok(GoogleResponseConverter.to_generic_provider_response(&data))
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

/// The data structures for the Google API response
mod data {
    use serde::Deserialize;

    use crate::provider::api::{
        ProviderCompletionResponse, ProviderResponseChoice, ProviderResponseConverter,
        ProviderResponseMessage,
    };

    // The structs that represent the Google API response
    /// Top-level response from Google
    #[derive(Debug, Deserialize)]
    pub struct GoogleCompletionResponse {
        pub model: Option<String>,
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

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_to_generic_provider_response() {
            let google_response = GoogleCompletionResponse {
                model: Some("gemini-pro".to_string()),
                candidates: vec![Candidate {
                    content: Content {
                        parts: vec![
                            Part {
                                text: "Hello".to_string(),
                            },
                            Part {
                                text: "World".to_string(),
                            },
                        ],
                        role: GoogleRole::User,
                    },
                    finish_reason: "complete".to_string(),
                    index: 0,
                    safety_ratings: vec![SafetyRating {
                        category: "safety".to_string(),
                        probability: "high".to_string(),
                    }],
                }],
                prompt_feedback: PromptFeedback {
                    safety_ratings: vec![SafetyRating {
                        category: "safety".to_string(),
                        probability: "high".to_string(),
                    }],
                },
            };

            let converter = GoogleResponseConverter;
            let provider_response = converter.to_generic_provider_response(&google_response);

            assert_eq!(provider_response.choices.len(), 1);
            assert_eq!(provider_response.choices[0].message.content, "Hello\nWorld");
        }
    }
}
