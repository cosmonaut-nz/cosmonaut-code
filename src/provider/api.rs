//! This module provides a set of structures and traits to facilitate the interaction
//! with various language model providers like OpenAI, Google, etc. It includes data
//! structures for sending requests and receiving responses, and also converters to
//! translate between generic and provider-specific formats.

use openai_api_rs::v1::chat_completion::{
    ChatCompletionChoice, ChatCompletionMessage, ChatCompletionMessageForResponse,
    ChatCompletionResponse, MessageRole,
};
use serde::{Deserialize, Serialize};

// Enum to represent the role of a message in a conversation.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub(crate) enum ProviderMessageRole {
    User,
    System,
    Assistant,
    Function,
}

// Struct for messages sent to the language model provider.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct ProviderCompletionMessage {
    pub(crate) role: ProviderMessageRole,
    pub(crate) content: String,
}

// Struct for responses received from the language model provider.
#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct ProviderCompletionResponse {
    pub(crate) id: String,
    pub(crate) model: String,
    pub(crate) choices: Vec<ProviderResponseChoice>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct ProviderResponseChoice {
    pub(crate) message: ProviderResponseMessage,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct ProviderResponseMessage {
    pub(crate) content: String,
}

// Trait to convert generic messages to provider-specific messages.
pub(crate) trait ProviderMessageConverter {
    type ProviderOutputMessage;

    fn convert_message(&self, message: &ProviderCompletionMessage) -> Self::ProviderOutputMessage;
    fn convert_messages(
        &self,
        messages: &[ProviderCompletionMessage],
    ) -> Vec<Self::ProviderOutputMessage>;
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

// Similarly, create converters for Google, Anthropic, Meta, etc. here

// Trait for converting provider-specific responses to generic format.
pub(crate) trait ProviderResponseConverter {
    fn to_generic_provider_response(
        &self,
        response: &ChatCompletionResponse,
    ) -> ProviderCompletionResponse;
}

// Implementation of ProviderResponseConverter for OpenAI.
pub(crate) struct OpenAIResponseConverter;

impl ProviderResponseConverter for OpenAIResponseConverter {
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
    use super::*;
    use openai_api_rs::v1::common::Usage;

    #[test]
    fn test_provider_message_role_serialization() {
        let roles = vec![
            ProviderMessageRole::User,
            ProviderMessageRole::System,
            ProviderMessageRole::Assistant,
            ProviderMessageRole::Function,
        ];

        for role in roles {
            let serialized = serde_json::to_string(&role).unwrap();
            let deserialized: ProviderMessageRole = serde_json::from_str(&serialized).unwrap();
            assert_eq!(role, deserialized);
        }
    }

    #[test]
    fn test_provider_completion_message_serialization() {
        let message = ProviderCompletionMessage {
            role: ProviderMessageRole::User,
            content: "Test message".to_string(),
        };

        let serialized = serde_json::to_string(&message).unwrap();
        let deserialized: ProviderCompletionMessage = serde_json::from_str(&serialized).unwrap();
        assert_eq!(message.role, deserialized.role);
        assert_eq!(message.content, deserialized.content);
    }

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
