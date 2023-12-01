//! This file contains the data structures that will hold the data that returns from
//! the AI code review API.
//! The JSON will be written to a file to allow subsequent analysis.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::impl_builder_methods;

#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq)]
pub(crate) enum RAGStatus {
    #[default]
    Green,
    Amber,
    Red,
}
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub(crate) struct RepositoryReview {
    pub(crate) repository_name: String, // Derived from path
    #[serde(skip_serializing_if = "Option::is_none")]
    generative_ai_service_and_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    repository_type: Option<String>, // The type of repository, e.g., Java, .Net, etc.
    date: String,                                // Date of execution (formatted)
    repository_purpose: Option<String>, // Derive from README, if present, else allow user entry in UI
    pub(crate) summary: Option<ReviewBreakdown>, // A roll up of the findings
    repository_rag_status: RAGStatus,   // In {Red, Amber, Green}
    sum_loc: Option<i64>,               // Total lines of code
    sum_num_files: Option<i32>,         // Total number of files
    contributors: Vec<Contributor>,     // List of contributors to the codebase from commit history
    language_file_types: Vec<LanguageFileType>, // The languages (as a %) found in the repository (a la GitHub)
    pub(crate) file_reviews: Vec<FileReview>,   // Each of the code files
}
impl RepositoryReview {
    pub(crate) fn new() -> Self {
        RepositoryReview {
            repository_name: String::new(),
            generative_ai_service_and_model: None,
            repository_type: None,
            date: String::new(),
            repository_purpose: None,
            summary: None,
            repository_rag_status: RAGStatus::Green,
            sum_loc: None,
            sum_num_files: None,
            contributors: Vec::new(),
            language_file_types: Vec::new(),
            file_reviews: Vec::new(),
        }
    }
    /// pushes a [`FileReview`] into the filereviews [`Vec`]
    pub(crate) fn add_file_review(&mut self, file_review: FileReview) {
        self.file_reviews.push(file_review);
    }
}
impl Default for RepositoryReview {
    fn default() -> Self {
        Self::new()
    }
}

impl_builder_methods!(
    RepositoryReview,
    repository_name: String,
    generative_ai_service_and_model: Option<String>,
    repository_type: Option<String>,
    date: String,
    repository_purpose: Option<String>,
    summary: Option<ReviewBreakdown>,
    repository_rag_status: RAGStatus,
    sum_loc: Option<i64>,
    sum_num_files: Option<i32>,
    contributors: Vec<Contributor>,
    language_file_types: Vec<LanguageFileType>
);
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
pub(crate) struct FileReview {
    pub(crate) filename: String,           // The name of the file
    pub(crate) summary: String,            // A summary of the findings of the review
    pub(crate) file_rag_status: RAGStatus, // In {Red, Amber, Green}
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) errors: Option<Vec<Error>>, // A list of errors found in the code giving the issue and potential resolution for each
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) improvements: Option<Vec<Improvement>>, // A list of improvements, giving a suggestion and example for each
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) security_issues: Option<Vec<SecurityIssue>>, // A list of security issues, giving the threat and mitigation for each
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) statistics: Option<LanguageFileType>, // A list of statistics (e.g., lines of code, functions, methods, etc.)
}

impl FileReview {
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
pub(crate) struct Improvement {
    code: String,
    suggestion: String,
    example: String,
}
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub(crate) struct Error {
    code: String,
    issue: String,
    resolution: String,
}
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub(crate) struct SecurityIssue {
    pub(crate) severity: Severity,
    pub(crate) code: String,
    pub(crate) threat: String,
    pub(crate) mitigation: String,
}
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub(crate) enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

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
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub(crate) struct LanguageFileType {
    pub(crate) language: String,
    pub(crate) extension: String,
    pub(crate) percentage: f64,
    pub(crate) loc: i64,
    pub(crate) total_size: u64,
    pub(crate) file_count: i32,
}
impl LanguageFileType {
    // Method to check if an extension is valid among a collection of LanguageFileType
    pub(crate) fn _has_extension_of(ext: &str, file_types: &[LanguageFileType]) -> bool {
        file_types.iter().any(|lft| lft.extension == ext)
    }
    pub(crate) fn sum_lines_of_code(language_file_types: &[LanguageFileType]) -> i64 {
        language_file_types.iter().map(|lft| lft.loc).sum()
    }
    pub(crate) fn get_predominant_language(languages: &[LanguageFileType]) -> Option<String> {
        let mut predominant_language = None;
        let mut highest_percentage = 0.0;
        let mut largest_size = 0;

        for lang in languages {
            if lang.percentage > highest_percentage
                || (lang.percentage == highest_percentage && lang.total_size > largest_size)
            {
                highest_percentage = lang.percentage;
                largest_size = lang.total_size;
                predominant_language = Some(lang.language.clone());
            }
        }

        predominant_language
    }
    /// Used in the HTML template
    #[allow(dead_code)]
    pub(crate) fn formatted_percentage(&self) -> String {
        format!("{:.2}", self.percentage)
    }
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
///
/// Deserializes a str into a ['FileReview'] struct.
///
/// # Parameters
///
/// * `json_str` - A str representation of the JSON to be deserialized
///
/// # Returns
///
/// * A ['FileReview'] struct
///
pub(crate) fn deserialize_file_review(json_str: &str) -> Result<FileReview, serde_json::Error> {
    serde_json::from_str(json_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_file_review() {
        let str_json = r#"{
                    "filename": "/Users/avastmick/repos/cosmonaut-code/src/provider/static_config.rs",
                    "summary": "The code defines a constant for a request timeout without any visible issues or security threats. However, the usage of 'pub(crate) (crate) const' could be improved for better code maintainability.",
                    "file_rag_status": "Green",
                    "errors": [],
                    "improvements": [
                        {
                            "code": "pub(crate) (crate) const API_TIMEOUT: Duration = Duration::from_secs(30);",
                            "suggestion": "Consider using a configuration file or environment variables for API_TIMEOUT to allow for flexibility without recompilation.",
                            "example": "Implement a function to load the timeout from an environment variable or a configuration file."
                        }
                    ],
                    "security_issues": []
            }"#;

        let improvement = Improvement {
            code: "pub(crate) (crate) const API_TIMEOUT: Duration = Duration::from_secs(30);".to_string(),
            suggestion: "Consider using a configuration file or environment variables for API_TIMEOUT to allow for flexibility without recompilation.".to_string(),
            example: "Implement a function to load the timeout from an environment variable or a configuration file.".to_string(),
        };

        let expected_filereview: FileReview = FileReview {
            filename: "/Users/avastmick/repos/cosmonaut-code/src/provider/static_config.rs".to_string(),
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
