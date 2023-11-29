//!
//!
//!
//!
//!
mod common;
mod provider;
mod review;
mod settings;
use log::{error, info};
use std::{
    process::Command,
    time::{Duration, Instant},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let start = Instant::now();
    env_logger::init();

    // Load settings
    let settings: settings::Settings = match settings::Settings::new() {
        Ok(cfg) => cfg,
        Err(e) => {
            error!("Failed to load settings: {}", e);
            // Cannot recover due to incomplete configuration
            std::process::exit(1);
        }
    };

    // Call the assess_codebase, according to user configuration, either from commandline, or json settings files.
    let report_output = review::assess_codebase(settings).await?;

    info!("CODE REVIEW COMPLETE. See the output report for details.");
    if let Err(e) = open_file(&report_output) {
        error!("Failed to open file: {}", e);
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
fn open_file(file_path: &str) -> std::io::Result<()> {
    if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(["/C", "start", file_path])
            .spawn()?;
    } else if cfg!(target_os = "macos") {
        Command::new("open").arg(file_path).spawn()?;
    } else if cfg!(target_os = "linux") {
        Command::new("xdg-open").arg(file_path).spawn()?;
    } else {
        println!("Unsupported OS");
    }

    Ok(())
}
