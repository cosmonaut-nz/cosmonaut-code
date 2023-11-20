//! A set of tools, constants and functions to help review a software repository.
//!
use log::debug;
use std::fs;
use std::path::Path;
use walkdir::DirEntry;

/// Checks whether a file is valid code or configuration to review
pub fn is_valid_extension(ext: &str) -> bool {
    // TODO: Define valid extensions comprehensively
    let valid_extensions = ["rs", "py", "js", "cs", "yml"]; // Example extensions
    valid_extensions.contains(&ext)
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
    let mut blacklist = vec![String::from(".git"), String::from("/settings")];

    // Path to the `.gitignore` file
    let gitignore_path = repo_path.join(".gitignore");

    if gitignore_path.exists() {
        debug!("Collecting .gitignore entries");
        if let Ok(contents) = fs::read_to_string(gitignore_path) {
            for line in contents.lines() {
                if !line.starts_with('#') && !line.trim().is_empty() {
                    // Simple check for directories (ending with '/')
                    // You can enhance this to handle more complex patterns
                    debug!("Adding: '{}' to blacklist.", line);
                    blacklist.push(line.trim_matches('/').to_string());
                }
            }
        }
    }

    blacklist
}
