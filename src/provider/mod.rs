//!
//!  Handles the access to the LLM with utility functions for specified actions
//!
pub(crate) mod api;
pub(crate) mod prompts;
use crate::provider::api::{
    OpenAIMessageConverter, OpenAIResponseConverter, ProviderCompletionResponse,
    ProviderMessageConverter, ProviderResponseConverter,
};
use crate::provider::prompts::PromptData;
use crate::settings::{ProviderSettings, Settings};
use log::{info, warn};
use openai_api_rs::v1::api::Client;
use openai_api_rs::v1::chat_completion::{ChatCompletionMessage, ChatCompletionRequest};
use serde_json::json;

// Add similar structs and implementations for other providers.

/// Sends text contents to an LLM agent to evaluate according to the prompt passed to it.
///
/// # Parameters
///
/// * `settings` - A ['Config'] that contains information for the LLM
/// * `provider` - the configured provider that will handle the review request - e.g. 'openai', 'meta', etc.
/// * `prompt_data` - a preconfigured prompt text to ask the LLM to do a task, including the body of the output_file to be reviewed
///
/// # Returns
///
/// * A response from the LLM ['ProviderResponseMessage']
///
pub(crate) async fn review_or_summarise(
    request_type: RequestType,
    settings: &Settings,
    provider_settings: &ProviderSettings,
    prompt_data: &PromptData,
) -> Result<ProviderCompletionResponse, Box<dyn std::error::Error>> {
    match create_api_provider(provider_settings) {
        Ok(provider_handler) => {
            provider_handler
                .ask_request_of_provider(&request_type, settings, prompt_data)
                .await
        }
        Err(err) => Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("API provider error: {}", err),
        ))),
    }
}

/// Creates an APIProvider according to provider_settings.name
fn create_api_provider(
    provider_settings: &ProviderSettings,
) -> Result<Box<dyn APIProvider>, Box<dyn std::error::Error>> {
    match provider_settings.name.to_lowercase().as_str() {
        "openai" => Ok(Box::new(OpenAIProvider {
            model: provider_settings.get_active_service()?.model.to_string(),
        })),
        _ => Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Unsupported provider: {}", provider_settings.name),
        ))),
    }
}

/// An APIProvider trait allowing for multiple API providers to be implemented
/// A gamble on the future (geddit) of Rust here.
#[async_trait::async_trait]
trait APIProvider {
    async fn ask_request_of_provider(
        &self,
        request_type: &RequestType,
        settings: &Settings,
        prompt_data: &PromptData,
    ) -> Result<ProviderCompletionResponse, Box<dyn std::error::Error>>;
}

/// Holds a consistent 'seed' value
const SEED_VAL: i64 = 1;
pub(crate) enum RequestType {
    Review,
    Summarise,
}
/// Creates an API provider, e.g., 'OpenAI' (using the openai_api_rs crate)
struct OpenAIProvider {
    model: String,
}
#[async_trait::async_trait]
impl APIProvider for OpenAIProvider {
    async fn ask_request_of_provider(
        &self,
        request_type: &RequestType,
        settings: &Settings,
        prompt_data: &PromptData,
    ) -> Result<ProviderCompletionResponse, Box<dyn std::error::Error>> {
        let key = settings.sensitive.api_key.use_key(|key| key.to_string());

        let client: Client = Client::new(key);
        let completion_msgs = OpenAIMessageConverter.convert_messages(&prompt_data.messages);
        let req = self.build_chat_completion_request(request_type, completion_msgs);

        self.attempt_api_request(&client, &req, settings).await
    }
}

impl OpenAIProvider {
    fn build_chat_completion_request(
        &self,
        request_type: &RequestType,
        completion_msgs: Vec<ChatCompletionMessage>,
    ) -> ChatCompletionRequest {
        let mut request = ChatCompletionRequest::new(self.model.to_string(), completion_msgs);

        if self.model.contains("preview") || self.model.contains("turbo") {
            // Apply 'seed' for both 'Summarise' and 'Review'
            request = request.seed(SEED_VAL);

            // Apply 'response_format' only for 'Review'
            if let RequestType::Review = request_type {
                let res_format = json!({ "type": "json_object" });
                request = request.response_format(res_format);
            }
        }

        request
    }

    async fn attempt_api_request(
        &self,
        client: &Client,
        req: &ChatCompletionRequest,
        settings: &Settings,
    ) -> Result<ProviderCompletionResponse, Box<dyn std::error::Error>> {
        let max_retries = settings
            .get_active_provider()
            .map_or(0, |provider_settings| {
                provider_settings.max_retries.unwrap_or(0)
            });

        let mut attempts = 0;
        while attempts < max_retries {
            match client.chat_completion(req.clone()) {
                Ok(openai_res) => {
                    return Ok(OpenAIResponseConverter.to_generic_provider_response(&openai_res));
                }
                Err(openai_err) => {
                    attempts += 1;
                    if let Some(err_code) = extract_http_status(&openai_err.message) {
                        if err_code == 502 {
                            warn!(
                                "Received 502 error, retrying... (Attempt {} of {})",
                                attempts, max_retries
                            );
                            info!("Retrying request to OpenAI API.");
                            continue;
                        }
                    }
                    return Err(format!("OpenAI API request failed: {}", openai_err).into());
                }
            }
        }
        Err(format!("OpenAI API request failed after {} attempts", max_retries).into())
    }
}

fn extract_http_status(error_message: &str) -> Option<u16> {
    if error_message.contains("502") {
        return Some(502);
    }
    None
}
