//! contains functionality to look at the codebase and gain insights that will provide
//! statistics and inputs into subsequent review.
use crate::review::data::{FileReview, LanguageFileType, RAGStatus, Severity};
use linguist::{
    container::InMemoryLanguageContainer,
    resolver::{resolve_language_from_content_str, Language, Scope},
};
use log::error;
use regex::RegexSet;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    ffi::{OsStr, OsString},
    sync::Arc,
};

pub(crate) mod predefined {
    include!(concat!(env!("OUT_DIR"), "/languages.rs"));
    include!(concat!(env!("OUT_DIR"), "/heuristics.rs"));
    include!(concat!(env!("OUT_DIR"), "/vendors.rs"));
    include!(concat!(env!("OUT_DIR"), "/documentation.rs"));
}

const COMMENT_PREFIXES: &[&str] = &["//", "///", "//!", "#", "\"\"\" "];

// Initialize language container and return necessary objects
pub(crate) fn initialize_language_analysis() -> (
    InMemoryLanguageContainer,
    LanguageBreakdown,
    RegexSet,
    RegexSet,
) {
    let mut lc = InMemoryLanguageContainer::default();
    for &lang in predefined::LANGUAGES.iter() {
        lc.register_language(lang);
    }

    for &rule in predefined::HEURISTICS.iter() {
        lc.register_heuristic_rule(rule);
    }

    let breakdown = LanguageBreakdown {
        usages: HashMap::new(),
        total_size: 0,
    };

    let rules = RegexSet::new(predefined::VENDORS).unwrap();
    let docs = RegexSet::new(predefined::DOCUMENTATION).unwrap();

    (lc, breakdown, rules, docs)
}
#[derive(Debug)]
pub(crate) struct FileInfo {
    pub(crate) contents: Arc<OsString>,
    pub(crate) name: Arc<OsString>,
    pub(crate) id_hash: Arc<OsString>,
    pub(crate) ext: Arc<OsString>,
    pub(crate) language: Option<Language>,
    pub(crate) file_size: Option<u64>,
    pub(crate) loc: Option<i64>,
}

pub(crate) fn analyse_file_language(
    file_info: &FileInfo,
    lc: &InMemoryLanguageContainer,
    _rules: &RegexSet,
    _docs: &RegexSet,
) -> Option<(Language, u64, i64)> {
    // TODO: resolve the type of file if docs, dotfile, or config and handle separately, particularly documentation, which needs to be summarised
    // if is_vendor(entry.path(), rules)
    //     || is_documentation(relative_path, docs)
    //     || is_dotfile(relative_path)
    //     || is_configuration(relative_path)
    // {
    //     // TODO: handle if is_documentation: if so then work out frequency; higher the count the better for overall RAG
    //     //          if no documentation then needs to be in repository summary and flagged as issue
    //     //          - i.e. best practice is that documentation is versioned with code, new developers will find it more easily, etc.
    //     return None;
    // }
    let language: &Language = match resolve_language_from_content_str(
        file_info.contents.as_os_str(),
        file_info.name.as_os_str(),
        file_info.ext.as_os_str(),
        lc,
    ) {
        Ok(Some(lang)) => lang,
        _ => return None,
    };

    if language.scope != Scope::Programming && language.scope != Scope::Markup {
        return None;
    }

    let file_size = match get_file_contents_size(file_info.contents.as_os_str()) {
        Ok(size) => size,
        Err(e) => {
            error!("Error when determining file size: {}", e);
            0
        }
    };
    let loc: i64 = match count_lines_of_code(&file_info.contents) {
        Ok(num_lines) => num_lines,
        Err(e) => {
            error!("Error when determining lines of code: {}", e);
            0
        }
    };

    Some((language.clone(), file_size, loc))
}

/// Calculates the RAG status for a [`FileReview`] on the number of errors, improvements and security_issues, according to lines of code
pub(crate) fn calculate_rag_status_for_reviewed_file(
    reviewed_file: &FileReview,
) -> Option<RAGStatus> {
    let errors_count = reviewed_file
        .errors
        .as_ref()
        .map_or(0, |errors| errors.len());
    let improvements_count = reviewed_file
        .improvements
        .as_ref()
        .map_or(0, |improvements| improvements.len());
    let security_issues_count = reviewed_file
        .security_issues
        .as_ref()
        .map_or(0, |issues| issues.len());
    let loc = reviewed_file
        .statistics
        .as_ref()
        .map_or(0, |statistics| statistics.loc.unwrap());

    let error_ratio = errors_count as f64 / loc as f64;
    let security_issues_ratio = security_issues_count as f64 / loc as f64;
    let improvements_ratio = improvements_count as f64 / loc as f64;

    let green_error_threshold = 0.07; // 7% of loc
    let amber_error_threshold = 0.18; // 18% of loc
    let green_improvement_threshold = 0.15; // 15% of loc
    let amber_improvement_threshold = 0.40; // 40% of loc

    if let Some(security_issues) = &reviewed_file.security_issues {
        for issue in security_issues {
            match issue.severity {
                Severity::High | Severity::Critical => return Some(RAGStatus::Red),
                _ => continue,
            }
        }
    }
    if error_ratio <= green_error_threshold
        && security_issues_ratio <= 0.05 // 5% of loc
        && improvements_ratio <= green_improvement_threshold
    {
        return Some(RAGStatus::Green);
    } else if error_ratio <= amber_error_threshold
        && security_issues_ratio <= 0.12 // 12% of loc
        && improvements_ratio <= amber_improvement_threshold
    {
        return Some(RAGStatus::Amber);
    }
    Some(RAGStatus::Red)
}

fn get_file_contents_size(file_contents: impl AsRef<OsStr>) -> Result<u64, &'static str> {
    let content_str = file_contents
        .as_ref()
        .to_str()
        .ok_or("Invalid UTF-8 content")?;
    let length: u64 = content_str
        .len()
        .try_into()
        .map_err(|_| "Length conversion error")?;
    Ok(length)
}

/// Function to count lines of code in a file, skipping comments
// TODO: shift to using tokei crate to improve maintainability and accuracy
fn count_lines_of_code(file_content: impl AsRef<OsString>) -> Result<i64, &'static str> {
    let content_str = file_content
        .as_ref()
        .to_str()
        .ok_or("Invalid UTF-8 content")?;
    let mut is_comment_block = false;
    let mut functional_lines = 0;

    for line in content_str.lines() {
        let line = line.trim();
        if line.starts_with("/*") {
            is_comment_block = true;
        }
        if line.ends_with("*/") {
            is_comment_block = false;
            continue;
        }
        if COMMENT_PREFIXES
            .iter()
            .any(|&prefix| line.starts_with(prefix))
            || is_comment_block
        {
            continue;
        }
        functional_lines += 1;
    }

    Ok(functional_lines)
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub(crate) struct LanguageBreakdown {
    pub(crate) usages: HashMap<String, HashMap<String, (u64, i32, i64)>>, // size, count, loc
    pub(crate) total_size: u64,
}

impl LanguageBreakdown {
    pub(crate) fn add_usage(&mut self, lang: &str, ext: &str, size: u64, loc: i64) {
        let language_entry = self.usages.entry(lang.to_string()).or_default();
        let entry = language_entry.entry(ext.to_string()).or_insert((0, 0, 0));
        entry.0 += size; // Increase size
        entry.1 += 1; // Increment file count
        entry.2 += loc; // Increment LOC
        self.total_size += size;
    }
    pub(crate) fn to_language_file_types(&self) -> Vec<LanguageFileType> {
        let mut types = Vec::new();

        for (language, extensions) in &self.usages {
            for (extension, &(size, count, loc)) in extensions {
                let percentage = (size as f64 / self.total_size as f64) * 100.0;
                types.push(LanguageFileType {
                    language: Some(language.clone()),
                    extension: Some(extension.clone()),
                    percentage: Some(percentage),
                    loc: Some(loc),
                    total_size: Some(size),
                    file_count: Some(count),
                });
            }
        }
        types
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_file_contents_size() {
        let file_contents = "Hello, world!";
        let result = get_file_contents_size(file_contents);
        assert_eq!(result, Ok(13));
    }

    #[test]
    fn test_count_lines_of_code() {
        let file_content: OsString = OsString::from(
            r#"fn main() { // line 1 \n
                // this comment line doesn't add to the loc\n
                println!(\"Hello, world!\"); // line 2 \n
            } // line 3 "#,
        );
        let arc_os_string: Arc<OsString> = Arc::new(file_content);
        let result: Result<i64, &str> = count_lines_of_code(arc_os_string);
        assert_eq!(result, Ok(3));
    }

    #[test]
    fn test_language_breakdown_add_usage() {
        let mut breakdown = LanguageBreakdown {
            usages: HashMap::new(),
            total_size: 0,
        };
        breakdown.add_usage("Rust", "rs", 100, 10);
        breakdown.add_usage("Rust", "toml", 50, 5);

        let expected_usages: HashMap<String, HashMap<String, (u64, i32, i64)>> = [(
            "Rust".to_string(),
            [
                ("rs".to_string(), (100, 1, 10)),
                ("toml".to_string(), (50, 1, 5)),
            ]
            .iter()
            .cloned()
            .collect(),
        )]
        .iter()
        .cloned()
        .collect();

        assert_eq!(breakdown.usages, expected_usages);
        assert_eq!(breakdown.total_size, 150);
    }

    #[test]
    fn test_language_breakdown_to_language_file_types() {
        let mut breakdown = LanguageBreakdown {
            usages: HashMap::new(),
            total_size: 0,
        };
        breakdown.add_usage("Rust", "rs", 100, 10);
        breakdown.add_usage("Rust", "toml", 50, 5);

        let mut result: Vec<LanguageFileType> = breakdown.to_language_file_types();

        let mut expected_result: Vec<LanguageFileType> = vec![
            LanguageFileType {
                language: Some("Rust".to_string()),
                extension: Some("toml".to_string()),
                percentage: Some(33.33333333333333),
                loc: Some(5),
                total_size: Some(50),
                file_count: Some(1),
            },
            LanguageFileType {
                language: Some("Rust".to_string()),
                extension: Some("rs".to_string()),
                percentage: Some(66.66666666666666),
                loc: Some(10),
                total_size: Some(100),
                file_count: Some(1),
            },
        ];

        result.sort_by(|a, b| a.extension.cmp(&b.extension));
        expected_result.sort_by(|a, b| a.extension.cmp(&b.extension));

        assert_eq!(result, expected_result);
    }
}
