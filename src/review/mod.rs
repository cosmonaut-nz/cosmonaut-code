//! Handles the file in a software repository.
//! Iterates over the folder structure, ignoring files or folders that are not relevant.
//! Passes each relevant (code) file for review.
pub mod code;
mod data;
mod tools;
use crate::provider::api::ProviderCompletionResponse;
use crate::provider::prompts::PromptData;
use crate::provider::review_code_file;
use crate::review::data::{FileReview, LanguageFileType, RAGStatus, RepositoryReview};
use crate::review::tools::{get_git_contributors, is_not_blacklisted};
use crate::settings::Settings;
use chrono::{DateTime, Local, Utc};
use code::analyse_languages_in_repository;
use log::{debug, error, info};
use regex::Regex;
use serde::Deserialize;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use walkdir::WalkDir;

/// Takes the filepath to a repository and iterates over the code, sending each relevant file for review.
///
/// # Parameters
///
/// * `settings` - A ['Settings'] that contains information for the LLM
pub async fn assess_codebase(
    settings: Settings,
) -> Result<RepositoryReview, Box<dyn std::error::Error>> {
    // Used for the final report to write to disk
    let output_dir = PathBuf::from(&settings.report_output_path);
    let output_file_path =
        create_timestamped_filename(&output_dir, &settings.output_type, Local::now());

    // Collect the review data in the following data struct
    let mut review = RepositoryReview::new();
    match extract_directory_name(&settings.repository_path) {
        Ok(dir_name) => review.set_repository_name(dir_name.to_string()),
        Err(e) => error!("Error extracting directory name: {}", e),
    }

    // Walk the repository structure sending relevant files to the provider ai service to review
    let repository_root = Path::new(&settings.repository_path);

    if !repository_root.is_dir() {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!(
                "Provided path is not a directory: {}",
                &settings.repository_path
            ),
        )));
    }

    // First pass: get the repository file types and languages
    // Review the repository and get the overall language file types to process
    let repo_lfts: Vec<LanguageFileType> = analyse_languages_in_repository(repository_root);
    debug!("REPOSITORY LANGUAGE FILE TYPES: {:?}", repo_lfts);
    let blacklisted_dirs: Vec<String> = tools::get_blacklist_dirs(repository_root);
    debug!("BLACKLIST: {:?}", blacklisted_dirs);

    let mut overall_file_count: i32 = 0;
    let mut file_types_count: HashMap<String, i32> = HashMap::new();
    // Second pass: review code files
    for entry in WalkDir::new(repository_root)
        .into_iter()
        .filter_entry(|e| is_not_blacklisted(e, &blacklisted_dirs))
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        if path.is_file() {
            // Check if the file extension is valid for review
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if LanguageFileType::has_extension_of(ext, &repo_lfts) {
                    // Check if the file is empty
                    if let Ok(metadata) = fs::metadata(path) {
                        if metadata.len() > 0 {
                            if let Some(ext) = entry.path().extension().and_then(|e| e.to_str()) {
                                let count = file_types_count.entry(ext.to_string()).or_insert(0);
                                *count += 1;
                            }
                            // File is not empty, proceed with review
                            debug!("Handling file: {}", path.display());
                            overall_file_count += 1;
                            let _code_from_file: String = fs::read_to_string(path)?;
                            // Create a partial FileReview struct to hold the code (refactor)

                            let response = review_file(&settings, path).await?;
                            review.add_file_review(response);
                        } else {
                            // File is empty, skip
                            debug!("Empty file skipped: {}", path.display());
                        }
                    }
                } else {
                    debug!(
                        "Non-whitelisted file extension, skipped: {}",
                        path.display()
                    );
                }
            }
        }
    }
    // Complete the fields in the ['RepositoryReview'] struct
    review.set_repository_type(settings.repository_type);
    review.set_date(Utc::now());
    review.set_repository_purpose("purpose".to_string()); // TODO: Derive this from playing the README at the LLM
    review.set_summary("summary".to_string()); // TODO: Pull together all the filereview summaries and send to LLM for condensing
    review.set_repository_rag_status(RAGStatus::Green); // TODO: Derive from summing up the RAG statuses in the filereviews and calculate...
    review.set_contributors(get_git_contributors(&settings.repository_path));
    review.set_lfts(repo_lfts);

    // Serialize the review struct to JSON
    let review_json = serde_json::to_string_pretty(&review)
        .map_err(|e| format!("Error serializing review: {}", e))?;

    // Write the JSON to the specified output file
    let mut output_file = fs::File::create(&output_file_path)
        .map_err(|e| format!("Error creating output file: {}", e))?;
    output_file
        .write_all(review_json.as_bytes())
        .map_err(|e| format!("Error writing to output file: {}", e))?;

    info!("Total number of files processed: {}", overall_file_count);
    Ok(review)
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
/// Pulls the text from a ['File'] and sends it to the LLM for review
///
/// This function takes two integer parameters and returns their sum.
/// It demonstrates basic arithmetic operations in Rust.
///
/// # Parameters
///
/// * `Settings` - A ['Settings'] that contains information for the LLM
/// * `path` - - The path the the file to process
///
async fn review_file(
    settings: &Settings,
    path: &Path,
) -> Result<FileReview, Box<dyn std::error::Error>> {
    info!("Handling output_file: {}", path.display());
    // Set up the right provider
    let provider: &crate::settings::ProviderSettings = settings.get_active_provider()
                                              .expect("Either a default or chosen provider should be configured in \'default.json\'. \
                                              Either none was found, or the default provider did not match any name in the configured providers list.");
    // Determine the review type and generate the appropriate prompt
    let review_type = ReviewType::from_config(settings);
    let mut prompt_data = match review_type {
        ReviewType::General => PromptData::get_code_review_prompt(provider),
        ReviewType::Security => PromptData::get_security_review_prompt(provider),
    };

    // TODO move this up and have this function expect a FileReview struct instead, which includes the code from the file
    let code_from_file: String = fs::read_to_string(path)?;
    let review_request: String = format!("File name: {}\n{}\n", path.display(), code_from_file);
    // Add the file as PromptData
    prompt_data.add_user_message_prompt(review_request);
    // debug!("Prompt data sent: {:?}", prompt_data);

    // Pass the file to be reviewed by the LLM service
    let response: ProviderCompletionResponse =
        review_code_file(settings, provider, prompt_data).await?;
    let orig_response_json: String = response.choices[0].message.content.to_string();
    // Strip any model markers from the reponse. LLM use markdown-style annotations for code, inc. "JSON"
    match strip_artifacts_from(&orig_response_json) {
        Ok(stripped_json) => {
            #[cfg(debug_assertions)]
            pretty_print_json_for_debug(&stripped_json);

            match data::deserialize_file_review(&stripped_json) {
                Ok(filereview_from_json) => Ok(filereview_from_json),
                Err(e) => Err(format!("Failed to deserialize into FileReview: {}", e).into()),
            }
        }
        Err(e) => Err(format!("Error stripping JSON markers: {}", e).into()),
    }
}

/// Creates a timestamped file
///
/// # Parameters
///
/// * `base_path` - where the file will be created
/// * `file_extension` - the file extension, e.g., '.json'
/// * `timestamp` - the current time, makes testing easier to mock. Example input: 'Local::now()'
fn create_timestamped_filename(
    base_path: &Path,
    file_extension: &str,
    timestamp: DateTime<Local>,
) -> PathBuf {
    let formatted_timestamp = timestamp.format("%Y%m%d_%H%M%S").to_string();
    base_path.join(format!("{}.{}", formatted_timestamp, file_extension))
}

/// extracts the final part of the path
///
/// # Parameters
///
/// * `path_str` - a str representation of the path
#[derive(Debug)]
struct PathError {
    message: String,
}
impl PathError {
    fn new(message: &str) -> PathError {
        PathError {
            message: message.to_string(),
        }
    }
}
impl fmt::Display for PathError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Path error: {}", self.message)
    }
}
impl Error for PathError {}

fn extract_directory_name(path_str: &str) -> Result<&str, PathError> {
    let path = Path::new(path_str);

    // Check if the path points to a file (has an extension)
    if path.extension().is_some() && path.is_file() {
        return Err(PathError::new("Path points to a file, not a directory"));
    }

    let dir_name = if path.ends_with("/") {
        path.parent().and_then(|p| p.file_name())
    } else {
        path.file_name()
    };

    dir_name
        .and_then(|os_str| os_str.to_str())
        .ok_or_else(|| PathError::new("Invalid directory name"))
}
/// Removes any artefacts from an AI review
///
/// In some cases the AI agent add in markdown annotation for the content type,
/// e.g., openai adds "\`\`\`json" at the beginning, and "\`\`\`" at the end of response to mark the type of content
/// In others, spurious control characters are added that mangles the JSON for deserializing, e.g. characters in the range U+0000 to U+001F
///
/// TODO: May be the cause of specific issues and subsequent JSON serialization problems. Need to review and track, right now can't reproduce
///
/// # Parameters
///
/// * `json_str` - A str representation of the review_response
///
/// # Returns
///
/// * A String of the review_reponse with the markers removed
///
fn strip_artifacts_from(orig_json_str: &str) -> Result<String, &'static str> {
    // First, clean any control characters found in the JSON
    let re = Regex::new(r"[\x00-\x1F]").unwrap(); // Control characters regex
    let sanitized_json_str = re.replace_all(orig_json_str, "");

    // Next, find the first opening brace and the last closing brace
    if let (Some(start), Some(end)) = (sanitized_json_str.find('{'), sanitized_json_str.rfind('}'))
    {
        if start < end {
            // Extract the JSON substring and return it
            Ok(sanitized_json_str[start..=end].to_string())
        } else {
            Err("Invalid JSON structure")
        }
    } else {
        debug!(
            "Didn't find any valid JSON. What was found: {}",
            orig_json_str
        );
        Err("No valid JSON found")
    }
}

/// A utility to check the JSON sent back from the LLM
#[cfg(debug_assertions)]
fn pretty_print_json_for_debug(json_str: &str) {
    match serde_json::from_str::<serde_json::Value>(json_str) {
        Ok(json_value) => {
            if let Ok(pretty_json) = serde_json::to_string_pretty(&json_value) {
                debug!("{}", pretty_json);
            } else {
                debug!("Failed to pretty-print JSON. Likely mangled JSON.");
            }
        }
        Err(e) => {
            debug!("Cannot parse the JSON: {}", json_str);
            debug!(
                "Failed to parse JSON for debug pretty printing. Likely mangled JSON: {}",
                e
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    const JSON_OPENING: &str = "```json";
    const JSON_CLOSE: &str = "```";
    #[test]
    fn test_strip_json_markers() {
        let json_str_with_markers =
            format!("{}\n{{\"key\": \"value\"}}\n{}", JSON_OPENING, JSON_CLOSE);
        let result = strip_artifacts_from(&json_str_with_markers);
        assert_eq!(result.unwrap(), "{\"key\": \"value\"}");
    }

    #[test]
    fn test_no_markers() {
        let json_str = "{\"key\": \"value\"}";
        let result = strip_artifacts_from(json_str);
        assert_eq!(result.unwrap(), json_str);
    }

    #[test]
    fn test_invalid_json_markers() {
        let json_str_with_extra_text = "xxx\n{\"key\": \"value\"}\nyyy";
        let expected_json = "{\"key\": \"value\"}";
        let result = strip_artifacts_from(json_str_with_extra_text);
        assert_eq!(result.unwrap(), expected_json);
    }

    #[test]
    fn test_create_timestamped_filename() {
        let base_path = PathBuf::from("/some/path");
        let file_extension = "txt";
        let mock_time = Local.with_ymd_and_hms(2022, 4, 1, 12, 30, 45).unwrap();

        let result = create_timestamped_filename(&base_path, file_extension, mock_time);

        // Test that the result is in the correct directory
        assert_eq!(result.parent(), Some(base_path.as_path()));

        // Test the file extension
        assert_eq!(
            result.extension(),
            Some(std::ffi::OsStr::new(file_extension))
        );

        // Test the structure and correctness of the filename
        let expected_filename = format!("20220401_123045.{}", file_extension);
        assert_eq!(
            result.file_name().unwrap().to_str().unwrap(),
            &expected_filename
        );
    }

    #[test]
    fn test_normal_directory_path() {
        let path_str = "/location/dirname/cosmonaut-code";
        assert_eq!(extract_directory_name(path_str).unwrap(), "cosmonaut-code");
    }

    #[test]
    fn test_empty_path() {
        let path_str = "";
        assert!(extract_directory_name(path_str).is_err());
    }

    #[test]
    fn test_path_ending_with_slash() {
        let path_str = "/location/dirname/cosmonaut-code/";
        assert_eq!(extract_directory_name(path_str).unwrap(), "cosmonaut-code");
    }

    #[test]
    fn test_single_name_directory() {
        let path_str = "cosmonaut-code";
        assert_eq!(extract_directory_name(path_str).unwrap(), "cosmonaut-code");
    }
}
