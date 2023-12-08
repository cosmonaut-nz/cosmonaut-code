//! Functions to gather data on the 'git' repository, files and contributors

/// Functions to gather data on the 'git' repository
pub(crate) mod repository {
    use log::{debug, warn};
    use std::fs;
    use std::path::Path;
    use walkdir::DirEntry;
    /// Checks whether the dir passed in is on the blacklist, e.g., '.git'
    pub(crate) fn is_not_blacklisted(entry: &DirEntry, blacklist: &[String]) -> bool {
        // Not in the blacklist
        !entry
            .file_name()
            .to_str()
            .map(|s| blacklist.contains(&s.to_string()))
            .unwrap_or(false)
    }
    /// Gets the the blacklist from either defaults or dynamically from '.gitignore'
    pub(crate) fn get_blacklist_dirs(repo_path: &Path) -> Vec<String> {
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
                            blacklist.push(line.trim_matches('/').to_string());
                        }
                    }
                }
            } else {
                warn!("Failed to read '.gitignore' file");
            }
        }
        blacklist
    }
    /// Adds the character class pattern to the blacklist
    fn handle_character_class_pattern(blacklist: &mut Vec<String>, line: &str) {
        // Trying to match patterns like '[Rr]elease/' found in '.gitignore' files
        if line.starts_with("[Rr]") && line.ends_with('/') {
            let base = &line[4..line.len() - 1]; // Remove [Rr] and trailing '/'
            debug!("Adding: '{}' to blacklist.", line);
            blacklist.push(format!("R{}", base));
            blacklist.push(format!("r{}", base));
        }
    }
}

/// Functions to gather data on the 'git' files
pub(crate) mod file {
    use git2::Repository;

    pub(crate) fn get_file_change_frequency(
        repo_path: &str,
        file_path: &str,
    ) -> Result<(usize, usize, f32), git2::Error> {
        let repo = Repository::open(repo_path)?;
        let mut revwalk = repo.revwalk()?;
        revwalk.push_head()?;

        let mut total_commits = 0;
        let mut file_commits = 0;

        for commit_id in revwalk {
            let commit = repo.find_commit(commit_id?)?;
            total_commits += 1;

            if commit.parent_count() > 0 {
                let parent = commit.parent(0)?;
                let commit_tree = commit.tree()?;
                let parent_tree = parent.tree()?;

                let diff = repo.diff_tree_to_tree(Some(&parent_tree), Some(&commit_tree), None)?;
                diff.foreach(
                    &mut |delta, _| {
                        let filepath = delta
                            .new_file()
                            .path()
                            .unwrap_or(delta.old_file().path().unwrap());
                        if filepath.to_str() == Some(file_path) {
                            file_commits += 1;
                        }
                        true
                    },
                    None,
                    None,
                    None,
                )?;
            }
        }

        let frequency = file_commits as f32 / total_commits as f32;

        Ok((file_commits, total_commits, frequency))
    }
}

/// Functions to gather data on the 'git' contributors
pub(crate) mod contributor {
    use crate::review::data::Contributor;
    use chrono::{DateTime, NaiveDateTime, Utc};
    use git2::Repository;
    use std::collections::HashMap;
    /// Gets the contributors from the repository passed as the 'repo_path'
    pub(crate) fn get_git_contributors(repo_path: &str) -> Vec<Contributor> {
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

                let entry = contributions.entry(name).or_insert((date, 0));
                entry.1 += 1; // Increment contribution count
                if date > entry.0 {
                    entry.0 = date; // Update last contribution date if newer
                }
                total_contributions += 1;
            }
        }
        contributions
            .into_iter()
            .map(|(name, (last_contribution, num_commits))| {
                let percentage =
                    (num_commits as f32 / total_contributions as f32 * 100.0).round() as i32;
                Contributor::new(name, num_commits, last_contribution, percentage)
            })
            .collect()
    }
}
