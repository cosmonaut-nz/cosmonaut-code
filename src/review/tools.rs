//! A set of tools, constants and functions to help review a software repository.
//!
use super::data::Contributor;
use chrono::{DateTime, NaiveDateTime, Utc};
use git2::Repository;
use log::debug;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use walkdir::DirEntry;

pub struct LanguageFileTypes {
    types: HashMap<String, HashSet<String>>,
}

impl LanguageFileTypes {
    pub fn new() -> Self {
        let mut types = HashMap::new();
        // Rust
        types.insert(
            "Rust".to_string(),
            ["rs", "toml", "md"]
                .iter()
                .map(|s| s.to_string())
                .collect::<HashSet<String>>(),
        );
        // Python
        types.insert(
            "Python".to_string(),
            ["py", "md", "txt", "ini"]
                .iter()
                .map(|s| s.to_string())
                .collect::<HashSet<String>>(),
        );
        // Java
        types.insert(
            "Java".to_string(),
            ["java", "xml", "gradle", "md", "properties"]
                .iter()
                .map(|s| s.to_string())
                .collect::<HashSet<String>>(),
        );
        // Go
        types.insert(
            "Go".to_string(),
            ["go", "mod", "sum", "md"]
                .iter()
                .map(|s| s.to_string())
                .collect::<HashSet<String>>(),
        );
        // .Net
        types.insert(
            ".Net".to_string(),
            ["cs", "csproj", "sln", "config", "md", "xml", "ps1", "xaml"]
                .iter()
                .map(|s| s.to_string())
                .collect::<HashSet<String>>(),
        );
        // .Net Web
        types.insert(
            ".Net-Web".to_string(),
            [
                "cs", "cshtml", "razor", "js", "ts", "html", "css", "scss", "json", "md", "config",
                "xml", "aspx", "ascx",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect::<HashSet<String>>(),
        );
        // JavaScript (Node.js)
        types.insert(
            "JavaScript-Node".to_string(),
            ["js", "json", "md", "jsx"]
                .iter()
                .map(|s| s.to_string())
                .collect::<HashSet<String>>(),
        );
        // JavaScript (Web)
        types.insert(
            "JavaScript-Web".to_string(),
            ["js", "html", "css", "scss", "md"]
                .iter()
                .map(|s| s.to_string())
                .collect::<HashSet<String>>(),
        );
        // TypeScript
        types.insert(
            "TypeScript".to_string(),
            ["ts", "tsx", "json", "md"]
                .iter()
                .map(|s| s.to_string())
                .collect::<HashSet<String>>(),
        );
        // Ruby
        types.insert(
            "Ruby".to_string(),
            ["rb", "erb", "rake", "md", "gemspec"]
                .iter()
                .map(|s| s.to_string())
                .collect::<HashSet<String>>(),
        );

        Self { types }
    }
}

/// Checks whether a file is valid code or configuration to review
pub fn is_valid_extension(
    language_file_types: &LanguageFileTypes,
    language: &str,
    ext: &str,
) -> bool {
    if let Some(types) = language_file_types.types.get(language) {
        types.contains(ext)
    } else {
        false // Language not found
    }
}

/// Checks whether the dir passed in is on the blacklist, e.g., '.git'
pub fn is_not_blacklisted(entry: &DirEntry, blacklist: &[String]) -> bool {
    // Not in the blacklist
    !entry
        .file_name()
        .to_str()
        .map(|s| blacklist.contains(&s.to_string()))
        .unwrap_or(false)
}

/// Gets the the blacklist from either defaults or dynamically from '.gitignore'
pub fn get_blacklist_dirs(repo_path: &Path) -> Vec<String> {
    let mut blacklist = vec![String::from(".git")];

    // Path to the `.gitignore` file
    let gitignore_path = repo_path.join(".gitignore");

    if gitignore_path.exists() {
        debug!("Collecting .gitignore entries");
        if let Ok(contents) = fs::read_to_string(gitignore_path) {
            for line in contents.lines() {
                if !line.starts_with('#') && !line.trim().is_empty() {
                    // Simple check for directories (ending with '/')
                    if line.contains('[') && line.contains(']') {
                        // Manually expand the character class patterns
                        handle_character_class_pattern(&mut blacklist, line);
                    } else {
                        debug!("Adding: '{}' to blacklist.", line);
                        blacklist.push(line.trim_matches('/').to_string());
                    }
                }
            }
        }
    }

    blacklist
}

fn handle_character_class_pattern(blacklist: &mut Vec<String>, line: &str) {
    // Trying to match patterns like '[Rr]elease/' found in '.gitignore' files
    if line.starts_with("[Rr]") && line.ends_with('/') {
        let base = &line[4..line.len() - 1]; // Remove [Rr] and trailing '/'
        debug!("Adding: '{}' to blacklist.", line);
        blacklist.push(format!("R{}", base));
        blacklist.push(format!("r{}", base));
    }
}

/// Gets the contributors from the repository passed as the 'repo_path'
pub fn get_git_contributors(repo_path: &str) -> Vec<Contributor> {
    let repo = Repository::open(repo_path).expect("Failed to open repository");
    let mut revwalk = repo.revwalk().expect("Failed to get revwalk");
    revwalk.push_head().expect("Failed to push head");

    let mut contributions = HashMap::<String, (DateTime<Utc>, i32)>::new();
    let mut total_contributions = 0;

    for oid in revwalk {
        if let Ok(commit) = repo.find_commit(oid.expect("Invalid oid")) {
            let name = String::from(commit.author().name().unwrap_or_default());
            let time = commit.author().when();

            let naive_date_time = NaiveDateTime::from_timestamp_opt(time.seconds(), 0).unwrap();
            let date = DateTime::<Utc>::from_naive_utc_and_offset(naive_date_time, Utc);

            let entry: &mut (DateTime<Utc>, i32) = contributions.entry(name).or_insert((date, 0));
            entry.1 += 1; // Increment contribution count
            if date > entry.0 {
                entry.0 = date;
            } // Update last contribution date if newer
            total_contributions += 1;
        }
    }

    contributions
        .into_iter()
        .map(|(name, (last_contribution, count))| {
            let percentage = (count as f32 / total_contributions as f32 * 100.0).round() as i32;
            Contributor::new(name, last_contribution, percentage)
        })
        .collect()
}
