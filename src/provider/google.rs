pub(crate) mod gemini {
    use crate::provider::api::{ProviderCompletionResponse, ProviderResponseConverter};
    use crate::provider::prompts::PromptData;
    use crate::provider::{APIProvider, RequestType};
    use crate::settings::{ProviderSettings, Settings};

    use google_generative_ai_rs::v1::api::Client;
    use google_generative_ai_rs::v1::api::PostResult;
    use google_generative_ai_rs::v1::gemini::request::Request;
    use google_generative_ai_rs::v1::gemini::{Content, Part, Role};
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

            let client = Client::new(
                settings
                    .sensitive
                    .api_key
                    .as_ref()
                    .ok_or("No API Key set, please set to user provider service")?
                    .use_key(|key| key.to_owned()),
            );

            Ok(ask_request_of_gemini(&self.model.clone(), &client, provider, prompt_data).await?)
        }
    }

    pub(super) async fn ask_request_of_gemini(
        model: &str,
        client: &Client,
        provider: &ProviderSettings,
        prompt_data: &PromptData,
    ) -> Result<ProviderCompletionResponse, Box<dyn std::error::Error>> {
        let prompt_msgs: Vec<serde_json::Value> = prompt_data
            .messages
            .iter()
            .map(|message| json!([{"text": message.content}]))
            .collect();

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

        let converter = GeminiResponseConverter::new(model.to_string());

        Ok(converter.to_generic_provider_response(&post_result))
    }
}

pub(crate) mod vertex_ai {
    use crate::provider::api::ProviderCompletionResponse;
    use crate::provider::prompts::PromptData;
    use crate::provider::{APIProvider, RequestType};
    use crate::settings::{ProviderSettings, Settings};

    use google_generative_ai_rs::v1::api::Client;
    use google_generative_ai_rs::v1::errors::GoogleAPIError;

    use super::gemini::ask_request_of_gemini;
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

            let region = settings
                .sensitive
                .region
                .as_ref()
                .ok_or_else(|| GoogleAPIError {
                    message: format!("No provider region specified for {}", provider.name),
                    code: None,
                })?;
            let project_id =
                settings
                    .sensitive
                    .project_id
                    .as_ref()
                    .ok_or_else(|| GoogleAPIError {
                        message: format!("No provider project_id specified for {}", provider.name),
                        code: None,
                    })?;

            let client =
                Client::new_from_region_project_id(region.to_string(), project_id.to_string());

            Ok(ask_request_of_gemini(&self.model.clone(), &client, provider, prompt_data).await?)
        }
    }
    impl VertexAiProvider {}
}

/// The data structures for the Google API response
pub(super) mod data {
    use google_generative_ai_rs::v1::api::PostResult;

    use crate::provider::api::{
        ProviderCompletionResponse, ProviderResponseChoice, ProviderResponseConverter,
        ProviderResponseMessage,
    };
    // Implementation of ProviderResponseConverter for the Gemini FM.
    pub(crate) struct GeminiResponseConverter {
        pub(super) model: String,
    }
    impl ProviderResponseConverter<PostResult> for GeminiResponseConverter {
        fn new(model: String) -> Self {
            GeminiResponseConverter { model }
        }
        fn to_generic_provider_response(
            &self,
            google_response: &PostResult,
        ) -> ProviderCompletionResponse {
            let mut messages: Vec<ProviderResponseMessage> = vec![];
            match google_response {
                PostResult::Rest(response) => {
                    for candidate in &response.candidates {
                        for part in &candidate.content.parts {
                            messages.push(ProviderResponseMessage {
                                content: part.text.as_ref().unwrap().to_string(),
                            });
                        }
                    }
                }
                PostResult::Streamed(streamed_response) => {
                    for gemini_completion_response in &streamed_response.streamed_candidates {
                        for candidate in &gemini_completion_response.candidates {
                            for part in &candidate.content.parts {
                                messages.push(ProviderResponseMessage {
                                    content: part.text.as_ref().unwrap().to_string(),
                                });
                            }
                        }
                    }
                }
            }
            ProviderCompletionResponse {
                id: "".to_string(),
                model: self.model.clone(),
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
