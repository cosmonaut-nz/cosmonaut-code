//! contains functionality to look at the codebase and gain insights that will provide
//! statistics and inputs into subsequent review.
use crate::review::data::LanguageFileType;
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

// TODO: "security_issues": [
//         {
//           "code": "count_lines_of_code function",
//           "threat": "Potential denial of service by processing extremely large files or files with very long lines, as line read is unrestricted.",
//           "mitigation": "Implement file size and line length limits, and consider processing the file in chunks to protect against out-of-memory conditions."
//         },
//         {
//           "code": "analyse_file_language function (line 54)",
//           "threat": "Resolving symlinks without checks can lead to symlink attacks in certain contexts.",
//           "mitigation": "Ensure the resolution of the symbolic links is safe or avoid following symlinks by checking the `DirEntry` type."
//         }
//       ],

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

pub(crate) struct FileInfo {
    pub(crate) contents: Arc<OsString>,
    pub(crate) name: Arc<OsString>,
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
    // TODO: resolve the type of file if docs, dotfile, or config
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
                    language: language.clone(),
                    extension: extension.clone(),
                    percentage,
                    loc,
                    total_size: size,
                    file_count: count,
                });
            }
        }
        types
    }
}
