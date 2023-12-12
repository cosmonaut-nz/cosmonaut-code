//! Handles the review of a software repository. This is the most significant module in the application.
//! Iterates over the folder structure, ignoring files or folders that are not relevant.
//! Assesses the repository structure and file types to determine the predominant code language.
//! Passes each relevant (code) file for review.
//! Applies rules to the findings to produce a human readable summary and (set of) RAG statuses.
//! Produces a human readable report.
// TODO Complete refactor! The file is hard to manage, and oftentimes does not meet DRY or SOLID principles
//      refactor extract non-review aspects into other modules.
pub(crate) mod data;
pub(crate) mod report;
use crate::provider::api::ProviderCompletionResponse;
use crate::provider::prompts::PromptData;
use crate::provider::{get_provider, get_service_and_model, review_or_summarise, RequestType};
use crate::retrieval::code::{
    analyse_file_language, calculate_hash_from, calculate_rag_status_for_reviewed_file,
};
use crate::retrieval::data::{LanguageType, SourceFileInfo, Statistics};
use crate::retrieval::git::repository::{get_blacklist_dirs, get_total_commits};
use crate::retrieval::git::source_file::get_source_file_change_frequency;
use crate::retrieval::git::{contributor::get_git_contributors, repository::is_not_blacklisted};
use crate::review::data::{
    RAGStatus, RepositoryReview, ReviewSummary, SecurityIssueBreakdown, Severity, SourceFileReview,
};
use crate::review::report::create_report;
use crate::settings::{ProviderSettings, ReviewType, Settings};
use chrono::{DateTime, Local, Utc};
use log::{debug, error, info, warn};
use regex::Regex;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::{fmt, fs};
use walkdir::{DirEntry, WalkDir};

/// Takes the filepath to a repository and iterates over the code, gaining stats, and sending each relevant file for review.
///
/// # Parameters
///
/// * `settings` - A [`Settings`] that contains information for the LLM
///
// TODO: Heavy refactor. Re-assess and re-implement, first via heavy commentary of what I should be doing, which is represented by the 'RepositoryReview' struct
pub(crate) async fn assess_codebase(
    settings: Settings,
) -> Result<String, Box<dyn std::error::Error>> {
    // Check whether this a valid git repository
    let repository_root: PathBuf = validate_repository(PathBuf::from(&settings.repository_path))?;

    // Initialise the RepositoryReview data struct
    let mut review: RepositoryReview = initialise_repository_review(&settings)?;

    // Add the service and model to the RepositoryReview
    review.generative_ai_service_and_model(get_service_and_model(&settings));

    info!(
        "Reviewing: {}, with {}",
        review.repository_name,
        review.generative_ai_service_and_model.clone().unwrap()
    );
    // Initialise the RepositoryReview::ReviewSummary
    let mut review_summary_section: ReviewSummary = initialise_review_summary_section();

    // Initialise a file count that we will use to show how many files were processed
    let mut overall_processed_files = 0;

    // The RepositoryReview has a Vec<LanguageTypes>, initialise
    let mut lang_type_breakdown: Vec<LanguageType> = Vec::new();

    // The review of source files begins.
    // Iterate over the files in the repository that are not blacklisted
    for entry in valid_files_from_repository(&repository_root) {
        #[cfg(debug_assertions)]
        if settings.is_developer_mode() {
            if let Some(max_count) = settings.developer_mode.as_ref().unwrap().max_file_count {
                if max_count >= 0 && overall_processed_files >= max_count {
                    continue;
                }
            }
        }

        let result: Option<SourceFileInfo> =
            // Get the file info, including the file contents
            get_initial_source_file_info(&entry, &repository_root);

        if let Some(file_info) = result {
            overall_processed_files += 1;

            // Add the LanguageType to the Vec<LanguageType>
            update_language_type_statistics(&mut lang_type_breakdown, &file_info);

            let file_name_str = file_info.name.clone();
            let contents_str = file_info.get_source_file_contents();
            // Actually review the file via the LLM, returns a SourceFileReview
            match review_file(
                &settings,
                &file_name_str.to_string(),
                &contents_str.to_string(),
            )
            .await
            {
                Ok(Some(mut reviewed_file)) => {
                    update_repository_review_statistics(&mut review, &file_info);

                    reviewed_file.source_file_info = file_info.clone();
                    update_review_summary(&mut review_summary_section, &mut reviewed_file);

                    // Add SourceFileReview to the RepositoryReview
                    review.add_source_file_review(reviewed_file);
                }
                Ok(None) => warn!("No review actioned. None returned from 'review_file'"),
                Err(e) => return Err(e),
            }
        }
    } // end get_files_from_repository

    finalise_review(
        &mut review,
        &mut review_summary_section,
        &mut lang_type_breakdown,
        &settings,
    )
    .await?;

    // Should be good to go now, so create the report
    create_report(&settings, &review)
}

/// Updates the [`RepositoryReview`] statistics per [`SourceFileInfo`] processed
fn update_repository_review_statistics(review: &mut RepositoryReview, file_info: &SourceFileInfo) {
    review.statistics.size += file_info.statistics.size;
    review.statistics.loc += file_info.statistics.loc;
    review.statistics.num_files += 1;
}
/// Updates the language type statistics, adding a new one if it doesn't exist
/// Note that the [`LanguageType`].statistics.frequency is not updated here, but at the end
fn update_language_type_statistics(
    lang_type_breakdown: &mut Vec<LanguageType>,
    file_info: &SourceFileInfo,
) {
    match lang_type_breakdown
        .iter()
        .position(|lang| lang.name == file_info.language.name)
    {
        Some(index) => {
            let language_stats = lang_type_breakdown
                .get_mut(index)
                .and_then(|lang| lang.statistics.as_mut());
            if let Some(stats) = language_stats {
                stats.size += file_info.statistics.size;
                stats.loc += file_info.statistics.loc;
                stats.num_files += 1;
            }
        }
        None => {
            let mut new_lang_type = file_info.language.clone();
            new_lang_type.statistics = Some(file_info.statistics.clone());
            lang_type_breakdown.push(new_lang_type);
        }
    }
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
fn valid_files_from_repository(repository_root: &PathBuf) -> Vec<DirEntry> {
    let blacklisted_dirs = get_blacklist_dirs(repository_root);
    WalkDir::new(repository_root)
        .into_iter()
        .filter_entry(|e| is_not_blacklisted(e, &blacklisted_dirs) && !e.file_type().is_symlink())
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .collect()
}
/// TODO: to implement the review of the state of documentation, etc.
fn initialise_review_summary_section() -> ReviewSummary {
    ReviewSummary {
        text: String::new(),
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

/// Updates the [`ReviewSummary`] with the results of the [`SourceFileReview`]
fn update_review_summary(review_summary: &mut ReviewSummary, reviewed_file: &mut SourceFileReview) {
    review_summary.errors += reviewed_file.errors.as_ref().map_or(0, Vec::len) as i32;
    review_summary.improvements += reviewed_file.improvements.as_ref().map_or(0, Vec::len) as i32;

    if let Some(issues) = &reviewed_file.security_issues {
        for issue in issues {
            review_summary.security_issues.total += 1;
            match issue.severity {
                Severity::Low => review_summary.security_issues.low += 1,
                Severity::Medium => review_summary.security_issues.medium += 1,
                Severity::High => review_summary.security_issues.high += 1,
                Severity::Critical => review_summary.security_issues.critical += 1,
            }
        }
    }
    review_summary.text.push_str(&reviewed_file.summary);
    review_summary.text.push('\n');

    reviewed_file.file_rag_status =
        calculate_rag_status_for_reviewed_file(reviewed_file).unwrap_or_default();
}

/// Finalise the [`RepositoryReview`] by adding the [`ReviewSummary`], Vec<LanguageType>, and other data
async fn finalise_review(
    review: &mut RepositoryReview,
    review_summary: &mut ReviewSummary,
    breakdown: &mut [LanguageType],
    settings: &Settings,
) -> Result<(), Box<dyn std::error::Error>> {
    if !review.file_reviews.is_empty() {
        match summarise_review_summaries(settings, review_summary).await {
            Ok(Some(summary)) => {
                review_summary.text = summary;
            }
            Ok(None) => {
                warn!("Summary response was returned as 'None'!");
                review_summary.text = String::new();
            }
            Err(e) => return Err(e),
        };
    }
    review.summary(Some(review_summary.clone()));

    // Handle the statistics for the language types
    LanguageType::calculate_percentage_distribution(breakdown);
    let predominant_language: String = LanguageType::get_predominant_language(breakdown);
    review.repository_type(Some(predominant_language));

    review.date(get_review_date());
    review.repository_purpose(None); // TODO Implement this and incorporate the documentation status
    review.repository_rag_status(get_overall_rag_for(review));
    review.statistics.num_commits = get_total_commits(&settings.repository_path)?;
    review.contributors(get_git_contributors(&settings.repository_path));
    review.language_types(breakdown.to_vec());

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
/// * [`SourceFileReview`]
///
async fn review_file(
    settings: &Settings,
    code_file_path: &String,
    code_file_contents: &String,
) -> Result<Option<SourceFileReview>, Box<dyn std::error::Error>> {
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
        ReviewType::General => PromptData::get_code_review_prompt().map(Some),
        ReviewType::Security => PromptData::get_security_review_prompt().map(Some),
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
) -> Result<Option<SourceFileReview>, Box<dyn std::error::Error>> {
    let max_retries = provider.max_retries.unwrap_or(0);
    let mut attempts = 0;

    loop {
        match review_or_summarise(RequestType::Review, settings, provider, prompt_data).await {
            Ok(response) => match process_llm_response(&response) {
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
/// processes the response returned by the LLM, stripping any artefacts, or illegal chars, then loading the JSON into a [`SourceFileReview`]
fn process_llm_response(
    response: &ProviderCompletionResponse,
) -> Result<SourceFileReview, Box<dyn std::error::Error>> {
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
/// Asks the LLM to summarise a concat text of [`SourceFileReview`] summaries (in `review_summary.text`) into a concise overall repository summary
/// # Parameters:
/// * `settings` - A [`Settings`] that contains information for the LLM
/// * `review_summary` - A [`ReviewSummary`] that contains the summaries (as text) of each [`SourceFileReview`]
pub(crate) async fn summarise_review_summaries(
    settings: &Settings,
    review_summary: &ReviewSummary,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    info!("Creating repository summary statement");

    let provider: &ProviderSettings = get_provider(settings);
    let mut prompt_data: PromptData = PromptData::get_overall_summary_prompt()?;

    debug!("Input review summaries: {}", review_summary.text);

    let summary_request: String = review_summary.text.to_string();
    prompt_data.add_user_message_prompt(summary_request);

    let response_result: Result<ProviderCompletionResponse, Box<dyn Error>> =
        review_or_summarise(RequestType::Summarise, settings, provider, &prompt_data).await;
    // TODO: provide a suitable reponse that hides the implementation details
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

/// Builds the initial [`SourceFileInfo`]
/// At his point it is not known if the file is a source file, so this is determined by the [`LanguageType`] returned from the Linguist crate
/// There is cost in doing all this work here, but the contents of the file needs to be assessed for accuracy.
///
/// # Parameters:
/// * `entry` - A [`DirEntry`] that represents the file to be assessed
/// * `repo_root` - A [`PathBuf`] that represents the root of the repository
///
/// # Returns:
/// * A [`SourceFileInfo`] if the file is a source file, otherwise None
fn get_initial_source_file_info(entry: &DirEntry, repo_root: &PathBuf) -> Option<SourceFileInfo> {
    let path = entry.path();
    let relative_path = path.strip_prefix(repo_root).ok()?.to_path_buf();

    // We need these as strings
    let file_name = path.file_name().unwrap().to_str().unwrap().to_string();
    let relative_path_str = relative_path.to_str()?.to_string();

    let contents = fs::read_to_string(path).ok()?;
    let id_hash = calculate_hash_from(&contents);
    let ext = path.extension()?.to_str()?.to_string();

    let stats: Statistics =
        get_source_file_change_frequency(repo_root.to_str()?, &relative_path_str)
            .ok()?
            .get_as_statistics();

    let language = LanguageType {
        name: String::new(), // Don't know this yet
        extension: ext,
        statistics: Some(stats.clone()),
    };
    let source_file_info: &mut SourceFileInfo = &mut SourceFileInfo::new(
        file_name,
        relative_path_str,
        language,
        id_hash,
        stats.clone(),
    );
    source_file_info.set_source_file_contents(contents);

    analyse_file_language(source_file_info).cloned()
}

/// Gets an overall [`RAGStatus`] for the passed [`RepositoryReview`]
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
/// Gets the current time and date as a string
fn get_review_date() -> String {
    // Date stamp the review
    let now_utc: DateTime<Utc> = Utc::now();
    let now_local = now_utc.with_timezone(&Local);
    let review_date = now_local.format("%H:%M, %d/%m/%Y").to_string();
    review_date
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
