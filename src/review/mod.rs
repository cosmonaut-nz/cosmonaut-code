//! Handles the review of a software repository. This is the most significant module in the application.
//! Iterates over the folder structure, ignoring files or folders that are not relevant.
//! Assesses the repository structure and file types to determine the predominant code language.
//! Passes each relevant (code) file for review.
//! Applies rules to the findings to produce a human readable summary and (set of) RAG statuses.
//! Produces a human readable report.
mod code;
pub(crate) mod data;
pub(crate) mod report;
mod tools;
use crate::provider::api::ProviderCompletionResponse;
use crate::provider::prompts::PromptData;
use crate::provider::{review_or_summarise, RequestType};
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
    // Collect the review data in the following data struct
    let mut review: RepositoryReview = RepositoryReview::new();

    match extract_repository_name(&settings.repository_path) {
        Ok(dir_name) => review.repository_name(dir_name.to_string()),
        Err(e) => return Err(Box::new(e)),
    };
    let repository_root = match validate_repository(Path::new(&settings.repository_path)) {
        Ok(path) => path,
        Err(e) => return Err(Box::new(e)),
    };
    let repository_root_pathbuf = repository_root.to_path_buf();
    info!("Reviewing repository: {}", review.repository_name);

    // TODO:    Lookup whether there is a README / README.md / README.rs / Readme.txt (and variations)
    //          If there is extract to a string and pass to LLM for summarising as RepositoryReview.repository_purpose

    let blacklisted_dirs: Vec<String> = tools::get_blacklist_dirs(repository_root);

    let mut overall_file_count: i32 = 0;
    let (lc, mut breakdown, rules, docs) = initialize_language_analysis();

    let mut review_breakdown = ReviewBreakdown {
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
        documentation: None, // TODO: to implement the review of the state of documentation, etc.
    };

    // Fetch files from non-blacklisted dirs (that are not symlinks)
    for entry in WalkDir::new(repository_root)
        .into_iter()
        .filter_entry(|e| is_not_blacklisted(e, &blacklisted_dirs) && !e.file_type().is_symlink())
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        // This holds hard stats on the file, note that the LLM does attempt to fill in some of this, but often gets it wrong.
        let result: Option<FileInfo> =
            get_file_info(&entry, &repository_root_pathbuf).and_then(|file_info| {
                analyse_file_language(&file_info, &lc, &rules, &docs).map(
                    |(language, file_size, loc)| FileInfo {
                        contents: file_info.contents,
                        name: file_info.name,
                        ext: file_info.ext,
                        language: Some(language),
                        file_size: Some(file_size),
                        loc: Some(loc),
                    },
                )
            });

        if let Some(file_info) = result {
            overall_file_count += 1;
            breakdown.add_usage(
                &file_info.language.clone().unwrap().name,
                file_info.ext.to_str().unwrap_or_default(),
                file_info.file_size.unwrap(),
                file_info.loc.unwrap(),
            );

            #[cfg(debug_assertions)]
            if settings.is_developer_mode() {
                if let Some(max_count) = settings.developer_mode.as_ref().unwrap().max_file_count {
                    // To make JSON config easier, a negative max_file_count skips
                    if max_count >= 0 && overall_file_count > max_count {
                        continue;
                    }
                }
            }

            let contents_str = match file_info.contents.to_str() {
                Some(contents) => contents,
                None => {
                    error!(
                        "Contents of the code file, {:?}, are not valid UTF-8, skipping.",
                        entry.file_name()
                    );
                    continue;
                }
            };
            let file_name_str = match file_info.name.to_str() {
                Some(name) => name,
                None => {
                    error!(
                        "File name, {:?}, is not valid UTF-8, skipping.",
                        entry.file_name()
                    );
                    continue;
                }
            };
            match review_file(
                &settings,
                &file_name_str.to_string(),
                &contents_str.to_string(),
            )
            .await
            {
                Ok(Some(mut reviewed_file)) => {
                    review_breakdown.errors +=
                        reviewed_file.errors.as_ref().map_or(0, Vec::len) as i32;
                    review_breakdown.improvements +=
                        reviewed_file.improvements.as_ref().map_or(0, Vec::len) as i32;

                    if let Some(issues) = &reviewed_file.security_issues {
                        for issue in issues {
                            review_breakdown.security_issues.total += 1;
                            match issue.severity {
                                Severity::Low => review_breakdown.security_issues.low += 1,
                                Severity::Medium => review_breakdown.security_issues.medium += 1,
                                Severity::High => review_breakdown.security_issues.high += 1,
                                Severity::Critical => {
                                    review_breakdown.security_issues.critical += 1
                                }
                            }
                        }
                    }
                    review_breakdown.summary.push_str(&reviewed_file.summary);
                    review_breakdown.summary.push('\n');

                    let file_statistics = LanguageFileType {
                        language: file_info
                            .language
                            .as_ref()
                            .map_or(String::new(), |lang| lang.to_string()),
                        extension: file_info.ext.to_string_lossy().into_owned(),
                        percentage: 0.0,
                        loc: file_info.loc.unwrap_or(0),
                        total_size: file_info.file_size.unwrap_or(0),
                        file_count: 1,
                    };
                    reviewed_file.statistics = Some(file_statistics);
                    reviewed_file.file_rag_status =
                        calculate_rag_status_for_reviewed_file(&reviewed_file).unwrap_or_default();

                    review.add_file_review(reviewed_file);
                }
                Ok(None) => {
                    warn!("No review actioned. None returned from 'review_file'")
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }
    }
    if !review.file_reviews.is_empty() {
        match summarise_review_breakdown(&settings, &review_breakdown).await {
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
    let now_utc: DateTime<Utc> = Utc::now();
    let now_local = now_utc.with_timezone(&Local);
    let review_date = now_local.format("%H:%M, %d/%m/%Y").to_string();

    // Complete the fields in the [`RepositoryReview`] struct
    if let Some(language) =
        LanguageFileType::get_predominant_language(&breakdown.to_language_file_types())
    {
        review.repository_type(Some(language));
    } else {
        review.repository_type(Some("UNKNOWN".to_string()));
    }
    review.date(review_date);
    review.repository_purpose(None); // TODO: Derive this from summarising the README by the LLM (not in MVP)
    review.summary(Some(review_breakdown));
    review.repository_rag_status(get_overall_rag_for(&review));
    review.sum_num_files(Some(overall_file_count));
    review.sum_loc(Some(LanguageFileType::sum_lines_of_code(
        &breakdown.to_language_file_types(),
    )));
    review.contributors(get_git_contributors(&settings.repository_path));
    review.language_file_types(breakdown.to_language_file_types());
    let provider: &ProviderSettings = get_provider(&settings);
    review.generative_ai_service_and_model(Some(format!(
        "provider: {}, service: {}, model: {}",
        provider.name, provider.service, provider.model
    )));

    let review_path = match create_report(&settings, &review) {
        Ok(review_path) => review_path,
        Err(e) => return Err(e),
    };

    info!("TOTAL NUMBER OF FILES PROCESSED: {}", overall_file_count);
    Ok(review_path)
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
    let provider: &ProviderSettings = get_provider(settings);
    let mut prompt_data: PromptData = match settings.review_type {
        ReviewType::General => PromptData::get_code_review_prompt(),
        ReviewType::Security => PromptData::get_security_review_prompt(),
        ReviewType::CodeStats => {
            info!("CODE STATISTICS ONLY. Only running code statistics, no review run.");
            return Ok(None);
        }
    };
    let review_request: String = format!("File name: {}\n{}\n", code_file_path, code_file_contents);
    prompt_data.add_user_message_prompt(review_request);

    let mut attempts = 0;
    let max_retries = provider.max_retries.unwrap_or(0);
    // TODO sooo... deep with the nesting! Please fix me.
    loop {
        let response_result: Result<ProviderCompletionResponse, Box<dyn Error>> =
            review_or_summarise(RequestType::Review, settings, provider, &prompt_data).await;
        match response_result {
            Ok(response) => {
                let orig_response_json = response.choices[0].message.content.to_string();
                match strip_artifacts_from(&orig_response_json) {
                    Ok(stripped_json) => match data::deserialize_file_review(&stripped_json) {
                        Ok(filereview_from_json) => return Ok(Some(filereview_from_json)),
                        Err(e) => {
                            error!("Failed to deserialize: {:?}, Possibly due to invalid escape character", &stripped_json);
                            if attempts >= max_retries {
                                return Err(format!(
                                    "Failed to deserialize into FileReview: {}",
                                    e
                                )
                                .into());
                            }
                        }
                    },
                    Err(e) => {
                        if attempts >= max_retries {
                            return Err(format!("Error stripping JSON markers: {}", e).into());
                        }
                    }
                }
            }
            Err(e) => {
                if attempts >= max_retries {
                    return Err(e);
                }
            }
        }

        attempts += 1;
        // TODO: Add a delay between retries
        // tokio::time::sleep(Duration::from_millis(1000)).await;
    }
}

async fn summarise_review_breakdown(
    settings: &Settings,
    review_breakdown: &ReviewBreakdown,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    info!("Creating repository summary statement");

    let provider: &ProviderSettings = get_provider(settings);
    let mut prompt_data = PromptData::get_overall_summary_prompt();

    debug!("Input review summaries: {}", review_breakdown.summary);

    let summary_request: String = format!(
        "Concisely summarise the following: {}\n",
        review_breakdown.summary
    );
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
fn validate_repository(repository_root: &Path) -> Result<&Path, PathError> {
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
    let ext = path.extension()?.to_os_string();

    Some(FileInfo {
        contents: Arc::new(OsStr::new(&contents).to_os_string()),
        name: Arc::new(OsStr::new(name).to_os_string()),
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
