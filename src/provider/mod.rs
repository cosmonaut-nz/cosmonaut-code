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
pub(crate) async fn review_code_file(
    settings: &Settings,
    provider_settings: &ProviderSettings,
    prompt_data: PromptData,
) -> Result<ProviderCompletionResponse, Box<dyn std::error::Error>> {
    match create_api_provider(provider_settings) {
        Ok(provider_handler) => provider_handler.review_code(settings, prompt_data).await,
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
            model: provider_settings.model.clone(),
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
    async fn review_code(
        &self,
        settings: &Settings,
        prompt_data: PromptData,
    ) -> Result<ProviderCompletionResponse, Box<dyn std::error::Error>>;
}

/// Creates an API provider, e.g., 'OpenAI' (using the openai_api_rs crate)
struct OpenAIProvider {
    model: String,
}
#[async_trait::async_trait]
impl APIProvider for OpenAIProvider {
    async fn review_code(
        &self,
        settings: &Settings,
        prompt_data: PromptData,
    ) -> Result<ProviderCompletionResponse, Box<dyn std::error::Error>> {
        settings.sensitive.api_key.use_key(
            "from provider::OpenAIProvider::code_review for openai_api_rs::v1::api::Client::new",
            |key| {
                let client: Client = Client::new(key.to_string());
                let res_format = json!({ "type": "json_object" });
                let completion_msgs: Vec<ChatCompletionMessage> =
                    OpenAIMessageConverter.convert_messages(&prompt_data.messages);
                let req: ChatCompletionRequest =
                    ChatCompletionRequest::new(self.model.to_string(), completion_msgs).response_format(res_format);
                async move {
                    match client.chat_completion(req) {
                        Ok(openai_res) => {
                            let provider_completion_response: ProviderCompletionResponse =
                                OpenAIResponseConverter.to_generic_provider_response(&openai_res);
                            Ok(provider_completion_response)
                        }
                        Err(openai_err) => Err(format!("OpenAI API request failed: {}", openai_err).into()),
                    }
                }
            }
        ).await
    }
}
