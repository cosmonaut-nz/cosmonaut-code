pub(crate) mod gemini;
pub(crate) mod vertex_ai;

/// The data structures for the Google API response
pub(super) mod data {
    use serde::Deserialize;

    use crate::provider::api::{
        ProviderCompletionResponse, ProviderResponseChoice, ProviderResponseConverter,
        ProviderResponseMessage,
    };

    // The structs that represent the Google API response
    /// Top-level response from Google Gemini
    #[derive(Debug, Deserialize)]
    pub struct GeminiResponse {
        pub model: Option<String>,
        pub candidates: Vec<Candidate>,
        #[serde(rename = "promptFeedback")]
        pub prompt_feedback: Option<PromptFeedback>,
    }
    /// Google has the concept of a candidate. Similar to an OpenAI 'choice', but at the top-level
    #[derive(Debug, Deserialize)]
    pub struct Candidate {
        pub content: Content,
        #[serde(rename = "finishReason")]
        pub finish_reason: Option<String>,
        pub index: i32,
        #[serde(rename = "safetyRatings")]
        pub safety_ratings: Vec<SafetyRating>,
    }
    /// The Content for a Candidate is further broken down into Parts
    #[derive(Debug, Deserialize)]
    pub struct Content {
        pub parts: Vec<Part>,
        pub role: GeminiRole,
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
    pub enum GeminiRole {
        User,
        Model,
    }
    // Implementation of ProviderResponseConverter for LM Studio.
    pub(crate) struct GeminiResponseConverter;
    impl ProviderResponseConverter<GeminiResponse> for GeminiResponseConverter {
        fn to_generic_provider_response(
            &self,
            google_response: &GeminiResponse,
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
            let google_response = GeminiResponse {
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
                        role: GeminiRole::User,
                    },
                    finish_reason: Some("complete".to_string()),
                    index: 0,
                    safety_ratings: vec![SafetyRating {
                        category: "safety".to_string(),
                        probability: "high".to_string(),
                    }],
                }],
                prompt_feedback: Some(PromptFeedback {
                    safety_ratings: vec![SafetyRating {
                        category: "safety".to_string(),
                        probability: "high".to_string(),
                    }],
                }),
            };

            let converter = GeminiResponseConverter;
            let provider_response = converter.to_generic_provider_response(&google_response);

            assert_eq!(provider_response.choices.len(), 1);
            assert_eq!(provider_response.choices[0].message.content, "Hello\nWorld");
        }
    }
}
