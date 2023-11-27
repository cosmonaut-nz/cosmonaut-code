//! Provides a suite of structs, traits and functions to marshall provider-specific API needs
//!
use openai_api_rs::v1::chat_completion::{
    ChatCompletionChoice, ChatCompletionMessage, ChatCompletionMessageForResponse,
    ChatCompletionResponse, MessageRole,
};
use serde::{Deserialize, Serialize};

// TODO: This module needs a retrospective refactor once more than one provider is wired in.
//          Right now the generalisation is based on the OpenAI and Claud 2 API structures (which are very similar)
//          Things to do:
//             - naming conventions are a messy and need a review for readability and usage.
//                  The usage should be: [Provider]+data structure, e.g., openai_api_rs::v1::chat_completion::ChatCompletionMessage would be generalised as ProviderCompletionMessage
//             - fields in structs may not fully encapsulate need as the LLM usage is more finly tuned

// Data structures that can be outbound (requests) or inbound (responses)

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) enum ProviderMessageRole {
    User,
    System,
    Assistant,
    Function,
}

// Outbound data structures - i.e. for requests to the provider LLM
#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct ProviderCompletionMessage {
    // pub(crate) id: String,
    pub(crate) role: ProviderMessageRole,
    pub(crate) content: String,
}

// Inbound data structures - i.e. for reponses from the provider LLM
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

// Provider specific data structure conversion - i.e. create an openai ['ChatCompletionMessage'] from an generic ['ProviderCompletionMessage']

// Request conversions
/// Converts a [`ProviderCompletionMessage`] struct into a [`ProviderSpecificMessage`]
pub(crate) trait ProviderMessageConverter {
    type ProviderOutputMessage;

    fn convert_message(&self, message: &ProviderCompletionMessage) -> Self::ProviderOutputMessage;
    fn convert_messages(
        &self,
        messages: &[ProviderCompletionMessage],
    ) -> Vec<Self::ProviderOutputMessage>;
}

/// An OpenAI converter
pub(crate) struct OpenAIMessageConverter;

impl ProviderMessageConverter for OpenAIMessageConverter {
    type ProviderOutputMessage = ChatCompletionMessage;

    fn convert_message(&self, message: &ProviderCompletionMessage) -> Self::ProviderOutputMessage {
        let provider_message_role: Option<ProviderMessageRole> = Some(message.role.clone());

        let role: MessageRole = match provider_message_role {
            Some(ProviderMessageRole::User) => MessageRole::user,
            Some(ProviderMessageRole::System) => MessageRole::system,
            Some(ProviderMessageRole::Assistant) => MessageRole::assistant,
            Some(ProviderMessageRole::Function) => MessageRole::function,
            _ => MessageRole::user,
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
// Similarly, create converters for Google, Anthropic, Meta, etc.
// here

// Response conversions
pub(crate) trait ProviderResponseConverter {
    fn to_generic_provider_response(
        &self,
        response: &ChatCompletionResponse,
    ) -> ProviderCompletionResponse;
}

/// OpenAI converter
pub(crate) struct OpenAIResponseConverter;
/// converts an openai_api_rs [`ChatCompletionResponse`] to a [`ProviderCompletionResponse`]
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
