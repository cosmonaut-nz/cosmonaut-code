//! Contains functionality to retrieve data from the codebase and gain insights that will provide
//! statistics and inputs into subsequent review by an LLM.
//!
//! # Nomenclature:
//! - **\*Info**: data representation struct for a specific purpose, e.g. [`SourceFileInfo`], which is used to build [`SourceFileReview`]s
//! - **\*Breakdown**: a builder data struct that builds information for a specific purpose, e.g. [`LanguageBreakdown`], which is used to build [`LanguageFileType`]s
use crate::review::data::{RAGStatus, Severity, SourceFileReview};
use linguist::{
    container::InMemoryLanguageContainer,
    resolver::{resolve_language_from_content_str, Language, Scope},
    utils::{
        is_configuration_from_str, is_documentation_from_str, is_dotfile_from_str,
        is_vendor_from_str,
    },
};
use log::error;
use regex::RegexSet;
use std::ffi::OsStr;

use super::data::SourceFileInfo;
/// Contains the predefined languages, heuristics, vendors and documentation regexes from the GitHub Linguist project
pub(crate) mod predefined {
    include!(concat!(env!("OUT_DIR"), "/languages.rs"));
    include!(concat!(env!("OUT_DIR"), "/heuristics.rs"));
    include!(concat!(env!("OUT_DIR"), "/vendors.rs"));
    include!(concat!(env!("OUT_DIR"), "/documentation.rs"));
}
/// The prefixes that indicate a comment in a file
/// TODO: move to tokei crate
const COMMENT_PREFIXES: &[&str] = &["//", "///", "//!", "#", "\"\"\" "];

/// Initialize the language analysis by registering the predefined languages and heuristics as provided by the [`linguist`] crate
pub(crate) fn initialize_language_analysis() -> (InMemoryLanguageContainer, RegexSet, RegexSet) {
    let mut lc = InMemoryLanguageContainer::default();
    for &lang in predefined::LANGUAGES.iter() {
        lc.register_language(lang);
    }
    for &rule in predefined::HEURISTICS.iter() {
        lc.register_heuristic_rule(rule);
    }

    let rules = RegexSet::new(predefined::VENDORS).unwrap();
    let docs = RegexSet::new(predefined::DOCUMENTATION).unwrap();

    (lc, rules, docs)
}

/// Analyse the file language, returning the language, file size and lines of code
/// #Returns:
/// - Some((Language, file_size u64, loc i64)) if successful
// TODO: refactor to handle documentation, dotfiles, etc.
// TODO: refactor all this as it stinks
pub(crate) fn analyse_file_language(file_info: &mut SourceFileInfo) -> Option<&SourceFileInfo> {
    let (lc, rules, docs) = initialize_language_analysis();

    // TODO: resolve the type of file if docs, dotfile, or config and handle separately, particularly documentation, which needs to be summarised
    // [`linguist`] crate doesn't handle this very well, so need to resolve as the maintainer is very quiet
    if is_vendor_from_str(file_info.relative_path.clone(), &rules)
        || is_documentation_from_str(file_info.relative_path.clone(), &docs)
        || is_dotfile_from_str(file_info.relative_path.clone())
        || is_configuration_from_str(file_info.language.extension.clone())
    {
        // TODO: handle if is_documentation: if so then work out frequency; higher the count the better for overall RAG
        //          if no documentation then needs to be in repository summary and flagged as issue
        //          - i.e. best practice is that documentation is versioned with code, new developers will find it more easily, etc.
        return None;
    }

    // Use the Linguist crate to determine the language
    let language: &Language = match resolve_language_from_content_str(
        file_info.get_source_file_contents(),
        file_info.language.name.clone(),
        file_info.language.extension.clone(),
        &lc,
    ) {
        Ok(Some(lang)) => lang,
        _ => return None,
    };
    if language.scope != Scope::Programming && language.scope != Scope::Markup {
        return None;
    }

    let file_size: i64 = match get_file_contents_size(file_info.get_source_file_contents()) {
        Ok(size) => size as i64,
        Err(e) => {
            error!("Error when determining file size: {}", e);
            0
        }
    };
    let loc: i64 = match count_lines_of_code(file_info.get_source_file_contents()) {
        Ok(num_lines) => num_lines,
        Err(e) => {
            error!("Error when determining lines of code: {}", e);
            0
        }
    };
    file_info.statistics.size = file_size;
    file_info.statistics.loc = loc;
    file_info.statistics.num_files += 1;

    Some(file_info)
}

/// Calculates the RAG status for a [`SourceFileReview`] on the number of errors, improvements and security_issues, according to lines of code
pub(crate) fn calculate_rag_status_for_reviewed_file(
    reviewed_file: &SourceFileReview,
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
    let loc = reviewed_file.source_file_info.statistics.loc;

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
/// Calculates the size of the file_contents in bytes
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

/// Function to count lines of code in a file, skipping comments and empty lines
// TODO: shift to using tokei crate to improve maintainability and accuracy
fn count_lines_of_code(file_content: String) -> Result<i64, &'static str> {
    let mut is_comment_block = false;
    let mut functional_lines = 0;

    for line in file_content.lines() {
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
        let file_content: &str = r#"fn main() { // line 1 \n
                // this comment line doesn't add to the loc\n
                rror!(\"Hello, world!\"); // line 2 \n
            } // line 3 "#;
        let result: Result<i64, &str> = count_lines_of_code(file_content.to_string());
        assert_eq!(result, Ok(3));
    }
}
