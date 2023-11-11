//!
//! TODO:   Reformat and add nuance to calls to model.
//!         Extract the folder and file handling to another module:
//!             - Allows for multiple models or providers
//!             - Cleans up the code
//!             - Allows for simpler testing
//!         Add in a counter of number of files
//!         Have a timeout, or a means of knowing the status of the model.
//!         Add files to skip.
//!         Add folders to skip, based on
//! 
use crate::config::Config;
use reqwest::Client;
use serde_json::json;
use serde_derive::Deserialize;
use std::path::Path;
use walkdir::WalkDir;
use std::fs;
use std::fs::File;
use std::io::Write;

#[derive(Debug, Deserialize)]
pub struct ModelResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Debug, Deserialize)]
struct Message {
    content: String,
}

const GENERAL_CODE_REVIEW_PROMPT: &str = "You are an expert code reviewer. \
                                   Don't explain what you are doing. \
                                   Just tell me any errors or improvements.";
const SECURITY_CODE_REVIEW_PROMPT: &str = "You are an security expert who reviews code for security recommendations.";
// TODO: This needs to be better configured - TODO: pull all this into a configuration module
const MODEL_API_URL: &str = "https://api.openai.com/v1/chat/completions";

pub async fn run_code_review(config: Config) -> Result<(), Box<dyn std::error::Error>> {

    let system_message = match config.review_type.as_str() {
        // TODO: This stinks as a way of modifying the model input
        // TODO: add in additional needs from UI
        "1" => GENERAL_CODE_REVIEW_PROMPT,
        "2" => SECURITY_CODE_REVIEW_PROMPT,
        _ => {
            eprintln!("Invalid option. Exiting.");
            return Ok(());
        }
    };
    let client: Client = Client::new();

    // TODO: move into the UI as a choice
    let mut file: File = File::create("review.txt")?;

    for entry in WalkDir::new(config.folder_path) {
        let entry: walkdir::DirEntry = entry?;
        let path: &Path = entry.path();
        if path.is_file() {
            process_file(&client, &MODEL_API_URL,
                        &config.openai_api_key, 
                        &config.openai_model, 
                        path, &mut file, system_message).await?;
        } else {
            // TODO: add in skip files here
            println!("Directory {}.",path.display());
        }
    }

    Ok(())
}

async fn process_file(
    client: &Client,
    url: &str,
    api_key: &str,
    model: &str,
    path: &Path,
    file: &mut File,
    system_message: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Handling file: {}", path.display());
    let code: String = fs::read_to_string(path)?;
    // TODO: pull this into a formatter
    let prompt: String = format!("Review the following code:\n\n```\n{}\n```", code);

    let response: ModelResponse = send_prompt(&client, &url, &model, &api_key, &prompt, system_message).await?;
    
    // TODO: Need to have a proper formatter here so the reponses can be later post-processed
    //       Needs to be in structured text, such as a CSV format, so statistics can be gleaned
    let response_text: String = format!("File: {}\nReview:\n{}\n\n", 
                                        path.display(), 
                                        response.choices[0].message.content);

    // TODO: resolve the means of communicating to user
    println!("{}", response_text);
    file.write_all(response_text.as_bytes())?;
    Ok(())
}
// Refactored
async fn send_prompt(
    client: &Client,
    url: &str,
    model: &str,
    api_key: &str,
    prompt: &str,
    system_message: &str,
) -> Result<ModelResponse, Box<dyn std::error::Error>> {
    let response = client
        .post(url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&json!({
            "model": model,
            "messages": [{"role": "system", "content": system_message}, {"role": "user", "content": prompt}],
            "max_tokens": 2000,
            "n": 1,
            "stop": null
        }))
        .send()
        .await;

    match response {
        Ok(res) => {
            if res.status().is_success() {
                match res.json::<ModelResponse>().await {
                    Ok(data) => Ok(data),
                    Err(e) => Err(format!("Failed to deserialize response: {}", e).into()),
                }
            } else {
                Err(format!("Server returned error: {}", res.status()).into())
            }
        }
        Err(e) => {
            if e.is_timeout() {
                return Err("Network request timed out".into());
            } else if e.is_status() {
                return Err(format!("Server returned error: {}", e.status().unwrap()).into());
            } else {
                return Err(format!("Network request failed: {}", e).into());
            }
        }

    }
}

