//!
//!  Handles the review of a file from a repository
//!
use crate::data;
mod prompts;
mod static_config;
use crate::settings::Settings;
use log::{debug, info};
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;

#[derive(Debug, Deserialize)]
pub struct ModelResponse {
    pub choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
pub struct Choice {
    pub message: Message,
}

#[derive(Debug, Deserialize)]
pub struct Message {
    pub content: String,
}
//
#[derive(Debug, Deserialize, Default, PartialEq)]
enum ReviewType {
    #[default]
    General,
    Security,
}
/// We offer two types of review:
/// 1. A full general review of the code
/// 2. A review focussed on security only
impl ReviewType {
    pub fn from_config(settings: &Settings) -> Self {
        match settings.review_type {
            1 => ReviewType::General,
            2 => ReviewType::Security,
            _ => {
                info!("Using default: {:?}", ReviewType::default());
                ReviewType::default()
            }
        }
    }
}

/// Sends the contents of a code file to an LLM agent to evaluate according to the prompt
/// passed to it.
///
/// This function takes a Config struct, a prompt str and file_contents str as parameters
/// and returns the response from the LLM as a ModelResponse.
///
/// # Parameters
///
/// * `config` - A ['Config'] that contains information for the LLM
/// * 'file_contents' - the body of the output_file to be reviewed
///
/// # Returns
///
/// * A response from the LLM ['ModelResponse']
///
/// TODO DECISION: enable other output file formats - e.g., "csv", etc.?
///
pub async fn review_file_via_llm(
    settings: &Settings,
    file_contents: &str,
) -> Result<data::FileReview, Box<dyn std::error::Error>> {
    // Determine the review type and generate the appropriate prompt
    let review_type = ReviewType::from_config(settings);
    let prompt = match review_type {
        ReviewType::General => prompts::GENERAL_CODE_REVIEW_PROMPT,
        ReviewType::Security => prompts::SECURITY_CODE_REVIEW_PROMPT,
    };
    // We set a timeout on the client
    let client: Client = Client::builder()
        .timeout(static_config::API_TIMEOUT)
        .build()?;
    // TODO work this out and make nicer
    let response: Result<reqwest::Response, reqwest::Error> = client
        .post(&settings.provider.api_url)
        .header("Authorization", format!("Bearer {}", settings.sensitive.api_key.get()))
        .header("Content-Type", "application/json")
        .json(&json!({
            "model": settings.provider.model,
            "messages": [{"role": "system", "content": prompt}, {"role": "user", "content": file_contents}],
            "max_tokens": settings.provider.max_tokens,
            "n": 1,
            "stop": null
        }))
        .send()
        .await;
    match response {
        Ok(res) => {
            if res.status().is_success() {
                match res.json::<ModelResponse>().await {
                    Ok(data) => {
                        let orig_response_json: String =
                            data.choices[0].message.content.to_string();
                        // Strip any model markers from the reponse
                        match strip_json_markers_from(&orig_response_json, &settings.provider.name)
                        {
                            Ok(stripped_json) => {
                                #[cfg(debug_assertions)]
                                pretty_print_json_for_debug(&stripped_json);

                                match data::deserialize_file_review(&stripped_json) {
                                    Ok(filereview_from_json) => Ok(filereview_from_json),
                                    Err(e) => {
                                        panic!("Failed to deserialize into FileReview: {}", e)
                                    }
                                }
                            }
                            Err(e) => Err(format!("Error stripping JSON markers: {}", e).into()),
                        }
                    }
                    Err(e) => Err(format!("Failed to deserialize response: {}", e).into()),
                }
            } else {
                Err(format!("Server returned error: {}", res.status()).into())
            }
        }
        Err(e) => {
            if e.is_timeout() {
                Err("Network request timed out".into())
            } else if e.is_status() {
                Err(format!("Server returned error: {}", e.status().unwrap()).into())
            } else {
                Err(format!("Network request failed: {}", e).into())
            }
        }
    }
}

/// Removes any artefacts from an AI review
///
/// In some cases the AI agent add in markers for the content type,
/// e.g., openai adds "```json" at the beginning, and "```" at the end of response to mark the type of content
///
/// # Parameters
///
/// * `orig_json_str` - A str representation of the review_response
///
/// # Returns
///
/// * A String of the review_reponse with the markers removed
///
///
fn strip_json_markers_from(
    orig_json_str: &str,
    provider_org: &str,
) -> Result<String, &'static str> {
    if provider_org == static_config::openai::PROVIDER_NAME {
        debug!("Processing JSON response for OpenAI");

        // Find the first opening brace and the last closing brace
        if let (Some(start), Some(end)) = (orig_json_str.find('{'), orig_json_str.rfind('}')) {
            if start < end {
                // Extract the JSON substring
                Ok(orig_json_str[start..=end].to_string())
            } else {
                Err("Invalid JSON structure")
            }
        } else {
            Err("No valid JSON found")
        }
    } else {
        debug!("No processing needed for non-OpenAI provider.");
        Ok(orig_json_str.to_string())
    }
}

/// A utility to check the JSON sent back from the LLM
#[cfg(debug_assertions)]
fn pretty_print_json_for_debug(orig_json_str: &str) {
    debug!("Pretty printing the JSON\n {}", orig_json_str);
    match serde_json::from_str::<serde_json::Value>(orig_json_str) {
        Ok(json_value) => {
            if let Ok(pretty_json) = serde_json::to_string_pretty(&json_value) {
                debug!("Pretty JSON: {}", pretty_json);
            } else {
                debug!("Failed to pretty-print JSON");
            }
        }
        Err(e) => {
            debug!("Failed to parse JSON: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const JSON_OPENING: &str = "```json";
    const JSON_CLOSE: &str = "```";
    #[test]
    fn test_strip_json_markers_openai() {
        let json_str_with_markers =
            format!("{}\n{{\"key\": \"value\"}}\n{}", JSON_OPENING, JSON_CLOSE);
        let result =
            strip_json_markers_from(&json_str_with_markers, static_config::openai::PROVIDER_NAME);
        assert_eq!(result.unwrap(), "{\"key\": \"value\"}");
    }

    #[test]
    fn test_no_markers_openai() {
        let json_str = "{\"key\": \"value\"}";
        let result = strip_json_markers_from(json_str, static_config::openai::PROVIDER_NAME);
        assert_eq!(result.unwrap(), json_str);
    }

    #[test]
    fn test_invalid_provider() {
        let json_str_with_markers =
            format!("{}\n{{\"key\": \"value\"}}\n{}", JSON_OPENING, JSON_CLOSE);
        let result = strip_json_markers_from(&json_str_with_markers, "some_other_provider");
        assert_eq!(result.unwrap(), json_str_with_markers);
    }

    #[test]
    fn test_invalid_json_markers_openai() {
        let json_str_with_extra_text = "xxx\n{\"key\": \"value\"}\nyyy";
        let expected_json = "{\"key\": \"value\"}";
        let result = strip_json_markers_from(
            json_str_with_extra_text,
            static_config::openai::PROVIDER_NAME,
        );
        assert_eq!(result.unwrap(), expected_json);
    }
}
