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
                                Do not generalise; you link your statements to the code; you must state 'is' or 'will', not 'may' or 'shall'; 
                                it must be specific to the text of the code you are reviewing.
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
                // TODO: add in fake "replay" to show previous user/assistant interactions and quality of output on a file
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
                // TODO: add in fake "replay" to show previous user/assistant interactions and quality of output on a file
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
                    content: r#"Use excellent grammar. Keep the summary very brief and concise. Only give significant information: general code quality; an overview of security; and overview of code quality. 
                                Do not mention file names."#.to_string(),
                },
                ProviderCompletionMessage {
                    role: ProviderMessageRole::System,
                    content: r"Do not use Markdown as output. 
                                Output in plaintext only. 
                                Always add newline characters (i.e., '\n') for paragraphs, indented bullets or item numbering.".to_string(),
                }, // Fake "replaying" previous interactions to show the level of summary.
                ProviderCompletionMessage {
                    role: ProviderMessageRole::User,
                    content: r#"Concisely summarise the following: 
                                The code in 'build.rs' does not appear to contain critical errors or security issues. 
                                The use of imports could be refined, and the string conversion for static strings could be optimized for better code quality and readability.
                                The code contains a high severity security issue related to the potential leaking of API keys, and a couple of errors that impact the clarity and security of the configuration data handling. 
                                The code does not adhere strictly to the best practices regarding secure configuration management. 
                                An improvement is suggested to enhance the security by modifying the Debug trait implementation for sensitive data.
                                The Rust source code provides prompt templates for a chat-based language model. There are no critical errors or security vulnerabilities within the code. 
                                Minor improvements suggested include handling JSON schemas and multi-line strings separately from the code, 
                                renaming a public function to adhere to naming conventions, and enhancing a documentation comment for clarity.
                                The code could lead to security vulnerabilities due to the exposure of sensitive API keys and lack of a backoff strategy in API request retries. 
                                Code improvements suggested include better error handling patterns, reorganizing the enumeration definition into its separate module, 
                                and deriving the Debug trait for better logging support. The use of wildcard imports should be replaced with specific imports for clarity and maintainability.
                                The code review did not find any critical errors or security issues. Suggested improvements focus on adding documentation, implementing better error handling, 
                                and refactoring the trait for conversion functions to accept iterators for greater flexibility.
                                "#.to_string(),
                },
                ProviderCompletionMessage {
                    role: ProviderMessageRole::Assistant,
                    content: r#"The codebase demonstrates a satisfactory level of code quality with no critical errors or security issues found. 
                                However, security-wise, there's a concerning issue as the code risks leaking API keys, which is a high severity vulnerability.
                                There is room for optimization and refinement to enhance code quality and readability. Configuration data handling could be improved, 
                                as it does not fully comply with secure configuration management best practices. 
                                Significant improvements to documentation for higher clarity is desired."#.to_string(),
                },
            ],
        }
    }
}
