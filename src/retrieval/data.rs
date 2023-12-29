//! This module contains the structs that describe the retrieval data, such as code, contributors, etc.
use std::{ffi::OsString, fmt, sync::Arc};

use chrono::{DateTime, Utc};
use linguist::resolver::Language;
use serde::{Deserialize, Serialize};

/// Struct to hold statistics on the code in a repository
///
/// # Fields:
/// * `size` - The size of the repository in bytes
/// * `loc` - The number of lines of code in the repository
/// * `num_file` - The number of files in the repository
/// * `num_commits` - The number of commits in the repository
/// * `frequency` - The frequency of commits to the repository, as a ratio of commits to total commits in the repository
#[derive(Clone, Default, Serialize, Deserialize, Debug, PartialEq)]
pub(crate) struct Statistics {
    pub(crate) size: i64,
    pub(crate) loc: i64,
    pub(crate) num_files: i32,
    pub(crate) num_commits: i32,
    pub(crate) frequency: f32,
}
impl Statistics {
    pub(crate) fn new() -> Self {
        Self {
            size: 0,
            loc: 0,
            num_files: 0,
            num_commits: 0,
            frequency: 0.0,
        }
    }
}
/// Struct to hold the data on a repository's contributors
///
/// # Fields:
/// * `name` - The name of the contributor
/// * `last_contribution` - The date and time of the last contribution made by the contributor
/// * `percentage_contribution` - The percentage of the total contributions made by the contributor
/// * `statistics` - The [`Statistics`] on the contributor's contributions
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub(crate) struct Contributor {
    name: String,
    last_contribution: DateTime<Utc>,
    percentage_contribution: f32,
    statistics: Statistics,
}
impl Contributor {
    pub(crate) fn new(
        name: String,
        last_contribution: DateTime<Utc>,
        percentage_contribution: f32,
        statistics: Statistics,
    ) -> Self {
        Self {
            name,
            last_contribution,
            percentage_contribution,
            statistics,
        }
    }
}
/// Top-level struct to hold statistics on the [`LanguageType`]s found in the repository.
/// Each source file will be assigned a [`LanguageType`] based on the language and file extension.
/// Note that the "Language", e.g., 'Rust', may have multiple file extensions, e.g., '.rs', '.toml', etc. and therefore multiple [`LanguageType`]s.
///
/// #Fields:
/// * `language` - The language of the file type
/// * `extension` - The file extension of the file type
/// * `percentage` - The percentage of the total lines of code in the repository that are of this [`LanguageType`]
/// * `statistics` - The [`Statistics`] on the file type
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub(crate) struct LanguageType {
    pub(crate) name: String,
    pub(crate) extension: String,
    pub(crate) statistics: Option<Statistics>,
}
impl LanguageType {
    /// gets the [`LanguageType`] from the linguist::Language
    pub(crate) fn from_language(language: &Language) -> Self {
        let ext = language
            .extensions
            .first()
            .map(|os_str| os_str.to_str().unwrap_or_default())
            .unwrap_or_default();
        Self {
            name: language.name.clone(),
            extension: ext.to_string(),
            statistics: None,
        }
    }
    /// Method to check if an extension is valid among a collection of LanguageFileType
    pub(crate) fn _has_extension_of(ext: &str, file_types: &[LanguageType]) -> bool {
        file_types.iter().any(|lft| lft.extension == *ext)
    }
    /// Sums the lines of code for an array of [`LanguageType`]s
    fn sum_lines_of_code(language_types: &[LanguageType]) -> i64 {
        language_types
            .iter()
            .filter_map(|lt| lt.statistics.as_ref().map(|s| s.loc))
            .sum()
    }
    /// Gets the predominant language from an array of [`LanguageType`]s
    pub(crate) fn get_predominant_language(languages: &[LanguageType]) -> String {
        let mut predominant_language = String::new();
        let mut highest_percentage = 0.0;
        let mut largest_size = 0_i64;

        for lang in languages {
            if let Some(statistics) = &lang.statistics {
                if statistics.frequency > highest_percentage
                    || (statistics.frequency == highest_percentage
                        && statistics.size > largest_size)
                {
                    highest_percentage = statistics.frequency;
                    largest_size = statistics.size;
                    predominant_language = lang.name.clone();
                }
            }
        }
        predominant_language
    }
    /// Calculates percentage distribution of the [`LanguageType`]s - i.e., the percentage of
    /// lines of code that each [`LanguageType`] in relation to each other and updates the [`Statistics`].frequency field for each [`LanguageType`]
    pub(crate) fn calculate_percentage_distribution(languages: &mut [LanguageType]) {
        let total_lines_of_code = LanguageType::sum_lines_of_code(languages);
        for language in languages {
            if let Some(statistics) = &mut language.statistics {
                statistics.frequency = (statistics.loc as f32 / total_lines_of_code as f32) * 100.0;
            }
        }
    }
    /// Formats the percentage to 2 decimal places - used in the HTML template
    #[allow(dead_code)]
    pub(crate) fn formatted_percentage(&self) -> String {
        if let Some(statistics) = &self.statistics {
            format!("{:.2}", statistics.frequency)
        } else {
            String::new()
        }
    }
}
/// Represents the information for a specific source file during the static retrieval phase
///
/// #Fields:
/// * `name` - The name of the file
/// * `relative_path` - The relative path of the file from the root of the repository
/// * `language` - The [`LanguageType`] of the file
/// * `id_hash` - The (SHA256) hash of the file
/// * `source_file` - The contents of the file in a [`SourceFile`] container
/// * `statistics` - The [`Statistics`] on the file
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub(crate) struct SourceFileInfo {
    pub(crate) name: String,
    pub(crate) relative_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) language: Option<LanguageType>,
    pub(crate) id_hash: Option<String>,
    #[serde(skip)]
    pub(crate) source_file: Option<Box<SourceFile>>,
    #[serde(skip_deserializing)]
    pub(crate) statistics: Statistics,
}
impl SourceFileInfo {
    pub(crate) fn new(
        name: String,
        relative_path: String,
        language: LanguageType,
        id_hash: String,
        statistics: Statistics,
    ) -> Self {
        Self {
            name,
            relative_path,
            language: Some(language),
            id_hash: Some(id_hash),
            source_file: None,
            statistics,
        }
    }
    pub(crate) fn set_source_file_contents(&mut self, contents: String) {
        self.source_file = Some(Box::new(SourceFile {
            parent: self.clone(),
            contents: Arc::new(contents.into()),
        }));
    }
    pub(crate) fn get_source_file_contents(&self) -> String {
        match &self.source_file {
            Some(source_file) => source_file
                .contents
                .to_str()
                .unwrap_or_default()
                .to_string(),
            None => {
                log::error!("Failed to retrieve source file: {}", self.name);
                String::new()
            }
        }
    }
}
/// Represents the contents of a source file
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SourceFile {
    parent: SourceFileInfo,
    contents: Arc<OsString>,
}
/// Captures the file change frequency for a file
/// #Fields:
/// * file_commits: the number of commits that the file has been changed in
/// * total_commits: the total number of commits in the repository as reference
/// * frequency: the frequency of the file being changed, as a ratio of file_commits to total_commits
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub(crate) struct SourceFileChangeFrequency {
    pub(crate) file_commits: i32,
    pub(crate) total_commits: i32,
    pub(crate) frequency: f32,
}
impl SourceFileChangeFrequency {
    pub(crate) fn get_as_statistics(&self) -> Statistics {
        Statistics {
            size: 0,
            loc: 0,
            num_files: 0,
            num_commits: self.file_commits,
            frequency: self.frequency,
        }
    }
}

pub(crate) enum SourceFileError {
    GitError(String),
}
impl fmt::Display for SourceFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SourceFileError::GitError(name) => write!(f, "Git error: {}", name),
        }
    }
}
impl std::fmt::Debug for SourceFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GitError(arg0) => f.debug_tuple("GitError").field(arg0).finish(),
        }
    }
}
impl From<git2::Error> for SourceFileError {
    fn from(error: git2::Error) -> Self {
        SourceFileError::GitError(error.message().to_string())
    }
}
impl std::error::Error for SourceFileError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_extension_of() {
        let file_types = vec![
            LanguageType {
                name: "Rust".to_string(),
                extension: ".rs".to_string(),
                statistics: None,
            },
            LanguageType {
                name: "Python".to_string(),
                extension: ".py".to_string(),
                statistics: None,
            },
        ];

        assert!(LanguageType::_has_extension_of(".rs", &file_types));
        assert!(!LanguageType::_has_extension_of(".toml", &file_types));
    }

    #[test]
    fn test_sum_lines_of_code() {
        let language_types = vec![
            LanguageType {
                name: "Rust".to_string(),
                extension: ".rs".to_string(),
                statistics: Some(Statistics {
                    loc: 100,
                    size: 2345,
                    num_files: 3,
                    num_commits: 12,
                    frequency: 12.34,
                }),
            },
            LanguageType {
                name: "Python".to_string(),
                extension: ".py".to_string(),
                statistics: Some(Statistics {
                    loc: 200,
                    size: 12345,
                    num_files: 10,
                    num_commits: 12,
                    frequency: 12.34,
                }),
            },
        ];

        assert_eq!(LanguageType::sum_lines_of_code(&language_types), 300);
    }

    #[test]
    fn test_get_predominant_language() {
        let languages = vec![
            LanguageType {
                name: "Rust".to_string(),
                extension: ".rs".to_string(),
                statistics: Some(Statistics {
                    loc: 100,
                    size: 2345,
                    num_files: 3,
                    num_commits: 12,
                    frequency: 12.34,
                }),
            },
            LanguageType {
                name: "Python".to_string(),
                extension: ".py".to_string(),
                statistics: Some(Statistics {
                    loc: 200,
                    size: 2345,
                    num_files: 3,
                    num_commits: 12,
                    frequency: 12.34,
                }),
            },
            LanguageType {
                name: "JavaScript".to_string(),
                extension: ".js".to_string(),
                statistics: Some(Statistics {
                    loc: 150,
                    size: 2345,
                    num_files: 3,
                    num_commits: 12,
                    frequency: 12.34,
                }),
            },
        ];

        assert_eq!(
            LanguageType::get_predominant_language(&languages),
            "Rust".to_string()
        );
    }

    #[test]
    fn test_formatted_percentage() {
        let mut stats = Statistics::new();
        stats.frequency = 0.1234;
        let language_type = LanguageType {
            name: "Rust".to_string(),
            extension: ".rs".to_string(),
            statistics: Some(stats),
        };

        assert_eq!(language_type.formatted_percentage(), "0.12");
    }
}
