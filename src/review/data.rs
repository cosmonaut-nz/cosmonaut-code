//! This file contains the data structures that will hold the data that returns from
//! the AI code review API.
//! The JSON will be written to a file to allow subsequent analysis.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{impl_builder_methods, retrieval::code::SourceFileChangeFrequency};

/// #Fields:
/// - 'repository_name': The name of the repository
/// - 'generative_ai_service_and_model': The generative AI service and model used to generate the review
/// - 'repository_type': The type of repository, e.g., Java, .Net, etc.
/// - 'date': Date of execution (formatted)
/// - 'repository_purpose': Derive from README, if present, else allow user entry in UI
/// - 'summary': A roll up of the findings generated via LLM
/// - 'repository_rag_status': In {Red, Amber, Green}
/// - 'sum_loc': Total lines of code
/// - 'sum_num_files': Total number of files
/// - 'contributors': List of contributors to the codebase from commit history
/// - 'language_file_types': The languages (as a %) found in the repository (a la GitHub)
/// - 'file_reviews': Each of the code files
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub(crate) struct RepositoryReview {
    pub(crate) repository_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) generative_ai_service_and_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    repository_type: Option<String>,
    date: String,
    repository_purpose: Option<String>,
    pub(crate) summary: Option<ReviewBreakdown>,
    repository_rag_status: RAGStatus,
    sum_loc: Option<i64>,
    sum_num_files: Option<i32>,
    sum_num_commits: Option<usize>,
    contributors: Vec<Contributor>,
    language_file_types: Vec<LanguageFileType>,
    pub(crate) file_reviews: Vec<SourceFileReview>,
}
impl RepositoryReview {
    pub(crate) fn new(repository_name: String) -> Self {
        RepositoryReview {
            repository_name,
            generative_ai_service_and_model: None,
            repository_type: None,
            date: String::new(),
            repository_purpose: None,
            summary: None,
            repository_rag_status: RAGStatus::Green,
            sum_loc: None,
            sum_num_files: None,
            sum_num_commits: None,
            contributors: Vec::new(),
            language_file_types: Vec::new(),
            file_reviews: Vec::new(),
        }
    }
    /// pushes a [`FileReview`] into the filereviews [`Vec`]
    pub(crate) fn add_file_review(&mut self, file_review: SourceFileReview) {
        self.file_reviews.push(file_review);
    }
}
impl_builder_methods!(
    RepositoryReview,
    generative_ai_service_and_model: Option<String>,
    repository_type: Option<String>,
    date: String,
    repository_purpose: Option<String>,
    summary: Option<ReviewBreakdown>,
    repository_rag_status: RAGStatus,
    sum_loc: Option<i64>,
    sum_num_files: Option<i32>,
    sum_num_commits: Option<usize>,
    contributors: Vec<Contributor>,
    language_file_types: Vec<LanguageFileType>
);
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub(crate) struct ReviewBreakdown {
    pub(crate) summary: String,
    pub(crate) security_issues: SecurityIssueBreakdown,
    pub(crate) errors: i32,
    pub(crate) improvements: i32,
    pub(crate) documentation: Option<Documentation>,
}
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub(crate) struct SecurityIssueBreakdown {
    pub(crate) low: i32,
    pub(crate) medium: i32,
    pub(crate) high: i32,
    pub(crate) critical: i32,
    pub(crate) total: i32,
}
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub(crate) enum Documentation {
    None,
    Some,
    Good,
    Excellent,
}
#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq)]
pub(crate) enum RAGStatus {
    #[default]
    Green,
    Amber,
    Red,
}
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub(crate) struct Contributor {
    name: String,
    num_commits: i32,
    last_contribution: DateTime<Utc>,
    percentage: i32,
}
impl Contributor {
    pub(crate) fn new(
        name: String,
        num_commits: i32,
        last_contribution: DateTime<Utc>,
        percentage: i32,
    ) -> Self {
        Self {
            name,
            num_commits,
            last_contribution,
            percentage,
        }
    }
}

/// Captures the LLM review of the specified file.
///
/// This struct will contain the fields passed back as JSON from the LLM.
///
/// #Fields
/// - 'filename': The name of the file to be reviewed
/// - 'summary': A summary of the findings of the review
/// - 'file_rag_status': Red = urgent attention required, Amber: some issues to be addressed, Green: code okay
/// - 'errors': a Vec of ['Error']s found in the code giving the issue and potential resolution for each
/// - 'improvements': a Vec of ['Improvement']s, giving a suggestion and example for each
/// - 'security_issues': a Vec of ['SecurityIssue']s, giving the threat and mitigation for each
/// - 'statistics': a list of statistics (e.g., lines of code, functions, methods, etc.)
///
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub(crate) struct SourceFileReview {
    pub(crate) filename: String,
    pub(crate) id_hash: Option<String>,
    pub(crate) summary: String,
    pub(crate) file_rag_status: RAGStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) security_issues: Option<Vec<SecurityIssue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) errors: Option<Vec<Error>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) improvements: Option<Vec<Improvement>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) statistics: Option<LanguageFileType>, // TODO: Change to SourceFileStatistics struct
}
impl SourceFileReview {
    #[allow(dead_code)]
    pub(crate) fn get_improvements(&self) -> &Option<Vec<Improvement>> {
        &self.improvements
    }
    pub(crate) fn get_errors(&self) -> &Option<Vec<Error>> {
        &self.errors
    }
    pub(crate) fn get_security_issues(&self) -> &Option<Vec<SecurityIssue>> {
        &self.security_issues
    }
    pub(crate) fn get_file_rag_status(&self) -> &RAGStatus {
        &self.file_rag_status
    }
}
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub(crate) struct SecurityIssue {
    pub(crate) severity: Severity,
    pub(crate) code: String,
    pub(crate) threat: String,
    pub(crate) mitigation: String,
}
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub(crate) struct Error {
    code: String,
    issue: String,
    resolution: String,
}
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub(crate) struct Improvement {
    code: String,
    suggestion: String,
    improvement_details: String,
}
/// Severity of the security issue as per CVSS v3.1
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub(crate) enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

/// Top-level struct to hold data on the file types found in the repository
/// #Fields:
/// - 'language': The language of the file
/// - 'extension': The extension of the file
/// - 'percentage': The percentage of the language in the repository
/// - 'loc': The lines of code for the language
/// - 'total_size': The total size of the source file in bytes
/// - 'file_commits': The number of commits for the file
/// - 'frequency': The % frequency of change for the file - i.e. how often the file is included in overall commits
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub(crate) struct LanguageFileType {
    pub(crate) language: Option<String>,
    pub(crate) extension: Option<String>,
    pub(crate) percentage: Option<f64>,
    pub(crate) loc: Option<i64>,
    pub(crate) total_size: Option<u64>,
    pub(crate) file_count: Option<i32>,
    pub(crate) file_change_frequency: Option<SourceFileChangeFrequency>,
}
impl LanguageFileType {
    // Method to check if an extension is valid among a collection of LanguageFileType
    pub(crate) fn _has_extension_of(ext: &str, file_types: &[LanguageFileType]) -> bool {
        file_types
            .iter()
            .any(|lft| lft.extension == Some(ext.to_string()))
    }
    pub(crate) fn sum_lines_of_code(language_file_types: &[LanguageFileType]) -> i64 {
        language_file_types.iter().map(|lft| lft.loc.unwrap()).sum()
    }
    pub(crate) fn get_predominant_language(languages: &[LanguageFileType]) -> Option<String> {
        let mut predominant_language = None;
        let mut highest_percentage = None;
        let mut largest_size = None;

        for lang in languages {
            if lang.percentage > highest_percentage
                || (lang.percentage == highest_percentage && lang.total_size > largest_size)
            {
                highest_percentage = lang.percentage;
                largest_size = lang.total_size;
                predominant_language = lang.language.clone();
            }
        }

        predominant_language
    }
    /// Used in the HTML template
    #[allow(dead_code)]
    pub(crate) fn formatted_percentage(&self) -> String {
        format!("{:.2}", self.percentage.unwrap())
    }
}

/// Deserializes a str into a [`SourceFileReview`] struct.
///
/// # Parameters
///
/// * `json_str` - A str representation of the JSON to be deserialized
///
/// # Returns
///
/// * A [`SourceFileReview`] struct
///
pub(crate) fn deserialize_file_review(
    json_str: &str,
) -> Result<SourceFileReview, serde_json::Error> {
    serde_json::from_str(json_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_file_review() {
        let str_json = r#"{
                    "filename": "src/provider/static_config.rs",
                    "id_hash": "",
                    "summary": "The code defines a constant for a request timeout without any visible issues or security threats. However, the usage of 'pub(crate) (crate) const' could be improved for better code maintainability.",
                    "file_rag_status": "Green",
                    "errors": [],
                    "improvements": [
                        {
                            "code": "pub(crate) (crate) const API_TIMEOUT: Duration = Duration::from_secs(30);",
                            "suggestion": "Consider using a configuration file or environment variables for API_TIMEOUT to allow for flexibility without recompilation.",
                            "improvement_details": "Implement a function to load the timeout from an environment variable or a configuration file."
                        }
                    ],
                    "security_issues": []
            }"#;

        let improvement = Improvement {
            code: "pub(crate) (crate) const API_TIMEOUT: Duration = Duration::from_secs(30);".to_string(),
            suggestion: "Consider using a configuration file or environment variables for API_TIMEOUT to allow for flexibility without recompilation.".to_string(),
            improvement_details: "Implement a function to load the timeout from an environment variable or a configuration file.".to_string(),
        };

        let expected_filereview: SourceFileReview = SourceFileReview {
            filename: "src/provider/static_config.rs".to_string(),
            id_hash: Some("".to_string()),
            summary:"The code defines a constant for a request timeout without any visible issues or security threats. However, the usage of 'pub(crate) (crate) const' could be improved for better code maintainability.".to_string(),
            file_rag_status: RAGStatus::Green,
            errors: Some(vec![]),
            improvements: Some(vec![improvement]),
            security_issues: Some(vec![]),
            statistics: None,
        };
        match deserialize_file_review(str_json) {
            Ok(filereview_from_json) => assert_eq!(expected_filereview, filereview_from_json),
            Err(e) => panic!("Failed to deserialize: {}", e),
        }
    }
}
