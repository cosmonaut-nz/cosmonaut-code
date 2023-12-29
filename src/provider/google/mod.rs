pub(crate) mod gemini;
pub(crate) mod vertex_ai;

/// The data structures for the Google API response
pub(super) mod data {
    use google_generative_ai_rs::v1::gemini::response::Response;
    use serde::Deserialize;

    use crate::provider::api::{
        ProviderCompletionResponse, ProviderResponseChoice, ProviderResponseConverter,
        ProviderResponseMessage,
    };

    // The structs that represent the Google API response
    /// Top-level response from Google Gemini
    #[derive(Debug, Default, Clone, Deserialize)]
    pub struct GeminiResponse {
        pub model: Option<String>,
        pub candidates: Vec<Candidate>,
        #[serde(rename = "promptFeedback")]
        pub prompt_feedback: Option<PromptFeedback>,
    }
    /// Google has the concept of a candidate. Similar to an OpenAI 'choice', but at the top-level
    #[derive(Debug, Clone, Deserialize)]
    pub struct Candidate {
        pub content: Content,
        #[serde(rename = "finishReason")]
        pub finish_reason: Option<String>,
        pub index: Option<i32>,
        #[serde(rename = "safetyRatings")]
        pub safety_ratings: Vec<SafetyRating>,
    }
    /// The Content for a Candidate is further broken down into Parts
    #[derive(Debug, Clone, Deserialize)]
    pub struct Content {
        pub parts: Option<Vec<Part>>,
        pub role: GeminiRole,
    }
    #[derive(Debug, Clone, Deserialize)]
    pub struct Part {
        pub text: String,
    }
    /// Feedback on the prompt
    #[derive(Debug, Clone, Deserialize)]
    pub struct PromptFeedback {
        #[serde(rename = "safetyRatings")]
        pub safety_ratings: Vec<SafetyRating>,
    }

    /// The Google safety rating
    #[derive(Debug, Clone, Deserialize)]
    pub struct SafetyRating {
        pub category: String,
        pub probability: String,
    }
    /// A Google role is only ever 'user' or 'agent'
    #[derive(Debug, Clone, Deserialize)]
    #[serde(rename_all = "lowercase")]
    pub enum GeminiRole {
        User,
        Model,
    }
    // Implementation of ProviderResponseConverter for the Gemini FM.
    pub(crate) struct GeminiResponseConverter {
        model: String,
    }
    impl ProviderResponseConverter<Response> for GeminiResponseConverter {
        fn new(model: String) -> Self {
            GeminiResponseConverter { model }
        }
        fn to_generic_provider_response(
            &self,
            google_response: &Response,
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

    #[cfg(test)]
    mod tests {}
}
