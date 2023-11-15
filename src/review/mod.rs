//! Handles the file in a software repository. Iterates over the folder structure, ignoring files or folders that are not relevant. Passes each relevant (code) file for review.

use crate::data::{FileReview, RAGStatus, RepositoryReview};
use crate::provider;
use crate::settings::Settings;
use chrono::{DateTime, Local, Utc};
use log::debug;
use log::info;
use std::error::Error;
use std::fmt;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use walkdir::WalkDir;

/// Takes the filepath to a repository and iterates over the code, sending each relevant file for review.
///
/// # Parameters
///
/// * `settings` - A ['Settings'] that contains information for the LLM
pub async fn assess_codebase(settings: Settings) -> Result<(), Box<dyn std::error::Error>> {
    // Used for the final report to write to disk
    let output_dir = PathBuf::from(&settings.report_output_path);
    let output_file_path =
        create_timestamped_filename(&output_dir, &settings.output_type, Local::now());

    // Collect the review data in the following data struct
    let mut review = RepositoryReview::new();
    match extract_directory_name(&settings.repository_path) {
        Ok(dir_name) => review.set_repository_name(dir_name.to_string()),
        Err(e) => eprintln!("Error extracting directory name: {}", e),
    }
    review.set_date(Utc::now());
    review.set_repository_purpose("purpose".to_string()); // TODO: Derive this from playing the README at the LLM
    review.set_summary("summary".to_string()); // TODO: Pull together all the filereview summaries and send to LLM for condensing
    review.set_repository_rag_status(RAGStatus::Green); // TODO: Derive from summing up the RAG statuses in the filereviews and calculate...
    review.set_contributors(Vec::new()); // TODO: Derive from the `git` statistics in `git log`
    review.set_code_types(Vec::new()); // TODO: Pull together a list from the files sent through for review, append then work out percentage

    // Walk the repository structure sending relevant files to the provider ai service to review
    for entry in WalkDir::new(&settings.repository_path) {
        let entry: walkdir::DirEntry = entry?;
        let path: &Path = entry.path();
        if path.is_file() {
            // TODO check extension to see if it is valid for review
            let review_report = review_file(&settings, path).await?;
            review.add_file_review(review_report);
        } else {
            // TODO: add in whitelisted directories, such as "src" only
            debug!("Directory {}.", path.display());
        }
    }

    // Serialize the review struct to JSON
    let review_json = serde_json::to_string_pretty(&review)
        .map_err(|e| format!("Error serializing review: {}", e))?;

    // Write the JSON to the specified output file
    let mut output_file = fs::File::create(&output_file_path)
        .map_err(|e| format!("Error creating output file: {}", e))?;
    output_file
        .write_all(review_json.as_bytes())
        .map_err(|e| format!("Error writing to output file: {}", e))?;

    Ok(())
}

/// Pulls the text from a ['File'] and sends it to the LLM for review
///
/// This function takes two integer parameters and returns their sum.
/// It demonstrates basic arithmetic operations in Rust.
///
/// # Parameters
///
/// * `Settings` - A ['Settings'] that contains information for the LLM
/// * `path` - - The path the the file to process
///
async fn review_file(
    settings: &Settings,
    path: &Path,
) -> Result<FileReview, Box<dyn std::error::Error>> {
    info!("Handling output_file: {}", path.display());

    let code_from_file: String = fs::read_to_string(path)?;
    let review_request: String = format!("File name: {}\n{}\n", path.display(), code_from_file);
    // Pass the file to be reviewed by the LLM service
    let review_report: FileReview =
        provider::review_file_via_llm(settings, &review_request).await?;

    Ok(review_report)
}

/// Creates a timestamped file
///
/// # Parameters
///
/// * `base_path` - where the file will be created
/// * `file_extension` - the file extension, e.g., '.json'
/// * `timestamp` - the current time, makes testing easier to mock. Example input: 'Local::now()'
fn create_timestamped_filename(
    base_path: &Path,
    file_extension: &str,
    timestamp: DateTime<Local>,
) -> PathBuf {
    let formatted_timestamp = timestamp.format("%Y%m%d_%H%M%S").to_string();
    base_path.join(format!("{}.{}", formatted_timestamp, file_extension))
}

/// extracts the final part of the path
///
/// # Parameters
///
/// * `path_str` - a str representation of the path
#[derive(Debug)]
pub struct PathError {
    message: String,
}

impl PathError {
    fn new(message: &str) -> PathError {
        PathError {
            message: message.to_string(),
        }
    }
}

impl fmt::Display for PathError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Path error: {}", self.message)
    }
}

impl Error for PathError {}

fn extract_directory_name(path_str: &str) -> Result<&str, PathError> {
    let path = Path::new(path_str);

    // Check if the path points to a file (has an extension)
    if path.extension().is_some() && path.is_file() {
        return Err(PathError::new("Path points to a file, not a directory"));
    }

    let dir_name = if path.ends_with("/") {
        path.parent().and_then(|p| p.file_name())
    } else {
        path.file_name()
    };

    dir_name
        .and_then(|os_str| os_str.to_str())
        .ok_or_else(|| PathError::new("Invalid directory name"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_create_timestamped_filename() {
        let base_path = PathBuf::from("/some/path");
        let file_extension = "txt";
        let mock_time = Local.with_ymd_and_hms(2022, 4, 1, 12, 30, 45).unwrap();

        let result = create_timestamped_filename(&base_path, file_extension, mock_time);

        // Test that the result is in the correct directory
        assert_eq!(result.parent(), Some(base_path.as_path()));

        // Test the file extension
        assert_eq!(
            result.extension(),
            Some(std::ffi::OsStr::new(file_extension))
        );

        // Test the structure and correctness of the filename
        let expected_filename = format!("20220401_123045.{}", file_extension);
        assert_eq!(
            result.file_name().unwrap().to_str().unwrap(),
            &expected_filename
        );
    }

    #[test]
    fn test_normal_directory_path() {
        let path_str = "/location/dirname/cosmonaut-code";
        assert_eq!(extract_directory_name(path_str).unwrap(), "cosmonaut-code");
    }

    #[test]
    fn test_empty_path() {
        let path_str = "";
        assert!(extract_directory_name(path_str).is_err());
    }

    #[test]
    fn test_path_ending_with_slash() {
        let path_str = "/location/dirname/cosmonaut-code/";
        assert_eq!(extract_directory_name(path_str).unwrap(), "cosmonaut-code");
    }

    #[test]
    fn test_single_name_directory() {
        let path_str = "cosmonaut-code";
        assert_eq!(extract_directory_name(path_str).unwrap(), "cosmonaut-code");
    }
}
