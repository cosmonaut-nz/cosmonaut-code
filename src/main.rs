//!
//!
//!
//!
//!
mod common;
mod provider;
mod review;
mod settings;
use log::{debug, error, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Load settings
    let settings = match settings::Settings::new() {
        Ok(cfg) => cfg,
        Err(e) => {
            error!("Failed to load settings: {}", e);
            std::process::exit(1); // Exit if settings cannot be loaded
        }
    };
    debug!("Settings loaded: {:?}", settings);

    // TODO: Wire up CLI here.

    // Call the assess_codebase, according to user configuration, either from commandline, or json settings files.
    review::assess_codebase(settings).await?;

    info!("Code review complete. See the output report for details.");

    Ok(())
}
