//! Produces reports in various formats according to [`OutputType`].
use chrono::Local;
use handlebars::Handlebars;
use log::info;
use serde::Deserialize;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::DateTime;

use crate::settings::Settings;

use super::data::RepositoryReview;

const HTML_TEMPLATE: &str = include_str!("./templates/report_template.html");

/// Creates an HTML report based on the [`RepositoryReview`] passed in.
/// Returns the path of the output report
/// TODO Not very DRY this and [`create_json_report`]
pub(crate) fn create_html_report(
    settings: &Settings,
    repository_review: &RepositoryReview,
) -> Result<String, Box<dyn std::error::Error>> {
    let output_dir: PathBuf = PathBuf::from(&settings.report_output_path);
    let output_file_path: PathBuf = create_named_timestamped_filename(
        &output_dir,
        &repository_review.repository_name,
        &settings.output_type,
        Local::now(),
    );
    let report_filepath = output_file_path.clone().to_string_lossy().into_owned();

    let mut output_file = fs::File::create(output_file_path)
        .map_err(|e| format!("Error creating output file: {}", e))?;

    let mut handlebars = Handlebars::new();

    handlebars
        .register_template_string("repository review", HTML_TEMPLATE)
        .unwrap();

    let rendered_html = handlebars
        .render("repository review", &repository_review)
        .unwrap();

    output_file
        .write_all(rendered_html.as_bytes())
        .map_err(|e| format!("Error writing to output file: {}", e))?;

    Ok(report_filepath)
}
/// Creates a JSON-based report based on the [`RepositoryReview`] passed in.
/// Returns the path of the output report
/// TODO Not very DRY this and [`create_html_report`]
pub(crate) fn create_json_report(
    settings: &Settings,
    repository_review: &RepositoryReview,
) -> Result<String, Box<dyn std::error::Error>> {
    let output_dir: PathBuf = PathBuf::from(&settings.report_output_path);
    let output_file_path: PathBuf = create_named_timestamped_filename(
        &output_dir,
        &repository_review.repository_name,
        &settings.output_type,
        Local::now(),
    );
    let report_filepath = output_file_path.clone().to_string_lossy().into_owned();

    let review_json = serde_json::to_string_pretty(&repository_review)
        .map_err(|e| format!("Error serializing review: {}", e))?;

    let mut output_file = fs::File::create(output_file_path)
        .map_err(|e| format!("Error creating output file: {}", e))?;
    output_file
        .write_all(review_json.as_bytes())
        .map_err(|e| format!("Error writing to output file: {}", e))?;

    Ok(report_filepath)
}

#[derive(Debug, Deserialize, Default, PartialEq)]
pub(crate) enum OutputType {
    #[default]
    Json,
    Pdf,
    Html,
}
/// We offer three types of output:
/// json. A review report in raw JSON
/// pdf. A review report in PDF format
/// html. A review report in HTML format
impl OutputType {
    pub(crate) fn from_config(settings: &Settings) -> Self {
        let output_type = settings.output_type.to_string();
        match output_type.as_str() {
            "json" => OutputType::Json,
            "pdf" => OutputType::Pdf,
            "html" => OutputType::Html,
            _ => {
                info!("Using default: {:?}", OutputType::default());
                OutputType::default()
            }
        }
    }
    pub(crate) fn create_report(
        &self,
        settings: &Settings,
        review: &RepositoryReview,
    ) -> Result<String, ReportError> {
        match self {
            OutputType::Json => {
                create_json_report(settings, review).map_err(|_| ReportError::NotImplemented)
            }
            OutputType::Pdf => Err(ReportError::NotImplemented),
            OutputType::Html => {
                create_html_report(settings, review).map_err(|_| ReportError::NotImplemented)
            }
        }
    }
}

/// Creates a timestamped file
///
/// # Parameters
///
/// * `base_path` - where the file will be created
/// * `repo_name` - the name of the repository being reviewed
/// * `file_extension` - the file extension, e.g., '.json'
/// * `timestamp` - the current time, makes testing easier to mock. Example input: 'Local::now()'
fn create_named_timestamped_filename(
    base_path: &Path,
    repo_name: &str,
    file_extension: &str,
    timestamp: DateTime<Local>,
) -> PathBuf {
    let formatted_timestamp = timestamp.format("%Y%m%d_%H%M%S").to_string();
    base_path.join(format!(
        "{}-{}.{}",
        repo_name, formatted_timestamp, file_extension
    ))
}

#[derive(Debug)]
pub(crate) enum ReportError {
    NotImplemented,
}
impl Display for ReportError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ReportError::NotImplemented => write!(f, "Feature not implemented"),
        }
    }
}

impl Error for ReportError {}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    #[test]
    fn test_create_named_timestamped_filename() {
        let base_path = PathBuf::from("/some/path");
        let file_extension = "txt";
        let mock_time = Local.with_ymd_and_hms(2022, 4, 1, 12, 30, 45).unwrap();

        let result =
            create_named_timestamped_filename(&base_path, "repos_name", file_extension, mock_time);

        // Test that the result is in the correct directory
        assert_eq!(result.parent(), Some(base_path.as_path()));

        // Test the file extension
        assert_eq!(
            result.extension(),
            Some(std::ffi::OsStr::new(file_extension))
        );

        // Test the structure and correctness of the filename
        let expected_filename = format!("repos_name-20220401_123045.{}", file_extension);
        assert_eq!(
            result.file_name().unwrap().to_str().unwrap(),
            &expected_filename
        );
    }
}
