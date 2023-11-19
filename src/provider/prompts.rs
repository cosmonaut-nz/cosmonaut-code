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

#[derive(Serialize, Deserialize, Debug)]
pub struct PromptData {
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
            messages: vec![
                ProviderCompletionMessage {
                    role: ProviderMessageRole::System,
                    content: "As an expert code reviewer with comprehensive knowledge in software development standards, review the following code.".to_string(),
                },
                ProviderCompletionMessage {
                    role: ProviderMessageRole::System,
                    content: r#"Focus on identifying critical errors, best practice violations, and security vulnerabilities.  
                                Exclude trivial issues like formatting errors or TODO comments. Use your expertise to provide insightful and actionable feedback.
                                If no errors or security issues are found, and less than ten (10) improvements found, the file_rag_status should be 'Green'.
                            "#.to_string(),
                },
                ProviderCompletionMessage {
                    role: ProviderMessageRole::System,
                    content: r#"Provide your analysis in a valid JSON format, in which any invalid characters are correctly escaped, exactly following this structure: 
                                { 
                                    filename: String, // The name of the file
                                    summary: String,  // A summary of the findings of the review
                                    file_rag_status: String, // In {'Red', 'Amber', 'Green'}
                                    errors: [Vec<Error>], // A list of errors found in the code giving: 
                                            [{ 
                                                code: String, // The code affected, including line number
                                                issue: String, // A description of the error
                                                resolution: String, // The potential resolution
                                            }]
                                    improvements: Vec<Improvement>, // A list of improvements to the code, if any, giving:
                                            [{ 
                                                code: String, // The code affected, including line number
                                                suggestion: String, // A suggestion to improve the code
                                                example: String, // An example improvement
                                            }]
                                    security_issues: Vec<SecurityIssue>, // A list of security issues found in the code, if any, giving:
                                            [{ 
                                                code: String, // The code affected, including line number 
                                                threat: String, // A description of the threat
                                                mitigation: String, // The potential mitigation
                                            }]
                                    statistics: String // A list of statistics (e.g., code type, lines of code, number of functions, number of methods, etc.)
                                }
                            "#.to_string(),
                },
            ],
        }
    }
    pub fn get_security_review_prompt(for_provider: &ProviderSettings) -> Self {
        debug!("Provider: {}", for_provider);
        Self {
            messages: vec![
                ProviderCompletionMessage {
                    role: ProviderMessageRole::System,
                    content: "As an expert security code reviewer with comprehensive knowledge in software and information security, review the following code.".to_string(),
                },
                ProviderCompletionMessage {
                    role: ProviderMessageRole::System,
                    content: r#"Focus exclusively on identifying security vulnerabilities and potential security flaws in the code. 
                                Provide actionable feedback and mitigation strategies for each identified issue.
                                If no errors or security issues are found, the file_rag_status should be 'Green'"#.to_string(),
                },
                ProviderCompletionMessage {
                    role: ProviderMessageRole::System,
                    content: r#"Provide your analysis in a valid JSON format, in which any invalid characters are correctly escaped, exactly following this structure: 
                                { 
                                    filename: String, // The name of the file
                                    summary: String,  // A summary of the findings of the review
                                    file_rag_status: String, // In {'Red', 'Amber', 'Green'}
                                    security_issues: Vec<SecurityIssue>, // A list of security issues found in the code, if any, giving:
                                            [{ 
                                                code: String, // The code affected, including line number 
                                                threat: String, // A description of the threat
                                                mitigation: String, // The potential mitigation
                                            }]
                                    statistics: String // A list of statistics (e.g., code type, lines of code, number of functions, number of methods, etc.)
                                }
                            "#.to_string(),
                },
                // Add user messages if needed
                // Message { role: "user".to_string(), content: "..." },
            ],
        }
    }
}

// TODO Add in prompts to summarise a file - e.g., the list of FileReview summaries after the code is reviewed, and the repository README.md
//
