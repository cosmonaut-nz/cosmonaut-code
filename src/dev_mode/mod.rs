//! This module provides tests to allow testing of flow and loading local data for merging and
//! re-play against LLMs or static analysis tools - i.e. breaks up the overall process and uses test data to reduce feedback loop
//! Note: this code will _NOT_ be included in the release binary
#[cfg(debug_assertions)]
pub mod comment_summary {
    use crate::review::data::{RepositoryReview, ReviewBreakdown};
    use crate::review::summarise_review_breakdown;
    use crate::settings::Settings;
    use log::{info, warn};
    use std::fs::File;
    use std::io::Read;

    pub(crate) async fn test_summary(
        settings: &Settings,
    ) -> Result<(), Box<dyn std::error::Error>> {
        info!("Mod: Testing summary creation.");

        let repo_review = deserialize_repository_review_from(
            settings
                .developer_mode
                .clone()
                .unwrap()
                .test_json_file
                .unwrap(),
        )?;

        // Create a [`ReviewBreakdown`]
        let mut review_breakdown: ReviewBreakdown = repo_review.summary.unwrap().clone();

        for review in repo_review.file_reviews {
            review_breakdown.summary.push_str(&review.summary);
            review_breakdown.summary.push('\n');
        }

        // info!("Pre summary: {}", &review_breakdown.summary);

        match summarise_review_breakdown(settings, &review_breakdown).await {
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
    pub(crate) fn deserialize_repository_review_from(
        file_path: String,
    ) -> Result<RepositoryReview, Box<dyn std::error::Error>> {
        let mut file = File::open(file_path)?;
        let mut json_data = String::new();
        file.read_to_string(&mut json_data)?;

        let repo_review: RepositoryReview = serde_json::from_str(&json_data)?;
        Ok(repo_review)
    }
}