//! contains functionality to look at the codebase and gain insights that will provide
//! statistics and inputs into subsequent review.
use crate::review::data::LanguageFileType;
use linguist::{
    container::InMemoryLanguageContainer,
    resolver::{resolve_language, Scope},
    utils::{is_configuration, is_documentation, is_dotfile, is_vendor},
};
use regex::RegexSet;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, os::unix::prelude::MetadataExt, path::Path};
use walkdir::WalkDir;

pub mod predefined {
    include!(concat!(env!("OUT_DIR"), "/languages.rs"));
    include!(concat!(env!("OUT_DIR"), "/heuristics.rs"));
    include!(concat!(env!("OUT_DIR"), "/vendors.rs"));
    include!(concat!(env!("OUT_DIR"), "/documentation.rs"));
}

pub fn analyse_languages_in_repository(repo_path: &Path) -> Vec<LanguageFileType> {
    let mut lc = InMemoryLanguageContainer::default();
    for &lang in predefined::LANGUAGES.iter() {
        lc.register_language(lang);
    }

    for &rule in predefined::HEURISTICS.iter() {
        lc.register_heuristic_rule(rule);
    }
    let mut breakdown = LanguageBreakdown {
        usages: HashMap::new(),
        total_size: 0,
    };
    let rules = RegexSet::new(predefined::VENDORS).unwrap();
    let docs = RegexSet::new(predefined::DOCUMENTATION).unwrap();

    let walker = WalkDir::new(repo_path);
    for entry in walker.into_iter().flatten() {
        if entry.path().is_dir() {
            continue;
        }
        let relative_path = entry.path().strip_prefix(repo_path).unwrap();
        if is_vendor(entry.path(), &rules)
            || is_documentation(relative_path, &docs)
            || is_dotfile(relative_path)
            || is_configuration(relative_path)
        {
            continue;
        }
        let language: &linguist::resolver::Language = match resolve_language(entry.path(), &lc) {
            Ok(Some(lang)) => lang,
            _ => continue,
        };
        if language.scope != Scope::Programming && language.scope != Scope::Markup {
            continue;
        }
        let file_size = entry.metadata().unwrap().size();
        let extension = entry
            .path()
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        breakdown.add_usage(&language.name, &extension, file_size);
    }

    breakdown.to_language_file_types()
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct LanguageBreakdown {
    usages: HashMap<String, HashMap<String, (u64, i32)>>, // Track both size and count
    total_size: u64,
}

impl LanguageBreakdown {
    pub fn add_usage(&mut self, lang: &str, ext: &str, size: u64) {
        let language_entry = self.usages.entry(lang.to_string()).or_default();
        let entry = language_entry.entry(ext.to_string()).or_insert((0, 0));
        entry.0 += size; // Increase size
        entry.1 += 1; // Increment file count
        self.total_size += size;
    }
    pub fn to_language_file_types(&self) -> Vec<LanguageFileType> {
        let mut types = Vec::new();

        for (language, extensions) in &self.usages {
            for (extension, &(size, count)) in extensions {
                let percentage = (size as f64 / self.total_size as f64) * 100.0;
                types.push(LanguageFileType {
                    language: language.clone(),
                    extension: extension.clone(),
                    percentage,
                    total_size: size,
                    file_count: count,
                });
            }
        }
        types
    }
}
