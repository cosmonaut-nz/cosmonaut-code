//! Produces reports in various formats according to [`OutputType`].
use super::data::RepositoryReview;
use crate::settings::Settings;
use chrono::DateTime;
use chrono::{Local, Utc};
use handlebars::{Context, Handlebars, Helper, HelperResult, Output, RenderContext};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

const HTML_TEMPLATE: &str = include_str!("./templates/report_template.html");

/// Creates and outputs a report for the [`Settings`] and [`RepositoryReview`] passed in
/// The function the renders according to [`OutputType`]
pub(crate) fn create_report(
    settings: &Settings,
    repository_review: &RepositoryReview,
) -> Result<String, Box<dyn std::error::Error>> {
    match settings.output_type {
        OutputType::Json => create_specific_report(repository_review, render_json, settings),
        OutputType::Html => create_specific_report(repository_review, render_html, settings),
        OutputType::Pdf => Err(Box::new(ReportError::NotImplemented)),
    }
}

/// There may be multiple report formats, so here we handle according, according to [`OutputType`]
fn create_specific_report<F>(
    repository_review: &RepositoryReview,
    render_fn: F,
    settings: &Settings,
) -> Result<String, Box<dyn std::error::Error>>
where
    F: Fn(&RepositoryReview, &Settings) -> Result<String, Box<dyn std::error::Error>>,
{
    let output_dir: PathBuf = PathBuf::from(&settings.report_output_path);
    let output_file_path: PathBuf = create_named_timestamped_filename(
        &output_dir,
        &repository_review.repository_name,
        &settings.output_type.to_string(),
        Local::now(),
    );
    let report_filepath = output_file_path.clone().to_string_lossy().into_owned();

    let output_content = render_fn(repository_review, settings)?;

    let mut output_file = fs::File::create(output_file_path)
        .map_err(|e| format!("Error creating output file: {}", e))?;
    output_file
        .write_all(output_content.as_bytes())
        .map_err(|e| format!("Error writing to output file: {}", e))?;

    Ok(report_filepath)
}

fn render_json(
    repository_review: &RepositoryReview,
    _settings: &Settings,
) -> Result<String, Box<dyn std::error::Error>> {
    serde_json::to_string_pretty(repository_review).map_err(|e| {
        Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Error serializing review: {}", e),
        )) as Box<dyn std::error::Error>
    })
}

fn render_html(
    repository_review: &RepositoryReview,
    _settings: &Settings,
) -> Result<String, Box<dyn std::error::Error>> {
    let current_year = Utc::now().format("%Y").to_string();
    let mut handlebars = Handlebars::new();
    handlebars.register_helper("format_percentage", Box::new(format_percentage));
    handlebars
        .register_template_string("repository review", HTML_TEMPLATE)
        .unwrap();
    let context = ReportContext {
        repository_review,
        current_year,
    };
    handlebars
        .render("repository review", &context)
        .map_err(|e| {
            Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Error rendering HTML: {}", e),
            )) as Box<dyn std::error::Error>
        })
}

#[derive(Debug, Serialize, Deserialize, Default, PartialEq)]
pub(crate) enum OutputType {
    #[serde(rename = "json")]
    #[default]
    Json,
    #[serde(rename = "pdf")]
    Pdf,
    #[serde(rename = "html")]
    Html,
}
impl fmt::Display for OutputType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                OutputType::Json => "json",
                OutputType::Pdf => "pdf",
                OutputType::Html => "html",
            }
        )
    }
}

/// Handlebars Helper to round a `f64` to two decimal places
fn format_percentage(
    h: &Helper<'_, '_>,
    _: &Handlebars<'_>,
    _: &Context,
    _: &mut RenderContext<'_, '_>,
    out: &mut dyn Output,
) -> HelperResult {
    let param = h.param(0).and_then(|v| v.value().as_f64()).unwrap_or(0.0);
    write!(out, "{:.2}", param)?;
    Ok(())
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

#[derive(Serialize)]
pub(crate) struct ReportContext<'a> {
    pub repository_review: &'a RepositoryReview,
    pub current_year: String,
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
