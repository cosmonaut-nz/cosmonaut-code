//! A set of prompts for a chat-based LLM.
//!
//! This is the location to 'tune' the prompts using something like 'Step-Back' or similar
//! The prompt can be specific to a provider
//!
use crate::provider::api::{ProviderCompletionMessage, ProviderMessageRole};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

const FILE_REVIEW_SCHEMA: &str = include_str!("../provider/specification/file_review.schema.json");
const CODE_REVIEW_PROMPT: &str = include_str!("../provider/prompts/code_review.json");
const SECURITY_REVIEW_PROMPT: &str = include_str!("../provider/prompts/security_review.json");
#[allow(dead_code)]
const README_SUMMARY_PROMPT: &str = include_str!("../provider/prompts/readme_summary.json");
const REPOSITORY_SUMMARY_PROMPT: &str = include_str!("../provider/prompts/repository_summary.json");

const LANGUAGE: &str = "British English";

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
    pub(crate) fn get_code_review_prompt() -> Result<Self, Box<dyn std::error::Error>> {
        let json_content = create_content(&[
            ("language", LANGUAGE),
            ("file_review_schema", FILE_REVIEW_SCHEMA),
        ]);
        let result = substitute_tokens(CODE_REVIEW_PROMPT, &json_content)?;
        let messages = get_messages_from(&result)?;
        Ok(Self { id: None, messages })
    }
    pub(crate) fn get_security_review_prompt() -> Result<Self, Box<dyn std::error::Error>> {
        let json_content = create_content(&[
            ("language", LANGUAGE),
            ("file_review_schema", FILE_REVIEW_SCHEMA),
        ]);
        let result = substitute_tokens(SECURITY_REVIEW_PROMPT, &json_content)?;
        let messages = get_messages_from(&result)?;
        Ok(Self { id: None, messages })
    }
    /// gets a [`PromptData`] for a LLM to summarise the README in a repository for the RepositoryReview.repository_purpose field
    // TODO not yet used. Part of the documentation review module
    pub(crate) fn _get_readme_summary_prompt() -> Result<Self, Box<dyn std::error::Error>> {
        let json_content = create_content(&[("language", LANGUAGE)]);
        let result = substitute_tokens(README_SUMMARY_PROMPT, &json_content)?;
        let messages = get_messages_from(&result)?;
        Ok(Self { id: None, messages })
    }
    /// gets a [`PromptData`] for a LLM to summarise the overall review from a [`Vec`] of [`FileReview`]  
    #[allow(dead_code)]
    pub(crate) fn get_overall_summary_prompt() -> Result<Self, Box<dyn std::error::Error>> {
        let json_content = create_content(&[("language", LANGUAGE)]);
        let result = substitute_tokens(REPOSITORY_SUMMARY_PROMPT, &json_content)?;
        let messages = get_messages_from(&result)?;
        Ok(Self { id: None, messages })
    }
}
/// Creates a [`HashMap`] from a slice of tuples
fn create_content(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}
/// Gets a [`Vec`] of [`ProviderCompletionMessage`]s from a JSON string
fn get_messages_from(json_data: &str) -> Result<Vec<ProviderCompletionMessage>, serde_json::Error> {
    let v: Value = serde_json::from_str(json_data)?;
    let messages: Vec<ProviderCompletionMessage> = serde_json::from_value(v["messages"].clone())?;

    Ok(messages)
}
/// Substitutes tokens in a JSON string with values from a [`HashMap`].
/// Usage: `substitute_tokens(json_str, &[("token", "value")])`
fn substitute_tokens(
    json_str: &str,
    content: &HashMap<String, String>,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut v: Value = serde_json::from_str(json_str)?;

    let re = Regex::new(r"\{\{(\w+)\}\}")?;

    if let Some(array) = v["messages"].as_array_mut() {
        for message in array {
            if let Some(content_str) = message["content"].as_str() {
                let mut new_content = content_str.to_string();
                for cap in re.captures_iter(content_str) {
                    if let Some(replacement) = content.get(&cap[1]) {
                        new_content = new_content.replace(&cap[0], replacement);
                    }
                }
                message["content"] = json!(new_content);
            }
        }
    }

    Ok(v.to_string())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_add_user_message_prompt() {
        let mut prompt_data = PromptData {
            id: Some("123".to_string()),
            messages: vec![ProviderCompletionMessage {
                role: ProviderMessageRole::User,
                content: "Hello".to_string(),
            }],
        };

        prompt_data.add_user_message_prompt("World".to_string());

        assert_eq!(prompt_data.messages.len(), 2);
        assert_eq!(prompt_data.messages[1].content, "World");
    }
    #[test]
    fn test_create_content() {
        let pairs = &[("language", "English"), ("file_review_schema", "Schema")];
        let content = create_content(pairs);

        assert_eq!(content.len(), 2);
        assert_eq!(content["language"], "English");
        assert_eq!(content["file_review_schema"], "Schema");
    }
    #[test]
    fn test_get_messages_from() {
        let json_data = r#"
            {
                "messages": [
                    {
                        "role": "user",
                        "content": "Hello"
                    },
                    {
                        "role": "system",
                        "content": "Welcome"
                    }
                ]
            }
        "#;

        let messages = get_messages_from(json_data).unwrap();

        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, ProviderMessageRole::User);
        assert_eq!(messages[0].content, "Hello");
        assert_eq!(messages[1].role, ProviderMessageRole::System);
        assert_eq!(messages[1].content, "Welcome");
    }
    #[test]
    fn test_substitute_tokens() {
        let json_str = r#"
            {
                "messages": [
                    {
                        "role": "User",
                        "content": "{{name}}"
                    }
                ]
            }
        "#;

        let mut content = HashMap::new();
        content.insert("name".to_string(), "John".to_string());

        let result = substitute_tokens(json_str, &content).unwrap();

        assert!(result.contains("John"));
    }
}
