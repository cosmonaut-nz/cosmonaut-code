//! A set of prompts for a chat-based LLM.
//!
//! This is the location to 'tune' the prompts using something like 'Step-Back' or similar
//! The prompt can be specific to a provider
//!
//!

use crate::provider::api::{ProviderCompletionMessage, ProviderMessageRole};
use crate::settings::ProviderSettings;
use log::debug;
use serde::{Deserialize, Serialize};

const FILE_REVIEW_SCHEMA: &str = include_str!("../provider/specification/file_review.schema.json");
const JSON_HANDLING_ADVICE: &str = r#"Provide your analysis in valid JSON format. 
                                    Strictly escape any characters within your response strings that will create invalid JSON, such as \" - i.e., double quotes. 
                                    Never use comments in your JSON. Ensure that your output exactly conforms to the following JSON Schema 
                                    and you follow the instructions provided in "description" fields."#;

/// Holds the id and [`Vec`] of [`ProviderCompletionMessage`]s
#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct PromptData {
    pub(crate) id: Option<String>,
    pub(crate) messages: Vec<ProviderCompletionMessage>,
}

impl PromptData {
    /// Adds a user Message to the Vec of Messages
    pub(crate) fn add_user_message_prompt(&mut self, content: String) {
        let user_message = ProviderCompletionMessage {
            role: ProviderMessageRole::User,
            content,
        };
        self.messages.push(user_message);
    }
    /// Gets a specific prompt for a given provider
    pub(crate) fn get_code_review_prompt(for_provider: &ProviderSettings) -> Self {
        debug!("Provider: {}", for_provider);
        Self {
            id: None,
            messages: vec![
                ProviderCompletionMessage {
                    role: ProviderMessageRole::System,
                    content: "As an expert code reviewer with comprehensive knowledge in software development standards, review the following code.".to_string(),
                },
                ProviderCompletionMessage {
                    role: ProviderMessageRole::System,
                    content: r#"Focus on identifying critical errors, best practice violations, and security vulnerabilities.  
                                Exclude trivial issues like formatting errors or TODO comments. Use your expertise to provide insightful and actionable feedback.
                            "#.to_string(),
                },
                ProviderCompletionMessage {
                    role: ProviderMessageRole::System,
                    content: JSON_HANDLING_ADVICE.to_string(),
                },
                ProviderCompletionMessage {
                    role: ProviderMessageRole::System,
                    content: FILE_REVIEW_SCHEMA.to_string(),
                },
            ],
        }
    }
    pub(crate) fn get_security_review_prompt(for_provider: &ProviderSettings) -> Self {
        debug!("Provider: {}", for_provider);
        Self {
            id: None,
            messages: vec![
                ProviderCompletionMessage {
                    role: ProviderMessageRole::System,
                    content: "As an expert security code reviewer with comprehensive knowledge in software and information security, review the following code.".to_string(),
                },
                ProviderCompletionMessage {
                    role: ProviderMessageRole::System,
                    content: r#"Focus exclusively on identifying security vulnerabilities and potential security flaws in the code. 
                                Provide actionable feedback and mitigation strategies for each identified issue.
                                You do not have to offer improvement recommendations for the code, focus solely on security.
                                If no errors or security issues are found, the file_rag_status should be 'Green'"#.to_string(),
                },
                ProviderCompletionMessage {
                    role: ProviderMessageRole::System,
                    content: JSON_HANDLING_ADVICE.to_string(),
                },
                ProviderCompletionMessage {
                    role: ProviderMessageRole::System,
                    content: FILE_REVIEW_SCHEMA.to_string(),
                },
            ],
        }
    }
    /// gets a [`PromptData`] for a LLM to summarise the README in a repository for the RepositoryReview.repository_purpose field
    // TODO: the overall summary of
    pub(crate) fn _get_readme_summary_prompt(for_provider: &ProviderSettings) -> Self {
        debug!("Provider: {}", for_provider);
        Self {
            id: None,
            messages: vec![ProviderCompletionMessage {
                role: ProviderMessageRole::System,
                content: "PROMPT TO WRITE HERE".to_string(),
            }],
        }
    }
    /// gets a [`PromptData`] for a LLM to summarise the overall review from a [`Vec`] of [`FileReview`]  
    // TODO: the overall summary of the returned FileReviews for the RepositoryReview.summary field
    pub(crate) fn _get_overall_summary_prompt(for_provider: &ProviderSettings) -> Self {
        debug!("Provider: {}", for_provider);
        Self {
            id: None,
            messages: vec![ProviderCompletionMessage {
                role: ProviderMessageRole::System,
                content: "PROMPT TO WRITE HERE".to_string(),
            }],
        }
    }
}
