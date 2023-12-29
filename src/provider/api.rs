//! This module provides a set of structures and traits to facilitate the interaction
//! with various language model providers like OpenAI, Google, etc. It includes data
//! structures for sending requests and receiving responses, and also converters to
//! translate between generic and provider-specific formats.

use serde::{Deserialize, Serialize};

// Enum to represent the role of a message in a conversation.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
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

// Trait for converting provider-specific responses to generic format.
pub(crate) trait ProviderResponseConverter<T> {
    fn new(model: String) -> Self;
    fn to_generic_provider_response(&self, response: &T) -> ProviderCompletionResponse;
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
