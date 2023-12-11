//! This module provides tests to allow testing of flow and loading local data for merging and
//! re-play against LLMs or static analysis tools - i.e. breaks up the overall process and uses test data to reduce feedback loop
//! Note: this code will _NOT_ be included in the release binary
#[cfg(debug_assertions)]
pub mod comment_summary {
    use crate::review::data::{RepositoryReview, ReviewSummary};
    use crate::review::summarise_review_summaries;
    use crate::settings::Settings;
    use log::{info, warn};
    use std::fs::File;
    use std::io::Read;

    pub(crate) async fn _test_summary(
        settings: &Settings,
    ) -> Result<(), Box<dyn std::error::Error>> {
        info!("Mod: Testing summary creation.");

        let repo_review = _deserialize_repository_review_from(
            settings
                .developer_mode
                .clone()
                .unwrap()
                .test_json_file
                .unwrap(),
        )?;

        // Create a [`ReviewSummary`]
        let mut review_summary: ReviewSummary = repo_review.summary.unwrap().clone();

        for review in repo_review.file_reviews {
            review_summary.text.push_str(&review.summary);
            review_summary.text.push('\n');
        }

        match summarise_review_summaries(settings, &review_summary).await {
            Ok(Some(summary)) => {
                info!("Revised summary: \n{}\n", summary);
            }
            Ok(None) => {
                warn!("Summary response was returned as 'None'!");
            }
            Err(e) => return Err(e),
        };

        Ok(())
    }
    pub(crate) fn _deserialize_repository_review_from(
        file_path: String,
    ) -> Result<RepositoryReview, Box<dyn std::error::Error>> {
        let mut file = File::open(file_path)?;
        let mut json_data = String::new();
        file.read_to_string(&mut json_data)?;

        let repo_review: RepositoryReview = serde_json::from_str(&json_data)?;
        Ok(repo_review)
    }
}

#[cfg(debug_assertions)]
pub mod _code_frequency {
    use crate::{
        retrieval::{data::SourceFileChangeFrequency, git::source_file::get_file_change_frequency},
        settings::Settings,
    };

    pub(crate) fn test_code_frequency(
        settings: &Settings,
    ) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Mod: Testing code frequency.");

        let repo_path = settings.repository_path.clone();
        // TODO: iterate over a set of files and determine the overall frequency ranking (top five) and also the average frequency (into the repository)
        let file_path = "src/review/mod.rs";

        let fcf: SourceFileChangeFrequency = get_file_change_frequency(&repo_path, file_path)?;

        log::info!(
            "File commits: {}, total commits: {}, frequency: {}",
            fcf.file_commits,
            fcf.total_commits,
            fcf.frequency
        );

        Ok(())
    }
}
