//! A set of prompts for a chat-based LLM.
//!
//! Eventually, this is the location to 'tune' the prompts using something like 'Step-Back' or similar
//!
//!

use crate::provider::api::{ProviderCompletionMessage, ProviderMessageRole};
use crate::settings::ProviderSettings;
use log::debug;
// TODO: assess how well these prompts are engineered and evaluate the need to alter prompts between providers for the optimal outcome for each.
use serde::{Deserialize, Serialize};

const FILE_REVIEW_SCHEMA: &str = include_str!("../provider/specification/file_review.schema.json");

#[derive(Serialize, Deserialize, Debug)]
pub struct PromptData {
    pub id: Option<String>,
    pub messages: Vec<ProviderCompletionMessage>,
}
impl PromptData {
    /// Adds a user Message to the Vec of Messages
    pub fn add_user_message_prompt(&mut self, content: String) {
        let user_message = ProviderCompletionMessage {
            role: ProviderMessageRole::User,
            content,
        };
        self.messages.push(user_message);
    }
    /// Gets a specific prompt for a given provider
    pub fn get_code_review_prompt(for_provider: &ProviderSettings) -> Self {
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
                    content: r#"Provide your analysis in a valid JSON format, in which any invalid characters are correctly escaped, please use the following JSON Schema.
                            "#.to_string(),
                },
                ProviderCompletionMessage {
                    role: ProviderMessageRole::System,
                    content: FILE_REVIEW_SCHEMA.to_string(),
                },
            ],
        }
    }
    pub fn get_security_review_prompt(for_provider: &ProviderSettings) -> Self {
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
                    content: r#"Provide your analysis in a valid JSON format, in which any invalid characters are correctly escaped, please use the following JSON Schema.
                            "#.to_string(),
                },
                ProviderCompletionMessage {
                    role: ProviderMessageRole::System,
                    content: FILE_REVIEW_SCHEMA.to_string(),
                },
            ],
        }
    }
}

// TODO Add in prompts to summarise a file - e.g., the list of FileReview summaries after the code is reviewed, and the repository README.md
//
