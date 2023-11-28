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
use std::time::{Duration, Instant};

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
    review::assess_codebase(settings).await?;

    info!("CODE REVIEW COMPLETE. See the output report for details.");

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
