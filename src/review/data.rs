//! This file contains the data structures that will hold the data that returns from
//! the AI code review API.
//! The JSON will be written to a file to allow subsequent analysis.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq)]
pub enum RAGStatus {
    #[default]
    Green,
    Amber,
    Red,
}
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct RepositoryReview {
    repository_name: String, // Derived from path
    #[serde(skip_serializing_if = "Option::is_none")]
    repository_type: Option<String>, // The type of repository, e.g., Java, .Net, etc.
    date: String,            // Date of execution (formatted)
    repository_purpose: String, // Derive from README, if present, else allow user entry in UI
    summary: String,         // A roll up of the findings
    repository_rag_status: RAGStatus, // In {Red, Amber, Green}
    sum_loc: Option<i64>,    // Total lines of code
    sum_num_files: Option<i32>, // Total number of files
    contributors: Vec<Contributor>, // List of contributors to the codebase from commit history
    language_file_types: Vec<LanguageFileType>, // The languages (as a %) found in the repository (a la GitHub)
    pub filereviews: Vec<FileReview>,           // Each of the code files
}
// TODO move over to impl_builder_methods!
impl RepositoryReview {
    pub fn new() -> Self {
        RepositoryReview {
            repository_name: String::new(),
            repository_type: None,
            date: String::new(),
            repository_purpose: String::new(),
            summary: String::new(),
            repository_rag_status: RAGStatus::Green,
            sum_loc: None,
            sum_num_files: None,
            contributors: Vec::new(),
            language_file_types: Vec::new(),
            filereviews: Vec::new(),
        }
    }
    pub fn get_file_reviews(&self) -> &Vec<FileReview> {
        &self.filereviews
    }
    pub fn set_repository_name(&mut self, name: String) {
        self.repository_name = name;
    }
    pub fn set_repository_type(&mut self, _type: String) {
        self.repository_type = Some(_type);
    }
    pub fn set_date(&mut self, date: String) {
        self.date = date;
    }
    pub fn set_repository_purpose(&mut self, purpose: String) {
        self.repository_purpose = purpose;
    }
    pub fn set_summary(&mut self, summary: String) {
        self.summary = summary;
    }
    pub fn set_repository_rag_status(&mut self, status: RAGStatus) {
        self.repository_rag_status = status;
    }
    pub fn set_sum_loc(&mut self, loc: i64) {
        self.sum_loc = Some(loc);
    }
    pub fn set_num_files(&mut self, num: i32) {
        self.sum_num_files = Some(num);
    }
    pub fn set_contributors(&mut self, contributors: Vec<Contributor>) {
        self.contributors = contributors;
    }
    pub fn set_lfts(&mut self, language_file_types: Vec<LanguageFileType>) {
        self.language_file_types = language_file_types;
    }
    pub fn add_file_review(&mut self, file_review: FileReview) {
        self.filereviews.push(file_review);
    }
}
impl Default for RepositoryReview {
    fn default() -> Self {
        Self::new()
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
pub struct FileReview {
    pub filename: String,           // The name of the file
    pub summary: String,            // A summary of the findings of the review
    pub file_rag_status: RAGStatus, // In {Red, Amber, Green}
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<Error>>, // A list of errors found in the code giving the issue and potential resolution for each
    #[serde(skip_serializing_if = "Option::is_none")]
    pub improvements: Option<Vec<Improvement>>, // A list of improvements, giving a suggestion and example for each
    #[serde(skip_serializing_if = "Option::is_none")]
    pub security_issues: Option<Vec<SecurityIssue>>, // A list of security issues, giving the threat and mitigation for each
    #[serde(skip_serializing_if = "Option::is_none")]
    pub statistics: Option<LanguageFileType>, // A list of statistics (e.g., lines of code, functions, methods, etc.)
}

impl FileReview {
    #[allow(dead_code)]
    pub fn get_improvements(&self) -> &Option<Vec<Improvement>> {
        &self.improvements
    }
    pub fn get_errors(&self) -> &Option<Vec<Error>> {
        &self.errors
    }
    pub fn get_security_issues(&self) -> &Option<Vec<SecurityIssue>> {
        &self.security_issues
    }
    pub fn get_file_rag_status(&self) -> &RAGStatus {
        &self.file_rag_status
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct Improvement {
    code: String,
    suggestion: String,
    example: String,
}
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct SecurityIssue {
    code: String,
    threat: String,
    mitigation: String,
}
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct Error {
    code: String,
    issue: String,
    resolution: String,
}
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct LanguageFileType {
    pub language: String,
    pub extension: String,
    pub percentage: f64,
    pub loc: i64,
    pub total_size: u64,
    pub file_count: i32,
}
impl LanguageFileType {
    // Method to check if an extension is valid among a collection of LanguageFileType
    pub fn _has_extension_of(ext: &str, file_types: &[LanguageFileType]) -> bool {
        file_types.iter().any(|lft| lft.extension == ext)
    }
    pub fn sum_lines_of_code(language_file_types: &[LanguageFileType]) -> i64 {
        language_file_types.iter().map(|lft| lft.loc).sum()
    }
    pub fn get_predominant_language(languages: &[LanguageFileType]) -> Option<String> {
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
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct Contributor {
    name: String,
    num_commits: i32,
    last_contribution: DateTime<Utc>,
    percentage: i32,
}
impl Contributor {
    pub fn new(
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
pub fn deserialize_file_review(json_str: &str) -> Result<FileReview, serde_json::Error> {
    serde_json::from_str(json_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_file_review() {
        let str_json = r#"{
                    "filename": "/Users/avastmick/repos/cosmonaut-code/src/provider/static_config.rs",
                    "summary": "The code defines a constant for a request timeout without any visible issues or security threats. However, the usage of 'pub (crate) const' could be improved for better code maintainability.",
                    "file_rag_status": "Green",
                    "errors": [],
                    "improvements": [
                        {
                            "code": "pub (crate) const API_TIMEOUT: Duration = Duration::from_secs(30);",
                            "suggestion": "Consider using a configuration file or environment variables for API_TIMEOUT to allow for flexibility without recompilation.",
                            "example": "Implement a function to load the timeout from an environment variable or a configuration file."
                        }
                    ],
                    "security_issues": []
            }"#;

        let improvement = Improvement {
            code: "pub (crate) const API_TIMEOUT: Duration = Duration::from_secs(30);".to_string(),
            suggestion: "Consider using a configuration file or environment variables for API_TIMEOUT to allow for flexibility without recompilation.".to_string(),
            example: "Implement a function to load the timeout from an environment variable or a configuration file.".to_string(),
        };

        let expected_filereview: FileReview = FileReview {
            filename: "/Users/avastmick/repos/cosmonaut-code/src/provider/static_config.rs".to_string(),
            summary:"The code defines a constant for a request timeout without any visible issues or security threats. However, the usage of 'pub (crate) const' could be improved for better code maintainability.".to_string(),
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
