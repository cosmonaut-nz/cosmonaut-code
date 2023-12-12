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
pub mod code_frequency {
    use crate::{
        retrieval::{
            data::SourceFileChangeFrequency,
            git::{repository::get_total_commits, source_file::get_source_file_change_frequency},
        },
        settings::Settings,
    };

    pub(crate) fn _test_total_commits(
        settings: &Settings,
    ) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Mod: Testing total commits.");

        let repo_path = settings.repository_path.clone();
        let total_commits: i32 = get_total_commits(&repo_path)?;

        log::info!("Total commits: {}", total_commits);

        Ok(())
    }

    pub(crate) fn _test_code_frequency(
        settings: &Settings,
    ) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Mod: Testing code frequency.");

        let repo_path = settings.repository_path.clone();
        // TODO: iterate over a set of files and determine the overall frequency ranking (top five) and also the average frequency (into the repository)
        let file_path = "src/review/mod.rs";

        let fcf: SourceFileChangeFrequency =
            get_source_file_change_frequency(&repo_path, file_path)?;

        log::info!(
            "File commits: {}, total commits: {}, frequency: {}",
            fcf.file_commits,
            fcf.total_commits,
            fcf.frequency
        );

        Ok(())
    }
}

#[cfg(debug_assertions)]
pub mod test_settings {
    use crate::{provider::get_service_and_model, settings::Settings};
    use log::info;

    pub(crate) fn _test_provider_settings(
        settings: &Settings,
    ) -> Result<(), Box<dyn std::error::Error>> {
        info!("Mod: Testing settings.");

        info!("Settings: {:?}", settings);

        info!("chosen_provider: {:?}", settings.chosen_provider);
        info!("chosen_service: {:?}", settings.chosen_service);

        info!("Provider settings: {:?}", get_service_and_model(settings));

        Ok(())
    }
}

#[cfg(debug_assertions)]
pub mod _test_utils {
    use log::debug;

    /// A (testing) utility to check the JSON sent back from the LLM
    pub(crate) fn _pretty_print_json_for_debug(json_str: &str) {
        match serde_json::from_str::<serde_json::Value>(json_str) {
            Ok(json_value) => {
                if let Ok(pretty_json) = serde_json::to_string_pretty(&json_value) {
                    debug!("{}", pretty_json);
                } else {
                    debug!("Failed to pretty-print JSON. Likely mangled JSON.");
                }
            }
            Err(e) => {
                debug!("Cannot parse the JSON: {}", json_str);
                debug!(
                    "Failed to parse JSON for debug pretty printing. Likely mangled JSON: {}",
                    e
                );
            }
        }
    }
}
