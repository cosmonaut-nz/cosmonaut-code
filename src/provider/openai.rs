//! OpenAI provider specific implementations and handling.
use super::{
    api::{
        ProviderCompletionMessage, ProviderMessageRole, ProviderResponseChoice,
        ProviderResponseMessage,
    },
    APIProvider, RequestType,
};
use crate::provider::prompts::PromptData;
use crate::provider::{
    api::{ProviderCompletionResponse, ProviderMessageConverter, ProviderResponseConverter},
    extract_http_status, HttpErrorCode,
};
use crate::settings::Settings;
use log::{info, warn};
use openai_api_rs::v1::{
    api::Client,
    chat_completion::{
        ChatCompletionChoice, ChatCompletionMessage, ChatCompletionMessageForResponse,
        ChatCompletionRequest, ChatCompletionResponse, MessageRole,
    },
};
use serde_json::json;

/// Holds a consistent 'seed' value, see https://cookbook.openai.com/examples/deterministic_outputs_with_the_seed_parameter
const SEED_VAL: i64 = 1234;

/// Creates an OpenAI API provider, uses the openai_api_rs crate
pub(super) struct OpenAIProvider {
    pub(super) model: String,
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
        client: &openai_api_rs::v1::api::Client,
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
                        if err_code == HttpErrorCode::BadGateway as u16 {
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

// Implementation of ProviderMessageConverter for OpenAI.
pub(crate) struct OpenAIMessageConverter;

impl ProviderMessageConverter for OpenAIMessageConverter {
    type ProviderOutputMessage = ChatCompletionMessage;

    fn convert_message(&self, message: &ProviderCompletionMessage) -> Self::ProviderOutputMessage {
        let role: MessageRole = match message.role {
            ProviderMessageRole::User => MessageRole::user,
            ProviderMessageRole::System => MessageRole::system,
            ProviderMessageRole::Assistant => MessageRole::assistant,
            ProviderMessageRole::Function => MessageRole::function,
        };

        ChatCompletionMessage {
            role,
            content: message.content.clone(),
            name: None,
            function_call: None,
        }
    }

    fn convert_messages(
        &self,
        messages: &[ProviderCompletionMessage],
    ) -> Vec<Self::ProviderOutputMessage> {
        messages
            .iter()
            .map(|message| self.convert_message(message))
            .collect()
    }
}

// Implementation of ProviderResponseConverter for OpenAI.
pub(crate) struct OpenAIResponseConverter;

impl ProviderResponseConverter<ChatCompletionResponse> for OpenAIResponseConverter {
    fn new(_model: String) -> Self {
        OpenAIResponseConverter
    }
    fn to_generic_provider_response(
        &self,
        response: &ChatCompletionResponse,
    ) -> ProviderCompletionResponse {
        ProviderCompletionResponse {
            id: response.id.clone(),
            model: response.model.clone(),
            choices: response
                .choices
                .iter()
                .map(convert_chat_choice_to_provider_choice)
                .collect(),
        }
    }
}

fn convert_chat_message_to_provider_message(
    chat_message: &ChatCompletionMessageForResponse,
) -> ProviderResponseMessage {
    ProviderResponseMessage {
        content: chat_message.content.clone().unwrap_or_default(),
    }
}

fn convert_chat_choice_to_provider_choice(
    chat_choice: &ChatCompletionChoice,
) -> ProviderResponseChoice {
    ProviderResponseChoice {
        message: convert_chat_message_to_provider_message(&chat_choice.message),
    }
}

#[cfg(test)]
mod tests {
    use crate::provider::api::{ProviderCompletionMessage, ProviderMessageRole};

    use super::*;
    use openai_api_rs::v1::{
        chat_completion::{
            ChatCompletionChoice, ChatCompletionMessageForResponse, ChatCompletionResponse,
            MessageRole,
        },
        common::Usage,
    };
    #[test]
    fn test_openai_message_converter() {
        let converter = OpenAIMessageConverter;
        let message = ProviderCompletionMessage {
            role: ProviderMessageRole::User,
            content: "Test message".to_string(),
        };

        let converted_message = converter.convert_message(&message);
        assert_eq!(converted_message.content, message.content);
    }
    #[test]
    fn test_openai_response_converter() {
        let usage = Usage {
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
        };
        let converter = OpenAIResponseConverter;
        let response = ChatCompletionResponse {
            id: "test_id".to_string(),
            model: "test_model".to_string(),
            object: "Some obj".to_string(),
            created: 0,
            usage,
            choices: vec![ChatCompletionChoice {
                index: 0,
                finish_reason: None,
                finish_details: None,
                message: ChatCompletionMessageForResponse {
                    name: None,
                    content: Some("Test content".to_string()),
                    role: MessageRole::user,
                    function_call: None,
                },
            }],
        };

        let converted_response = converter.to_generic_provider_response(&response);
        assert_eq!(converted_response.id, response.id);
        assert_eq!(converted_response.model, response.model);
        assert_eq!(
            converted_response.choices[0].message.content,
            "Test content"
        );
    }
}
