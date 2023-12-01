//! A set of prompts for a chat-based LLM.
//!
//! This is the location to 'tune' the prompts using something like 'Step-Back' or similar
//! The prompt can be specific to a provider
//!
//!

use crate::provider::api::{ProviderCompletionMessage, ProviderMessageRole};
use serde::{Deserialize, Serialize};

const FILE_REVIEW_SCHEMA: &str = include_str!("../provider/specification/file_review.schema.json");
const JSON_HANDLING_ADVICE: &str = r#"Provide your analysis strictly in valid JSON format. 
                                    Strictly escape any characters within your response strings that will create invalid JSON, such as \" - i.e., double quotes. 
                                    Never use comments in your JSON. 
                                    Ensure that your output exactly conforms to the following JSON Schema 
                                    and you follow exactly the instructions provided in "description" fields."#;

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
    pub(crate) fn get_code_review_prompt() -> Self {
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
                                Do not generalise; you link your statements to the code; you must state 'is' or 'will', not 'may' or 'shall'; it must be specific to the text of the code you are reviewing.
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
    pub(crate) fn get_security_review_prompt() -> Self {
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
    pub(crate) fn _get_readme_summary_prompt() -> Self {
        Self {
            id: None,
            messages: vec![
                ProviderCompletionMessage {
                    role: ProviderMessageRole::System,
                    content: r#"As an expert software code reviewer with comprehensive knowledge in software and information security, 
                                summarise the following documentation."#.to_string(),
                },
                ProviderCompletionMessage {
                    role: ProviderMessageRole::System,
                    content: r#"Keep the summary very brief and concise, while still clearly describing the purpose of the repository."#.to_string(),
                },
            ],
        }
    }
    /// gets a [`PromptData`] for a LLM to summarise the overall review from a [`Vec`] of [`FileReview`]  
    #[allow(dead_code)]
    pub(crate) fn get_overall_summary_prompt() -> Self {
        Self {
            id: None,
            messages: vec![
                ProviderCompletionMessage {
                    role: ProviderMessageRole::System,
                    content: r#"As an expert software code reviewer with comprehensive knowledge in software and information security, 
                                summarise a set of findings from the detailed review of this repository."#.to_string(),
                },
                ProviderCompletionMessage {
                    role: ProviderMessageRole::System,
                    content: r#"Keep the summary very brief and concise. Only give significant information, such an overview of security and code quality."#.to_string(),
                },
                ProviderCompletionMessage {
                    role: ProviderMessageRole::System,
                    content: r#"Format your output in a pretty manner, ensuring that it includes clear paragraphs and indented bullets or item numbering, if present."#.to_string(),
                },
            ],
        }
    }
}
