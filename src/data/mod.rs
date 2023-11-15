//! This file contains the data structures that will hold the data that returns from
//! the AI code review API.
//! The JSON will be written to a file to allow subsequent analysis.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default, PartialEq)]
pub enum RAGStatus {
    #[default]
    Green,
    Amber,
    Red,
}
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct RepositoryReview {
    repository_name: String,          // Derived from path
    date: DateTime<Utc>,              // Date of execution
    repository_purpose: String,       // Derive from README, if present, else allow user entry in UI
    summary: String,                  // A roll up of the findings
    repository_rag_status: RAGStatus, // In {Red, Amber, Green}
    contributors: Vec<Contributor>,   // List of contributors to the codebase from commit history
    code_types: Vec<CodeType>, // The languages (as a %) found in the repository (a la GitHub)
    filereviews: Vec<FileReview>, // Each of the code files
}
impl RepositoryReview {
    pub fn new() -> Self {
        RepositoryReview {
            repository_name: String::new(),
            date: Utc::now(),
            repository_purpose: String::new(),
            summary: String::new(),
            repository_rag_status: RAGStatus::Green,
            contributors: Vec::new(),
            code_types: Vec::new(),
            filereviews: Vec::new(),
        }
    }
    pub fn set_repository_name(&mut self, name: String) {
        self.repository_name = name;
    }
    pub fn set_date(&mut self, date: DateTime<Utc>) {
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
    pub fn set_contributors(&mut self, contributors: Vec<Contributor>) {
        self.contributors = contributors;
    }
    pub fn set_code_types(&mut self, code_types: Vec<CodeType>) {
        self.code_types = code_types;
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
/// - 'errors': a Vec of errors found in the code giving the issue and potential resolution for each
/// - 'improvements': a Vec of ['Improvement']s, giving a suggestion and example for each
/// - 'security_issues': a Vec of ['SecurityIssue']s, giving the threat and mitigation for each
/// - 'statistics': a list of statistics (e.g., lines of code, functions, methods, etc.)
///
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct FileReview {
    filename: String,                    // The name of the file
    summary: String,                     // A summary of the findings of the review
    file_rag_status: RAGStatus,          // In {Red, Amber, Green}
    errors: Vec<Error>, // A list of errors found in the code giving the issue and potential resolution for each
    improvements: Vec<Improvement>, // A list of improvements, giving a suggestion and example for each
    security_issues: Vec<SecurityIssue>, // A list of security issues, giving the threat and mitigation for each
    statistics: String, // A list of statistics (e.g., lines of code, functions, methods, etc.)
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Improvement {
    suggestion: String,
    example: String,
}
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct SecurityIssue {
    threat: String,
    mitigation: String,
}
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Error {
    issue: String,
    resolution: String,
}
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct CodeType {
    language: String,
    percentage: i32,
}
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Contributor {
    name: String,
    last_contribution: DateTime<Utc>,
    percentage: i32,
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
                    "summary": "The code defines a constant for a request timeout without any visible issues or security threats. However, the usage of 'pub const' could be improved for better code maintainability.",
                    "file_rag_status": "Green",
                    "errors": [],
                    "improvements": [
                        {
                            "suggestion": "Consider using a configuration file or environment variables for API_TIMEOUT to allow for flexibility without recompilation.",
                            "example": "Implement a function to load the timeout from an environment variable or a configuration file."
                        }
                    ],
                    "security_issues": [],
                    "statistics": "Lines of code: 6, Constants: 1, Imports: 1, Comments: 2"
            }"#;

        let improvement = Improvement {
            suggestion: "Consider using a configuration file or environment variables for API_TIMEOUT to allow for flexibility without recompilation.".to_string(),
            example: "Implement a function to load the timeout from an environment variable or a configuration file.".to_string(),
        };

        let expected_filereview: FileReview = FileReview {
            filename: "/Users/avastmick/repos/cosmonaut-code/src/provider/static_config.rs".to_string(),
            summary:"The code defines a constant for a request timeout without any visible issues or security threats. However, the usage of 'pub const' could be improved for better code maintainability.".to_string(),
            file_rag_status: RAGStatus::Green,
            errors: vec![],
            improvements: vec![improvement],
            security_issues: vec![],
            statistics: "Lines of code: 6, Constants: 1, Imports: 1, Comments: 2".to_string(),
        };
        match deserialize_file_review(str_json) {
            Ok(filereview_from_json) => assert_eq!(expected_filereview, filereview_from_json),
            Err(e) => panic!("Failed to deserialize: {}", e),
        }
    }
}
