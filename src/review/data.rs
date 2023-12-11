//! This module contains the data structures that will hold the review data for presentation in a report.
//! The intent is that the data will be serialised to JSON and then passed to a templating engine to generate the report.
use serde::{Deserialize, Serialize};

use crate::{
    impl_builder_methods,
    retrieval::data::{Contributor, LanguageType, SourceFileInfo, Statistics},
};

/// Represents the overall review of the repository
/// #Fields:
/// * `repository_name` - The name of the repository
/// * `generative_ai_service_and_model` - The name of the generative AI service and model used to generate the review
/// * `repository_type` - The type of repository, e.g., 'Java', '.Net', etc.
/// * `date` - The date the review was generated
/// * `repository_purpose` - The purpose of the repository
/// * `summary` - A [`ReviewSummary`] of the repository
/// * `repository_rag_status` - The overall [`RAGStatus`] of the repository
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub(crate) struct RepositoryReview {
    pub(crate) repository_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) generative_ai_service_and_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    repository_type: Option<String>,
    date: String,
    repository_purpose: Option<String>,
    pub(crate) summary: Option<ReviewSummary>,
    repository_rag_status: RAGStatus,
    #[serde(skip_deserializing)]
    pub(crate) statistics: Statistics,
    contributors: Vec<Contributor>,
    language_types: Vec<LanguageType>,
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
            statistics: Statistics::new(),
            contributors: Vec::new(),
            language_types: Vec::new(),
            file_reviews: Vec::new(),
        }
    }
    /// pushes a [`FileReview`] into the filereviews [`Vec`]
    pub(crate) fn add_source_file_review(&mut self, file_review: SourceFileReview) {
        self.file_reviews.push(file_review);
    }
}

impl_builder_methods!(
    RepositoryReview,
    generative_ai_service_and_model: Option<String>,
    repository_type: Option<String>,
    date: String,
    repository_purpose: Option<String>,
    summary: Option<ReviewSummary>,
    repository_rag_status: RAGStatus,
    contributors: Vec<Contributor>,
    language_types: Vec<LanguageType>
);
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub(crate) struct ReviewSummary {
    pub(crate) text: String,
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
/// Captures retrieved static and review data from an LLM for a specific source file.
///
/// This struct will contain the fields passed back as JSON from the LLM.
///
/// #Fields:
/// * `source_file_info` - A [`SourceFileInfo`] struct containing the static data for the source file
/// * `summary` - A summary of the review
/// * `file_rag_status` - The overall [`RAGStatus`] of the file
/// * `security_issues` - A [`Vec`] of [`SecurityIssue`]s
/// * `errors` - A [`Vec`] of [`Error`]s
/// * `improvements` - A [`Vec`] of [`Improvement`]s
///
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub(crate) struct SourceFileReview {
    pub(crate) source_file_info: SourceFileInfo,
    pub(crate) summary: String,
    pub(crate) file_rag_status: RAGStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) security_issues: Option<Vec<SecurityIssue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) errors: Option<Vec<Error>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) improvements: Option<Vec<Improvement>>,
}
impl SourceFileReview {
    #[allow(dead_code)]
    pub(crate) fn get_security_issues(&self) -> &Option<Vec<SecurityIssue>> {
        &self.security_issues
    }
    #[allow(dead_code)]
    pub(crate) fn get_errors(&self) -> &Option<Vec<Error>> {
        &self.errors
    }
    #[allow(dead_code)]
    pub(crate) fn get_improvements(&self) -> &Option<Vec<Improvement>> {
        &self.improvements
    }
    #[allow(dead_code)]
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
    use crate::{
        retrieval::data::{LanguageType, SourceFileInfo, Statistics},
        review::data::{
            deserialize_file_review, Error, Improvement, RAGStatus, SecurityIssue, Severity,
            SourceFileReview,
        },
    };

    #[test]
    fn test_deserialize_file_review() {
        let json_str = r#"
            {
                "source_file_info": {
                    "name": "build.rs",
                    "relative_path": "build.rs",
                    "language": {
                        "name": "Rust",
                        "extension": ".rs",
                        "statistics": {
                            "size": 0,
                            "loc": 0,
                            "num_files": 0,
                            "num_commits": 0,
                            "frequency": 0
                        }
                    },
                    "id_hash": "0",
                    "statistics": {
                        "size": 0,
                        "loc": 0,
                        "num_files": 0,
                        "num_commits": 0,
                        "frequency": 0
                    }
                },
                "file_rag_status": "Green",
                "summary": "This is a review summary",
                "security_issues": [
                                {
                                    "severity": "Low",
                                    "code": "SEC001",
                                    "threat": "Potential security vulnerability",
                                    "mitigation": "Apply security patch"
                                }],
                "errors": [
                                {
                                    "code": "ERR001",
                                    "issue": "Syntax error",
                                    "resolution": "Fix syntax error"
                                }],
                "improvements": [
                                {
                                    "code": "IMP001",
                                    "suggestion": "Refactor code",
                                    "improvement_details": "Improve code readability"
                                }]
            }
            "#;

        let expected_result = SourceFileReview {
            source_file_info: SourceFileInfo {
                name: "build.rs".to_string(),
                relative_path: "build.rs".to_string(),
                language: LanguageType {
                    name: "Rust".to_string(),
                    extension: ".rs".to_string(),
                    statistics: Statistics {
                        size: 0,
                        loc: 0,
                        num_files: 0,
                        num_commits: 0,
                        frequency: 0.0,
                    },
                },
                id_hash: "0".to_string(),
                source_file: None,
                statistics: Statistics {
                    size: 0,
                    loc: 0,
                    num_files: 0,
                    num_commits: 0,
                    frequency: 0.0,
                },
            },
            summary: "This is a review summary".to_string(),
            file_rag_status: RAGStatus::Green,
            security_issues: Some(vec![SecurityIssue {
                severity: Severity::Low,
                code: "SEC001".to_string(),
                threat: "Potential security vulnerability".to_string(),
                mitigation: "Apply security patch".to_string(),
            }]),
            errors: Some(vec![Error {
                code: "ERR001".to_string(),
                issue: "Syntax error".to_string(),
                resolution: "Fix syntax error".to_string(),
            }]),
            improvements: Some(vec![Improvement {
                code: "IMP001".to_string(),
                suggestion: "Refactor code".to_string(),
                improvement_details: "Improve code readability".to_string(),
            }]),
        };

        let result = deserialize_file_review(json_str).unwrap();
        assert_eq!(result, expected_result);
    }
}
