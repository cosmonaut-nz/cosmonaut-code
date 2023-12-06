//! Handles the review of a software repository. This is the most significant module in the application.
//! Iterates over the folder structure, ignoring files or folders that are not relevant.
//! Assesses the repository structure and file types to determine the predominant code language.
//! Passes each relevant (code) file for review.
//! Applies rules to the findings to produce a human readable summary and (set of) RAG statuses.
//! Produces a human readable report.
// TODO Complete refactor! The file is hard to manage, and oftentimes does not meet good DRY or SOLID principles
mod code;
pub(crate) mod data;
pub(crate) mod report;
mod tools;
use crate::provider::api::ProviderCompletionResponse;
use crate::provider::prompts::PromptData;
use crate::provider::{review_or_summarise, RequestType};
use crate::review::code::LanguageBreakdown;
use crate::review::code::{
    analyse_file_language, calculate_rag_status_for_reviewed_file, initialize_language_analysis,
    FileInfo,
};
use crate::review::data::{
    FileReview, LanguageFileType, RAGStatus, RepositoryReview, ReviewBreakdown,
    SecurityIssueBreakdown, Severity,
};
use crate::review::report::create_report;
use crate::review::tools::{get_git_contributors, is_not_blacklisted};
use crate::settings::{ProviderSettings, ReviewType, Settings};
use chrono::{DateTime, Local, Utc};
use log::{debug, error, info, warn};
use regex::Regex;
use sha2::{Digest, Sha256};
use std::error::Error;
use std::ffi::OsStr;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use walkdir::{DirEntry, WalkDir};

/// Takes the filepath to a repository and iterates over the code, gaining stats, and sending each relevant file for review.
///
/// # Parameters
///
/// * `settings` - A [`Settings`] that contains information for the LLM
///
pub(crate) async fn assess_codebase(
    settings: Settings,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut review = initialise_repository_review(&settings)?;
    let repository_root = validate_repository(PathBuf::from(&settings.repository_path))?;

    let provider: &ProviderSettings = get_provider(&settings);
    review.generative_ai_service_and_model(Some(format!(
        "provider: {}, service: {}, model: {}",
        provider.name, provider.service, provider.model
    )));
    info!(
        "Reviewing: {}, with {}",
        review.repository_name,
        review.generative_ai_service_and_model.clone().unwrap()
    );

    let blacklisted_dirs = tools::get_blacklist_dirs(&repository_root);

    let (lc, mut breakdown, rules, docs) = initialize_language_analysis();
    let mut review_breakdown: ReviewBreakdown = initialise_review_breakdown();

    let mut overall_file_count = 0;

    for entry in get_files_from_repository(&repository_root, &blacklisted_dirs) {
        let result: Option<FileInfo> =
            get_file_info(&entry, &repository_root).and_then(|file_info| {
                analyse_file_language(&file_info, &lc, &rules, &docs).map(
                    |(language, file_size, loc)| FileInfo {
                        contents: file_info.contents,
                        name: file_info.name,
                        id_hash: file_info.id_hash,
                        ext: file_info.ext,
                        language: Some(language),
                        file_size: Some(file_size),
                        loc: Some(loc),
                    },
                )
            });

        #[cfg(debug_assertions)]
        if settings.is_developer_mode() {
            if let Some(max_count) = settings.developer_mode.as_ref().unwrap().max_file_count {
                if max_count >= 0 && overall_file_count >= max_count {
                    continue;
                }
            }
        }

        if let Some(file_info) = result {
            overall_file_count += 1;
            update_language_breakdown(&mut breakdown, &file_info);

            if let Some(file_name_str) = file_info.name.to_str() {
                if let Some(contents_str) = file_info.contents.to_str() {
                    match review_file(
                        &settings,
                        &file_name_str.to_string(),
                        &contents_str.to_string(),
                    )
                    .await
                    {
                        Ok(Some(mut reviewed_file)) => {
                            update_review_breakdown(
                                &mut review_breakdown,
                                &mut reviewed_file,
                                &file_info,
                            );
                            review.add_file_review(reviewed_file);
                        }
                        Ok(None) => warn!("No review actioned. None returned from 'review_file'"),
                        Err(e) => return Err(e),
                    }
                } else {
                    error!(
                        "Contents of the code file, {:?}, are not valid UTF-8, skipping.",
                        entry.file_name()
                    );
                }
            } else {
                error!(
                    "File name, {:?}, is not valid UTF-8, skipping.",
                    entry.file_name()
                );
            }
        }
    }

    match finalise_review(
        &mut review,
        overall_file_count,
        &mut review_breakdown,
        &breakdown,
        &settings,
    )
    .await
    {
        Ok(_) => (),
        Err(e) => return Err(e),
    };

    create_report(&settings, &review)
}

/// Initialises a new [`RepositoryReview`] according to the configured name from path
fn initialise_repository_review(
    settings: &Settings,
) -> Result<RepositoryReview, Box<dyn std::error::Error>> {
    let repository_name = extract_repository_name(&settings.repository_path)
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    Ok(RepositoryReview::new(repository_name.to_string()))
}
/// gets files from non-blacklisted dirs (that are not symlinks)
fn get_files_from_repository(
    repository_root: &PathBuf,
    blacklisted_dirs: &[String],
) -> Vec<DirEntry> {
    WalkDir::new(repository_root)
        .into_iter()
        .filter_entry(|e| is_not_blacklisted(e, blacklisted_dirs) && !e.file_type().is_symlink())
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .collect()
}
/// TODO: to implement the review of the state of documentation, etc.
fn initialise_review_breakdown() -> ReviewBreakdown {
    ReviewBreakdown {
        summary: String::new(),
        security_issues: SecurityIssueBreakdown {
            low: 0,
            medium: 0,
            high: 0,
            critical: 0,
            total: 0,
        },
        errors: 0,
        improvements: 0,
        documentation: None,
    }
}
///
fn update_language_breakdown(breakdown: &mut LanguageBreakdown, file_info: &FileInfo) {
    if let Some(language) = &file_info.language {
        let language_name = language.name.clone();
        let file_extension = file_info.ext.to_str().unwrap_or_default().to_string();
        let file_size = file_info.file_size.unwrap_or(0);
        let loc = file_info.loc.unwrap_or(0);
        breakdown.add_usage(&language_name, &file_extension, file_size, loc);
    }
}

fn update_review_breakdown(
    review_breakdown: &mut ReviewBreakdown,
    reviewed_file: &mut FileReview,
    file_info: &FileInfo,
) {
    review_breakdown.errors += reviewed_file.errors.as_ref().map_or(0, Vec::len) as i32;
    review_breakdown.improvements += reviewed_file.improvements.as_ref().map_or(0, Vec::len) as i32;

    if let Some(issues) = &reviewed_file.security_issues {
        for issue in issues {
            review_breakdown.security_issues.total += 1;
            match issue.severity {
                Severity::Low => review_breakdown.security_issues.low += 1,
                Severity::Medium => review_breakdown.security_issues.medium += 1,
                Severity::High => review_breakdown.security_issues.high += 1,
                Severity::Critical => review_breakdown.security_issues.critical += 1,
            }
        }
    }
    review_breakdown.summary.push_str(&reviewed_file.summary);
    review_breakdown.summary.push('\n');

    let file_statistics = LanguageFileType {
        language: file_info
            .language
            .as_ref()
            .map_or(Some(String::new()), |lang| Some(lang.name.clone())),
        extension: Some(file_info.ext.to_string_lossy().into_owned()),
        percentage: Some(0.0),
        loc: Some(file_info.loc.unwrap_or(0)),
        total_size: Some(file_info.file_size.unwrap_or(0)),
        file_count: Some(1),
    };
    reviewed_file.statistics = Some(file_statistics);
    reviewed_file.file_rag_status =
        calculate_rag_status_for_reviewed_file(reviewed_file).unwrap_or_default();
    reviewed_file.id_hash = Some(file_info.id_hash.to_string_lossy().into_owned());
}

async fn finalise_review(
    review: &mut RepositoryReview,
    overall_file_count: i32,
    review_breakdown: &mut ReviewBreakdown,
    breakdown: &LanguageBreakdown,
    settings: &Settings,
) -> Result<(), Box<dyn std::error::Error>> {
    if !review.file_reviews.is_empty() {
        match summarise_review_breakdown(settings, review_breakdown).await {
            Ok(Some(summary)) => {
                review_breakdown.summary = summary;
            }
            Ok(None) => {
                warn!("Summary response was returned as 'None'!");
                review_breakdown.summary = String::new();
            }
            Err(e) => return Err(e),
        };
    }
    review.summary(Some(review_breakdown.clone()));

    let predominant_language =
        LanguageFileType::get_predominant_language(&breakdown.to_language_file_types())
            .unwrap_or_else(|| "UNKNOWN".to_string());
    review.repository_type(Some(predominant_language));

    // Date stamp the review
    let now_utc: DateTime<Utc> = Utc::now();
    let now_local = now_utc.with_timezone(&Local);
    let review_date = now_local.format("%H:%M, %d/%m/%Y").to_string();
    review.date(review_date);

    review.repository_purpose(None); // TODO Implement if required
    review.repository_rag_status(get_overall_rag_for(review));

    review.sum_num_files(Some(overall_file_count));
    review.sum_loc(Some(LanguageFileType::sum_lines_of_code(
        &breakdown.to_language_file_types(),
    )));
    review.contributors(get_git_contributors(&settings.repository_path));
    review.language_file_types(breakdown.to_language_file_types());

    Ok(())
}

/// Takes the file contents of a file and sends it to the LLM for review
///
/// # Parameters
///
/// * `settings` - A [`Settings`] that contains information for the LLM
/// * `code_file_path` - The path (as [`String`]) of the file to process
/// * `code_file_contents` - The contents (as [`String`]) of the file to process
///
/// # Returns
///
/// * [`FileReview`]
///
async fn review_file(
    settings: &Settings,
    code_file_path: &String,
    code_file_contents: &String,
) -> Result<Option<FileReview>, Box<dyn std::error::Error>> {
    info!("Reviewing file: {}", code_file_path);

    if let Some(mut prompt_data) = get_prompt_data_based_on_review_type(settings)? {
        let provider: &ProviderSettings = get_provider(settings);
        let review_request: String =
            format!("File name: {}\n{}\n", code_file_path, code_file_contents);

        prompt_data.add_user_message_prompt(review_request);
        perform_review(settings, provider, &prompt_data).await
    } else {
        Ok(None)
    }
}
/// Fetches the correct [`PromptData`] according to the [`ReviewType`] passed
fn get_prompt_data_based_on_review_type(
    settings: &Settings,
) -> Result<Option<PromptData>, Box<dyn std::error::Error>> {
    match settings.review_type {
        ReviewType::General => Ok(Some(PromptData::get_code_review_prompt())),
        ReviewType::Security => Ok(Some(PromptData::get_security_review_prompt())),
        ReviewType::CodeStats => {
            info!("CODE STATISTICS ONLY. Only running code statistics, no review run.");
            Ok(None)
        }
    }
}
/// passes to the LLM the required review via a preconfigured [`PromptData`]
async fn perform_review(
    settings: &Settings,
    provider: &ProviderSettings,
    prompt_data: &PromptData,
) -> Result<Option<FileReview>, Box<dyn std::error::Error>> {
    let max_retries = provider.max_retries.unwrap_or(0);
    let mut attempts = 0;

    loop {
        match review_or_summarise(RequestType::Review, settings, provider, prompt_data).await {
            Ok(response) => match process_response(&response) {
                Ok(file_review) => return Ok(Some(file_review)),
                Err(e) if attempts < max_retries => {
                    error!("Error processing response: {}", e);
                    attempts += 1;
                }
                Err(e) => return Err(e),
            },
            Err(e) if attempts < max_retries => {
                error!("Error in review: {}", e);
                attempts += 1;
            }
            Err(e) => return Err(e),
        }
    }
}
/// processes the response returned by the LLM, stripping any artefacts, or illegal chars, then loading the JSON into a [`FileReview`]
fn process_response(
    response: &ProviderCompletionResponse,
) -> Result<FileReview, Box<dyn std::error::Error>> {
    let orig_response_json = response.choices[0].message.content.to_string();

    strip_artifacts_from(&orig_response_json)
        .map_err(|e| {
            Box::new(std::io::Error::new(std::io::ErrorKind::Other, e))
                as Box<dyn std::error::Error>
        })
        .and_then(|stripped_json| {
            data::deserialize_file_review(&stripped_json).map_err(|e| {
                error!(
                    "Failed to deserialize: {:?}, Possibly due to invalid escape character",
                    &stripped_json
                );
                Box::new(e) as Box<dyn std::error::Error>
            })
        })
}
/// ask the LLM to summarise to whole set of [`FileReview`] summaries into a single repository summary
pub(crate) async fn summarise_review_breakdown(
    settings: &Settings,
    review_breakdown: &ReviewBreakdown,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    info!("Creating repository summary statement");

    let provider: &ProviderSettings = get_provider(settings);
    let mut prompt_data: PromptData = PromptData::get_overall_summary_prompt();

    debug!("Input review summaries: {}", review_breakdown.summary);

    let summary_request: String = review_breakdown.summary.to_string();
    prompt_data.add_user_message_prompt(summary_request);

    let response_result: Result<ProviderCompletionResponse, Box<dyn Error>> =
        review_or_summarise(RequestType::Summarise, settings, provider, &prompt_data).await;
    match response_result {
        Ok(response) => Ok(Some(
            response.choices[0]
                .message
                .content
                .to_string()
                .replace(" - ", "\n"),
        )),
        Err(e) => Err(e),
    }
}
/// validates the provided [`Path`] as being a directory that holds a '.git' subdirectory - i.e. is a valid git repository
fn validate_repository(repository_root: PathBuf) -> Result<PathBuf, PathError> {
    if !repository_root.is_dir() {
        return Err(PathError {
            message: format!(
                "Provided path is not a directory: {}",
                repository_root.display()
            ),
        });
    }
    if !repository_root.join(".git").is_dir() {
        return Err(PathError {
            message: format!(
                "Provided path is not a valid Git repository: {}",
                repository_root.display()
            ),
        });
    }

    Ok(repository_root)
}
/// gets the content, filename and extension of a [`walkdir::DirEntry`]
fn get_file_info(entry: &DirEntry, repo_root: &PathBuf) -> Option<FileInfo> {
    let path = entry.path();

    // Calculate the relative path from the repository root
    let relative_path = path.strip_prefix(repo_root).ok()?.to_path_buf();

    let contents = fs::read_to_string(path).ok()?;
    let name = relative_path.to_str()?;
    let id_hash = calculate_hash(&contents);
    let ext = path.extension()?.to_os_string();

    Some(FileInfo {
        contents: Arc::new(OsStr::new(&contents).to_os_string()),
        name: Arc::new(OsStr::new(name).to_os_string()),
        id_hash: Arc::new(OsStr::new(&id_hash).to_os_string()),
        ext: Arc::new(ext),
        language: None,
        file_size: None,
        loc: None,
    })
}
/// gives an overall [`RAGStatus`] for the passed [`RepositoryReview`]
fn get_overall_rag_for(review: &RepositoryReview) -> RAGStatus {
    if let Some(breakdown) = &review.summary {
        let num_total_files = review.file_reviews.len() as i32;

        if breakdown.security_issues.high > 0 || breakdown.security_issues.critical > 0 {
            return RAGStatus::Red;
        }

        let security_issues_ratio = breakdown.security_issues.total as f64 / num_total_files as f64;
        let errors_ratio = breakdown.errors as f64 / num_total_files as f64;
        let improvements_ratio = breakdown.improvements as f64 / num_total_files as f64;

        if security_issues_ratio > 0.05 || errors_ratio > 0.08 || improvements_ratio > 0.60 {
            return RAGStatus::Amber;
        }
    }

    RAGStatus::Green
}
// Gets the currently active provider. If there is a misconfiguration (i.e., a mangled `default.json`) then panics
fn get_provider(settings: &Settings) -> &crate::settings::ProviderSettings {
    let provider: &crate::settings::ProviderSettings = settings.get_active_provider()
                                              .expect("Either a default or chosen provider should be configured in \'default.json\'. \
                                              Either none was found, or the default provider did not match any name in the configured providers list.");
    provider
}
/// Gets the actual repository name from the path
fn extract_repository_name(path_str: &str) -> Result<&str, PathError> {
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
/// Calculates a hash from a string
fn calculate_hash(data: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();

    format!("{:x}", result)
}
/// Removes any artefacts from an AI review
///
/// In some cases the AI agent add in markdown annotation for the content type,
/// e.g., openai adds "\`\`\`json" at the beginning, and "\`\`\`" at the end of response to mark the type of content
/// In others, spurious control characters are added that mangles the JSON for deserializing, e.g. characters in the range U+0000 to U+001F
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
fn _pretty_print_json_for_debug(json_str: &str) {
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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_normal_directory_path() {
        let path_str = "/location/dirname/cosmonaut-code";
        assert_eq!(extract_repository_name(path_str).unwrap(), "cosmonaut-code");
    }

    #[test]
    fn test_empty_path() {
        let path_str = "";
        assert!(extract_repository_name(path_str).is_err());
    }

    #[test]
    fn test_path_ending_with_slash() {
        let path_str = "/location/dirname/cosmonaut-code/";
        assert_eq!(extract_repository_name(path_str).unwrap(), "cosmonaut-code");
    }

    #[test]
    fn test_single_name_directory() {
        let path_str = "cosmonaut-code";
        assert_eq!(extract_repository_name(path_str).unwrap(), "cosmonaut-code");
    }
}
