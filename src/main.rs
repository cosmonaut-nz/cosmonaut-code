//!
//!
//!
#[cfg(debug_assertions)]
mod dev_mode;

mod common;
mod provider;
mod retrieval;
mod review;
mod settings;
use log::{error, info};
use std::time::{Duration, Instant};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let start = Instant::now();
    env_logger::init();

    // Load settings
    let settings: settings::Settings = settings::Settings::new()?;
    // Should have the settings fully loaded and trusted at this point.

    #[cfg(debug_assertions)]
    {
        if !settings
            .developer_mode
            .clone()
            .is_some_and(|dev_path| dev_path.test_path)
        {
            let report_output = review::assess_codebase(settings).await?;

            info!("CODE REVIEW COMPLETE. See the output report for details.");
            if let Err(e) = open_file_or_files(&report_output) {
                error!("Failed to open file: {}", e);
            }
        } else {
            info!("Taking developer path.");

            // dev_mode::code_frequency::_test_total_commits(&settings)?;
            // dev_mode::code_frequency::_test_code_frequency(&settings)?;
            // dev_mode::_comment_summary::test_summary(&settings).await?;
            // dev_mode::test_settings::_test_provider_settings(&settings)?;
            // dev_mode::test_providers::_test_local_provider(&settings).await?;
            dev_mode::test_providers::_test_google_provider(&settings).await?;
        }
    }

    #[cfg(not(debug_assertions))]
    {
        // Call the assess_codebase, according to user configuration, either from environment variables, or json settings files.
        review::assess_codebase(settings).await?;
    }
    print_exec_duration(start.elapsed());
    Ok(())
}

/// prints the execution time for the application at info log level
fn print_exec_duration(duration: Duration) {
    let duration_secs = duration.as_secs();
    let minutes = duration_secs / 60;
    let seconds = duration_secs % 60;
    let millis = duration.subsec_millis();

    info!(
        "TOTAL EXECUTION TIME: {} minutes, {} seconds, and {} milliseconds",
        minutes, seconds, millis
    );
}
#[cfg(debug_assertions)]
fn open_file_or_files(file_paths: &str) -> std::io::Result<()> {
    for file_path in file_paths.split(',') {
        if cfg!(target_os = "windows") {
            std::process::Command::new("cmd")
                .args(["/C", "start", file_path.trim()])
                .spawn()?;
        } else if cfg!(target_os = "macos") {
            std::process::Command::new("open")
                .arg(file_path.trim())
                .spawn()?;
        } else if cfg!(target_os = "linux") {
            std::process::Command::new("xdg-open")
                .arg(file_path.trim())
                .spawn()?;
        } else {
            println!("Unsupported OS");
        }
    }

    Ok(())
}
