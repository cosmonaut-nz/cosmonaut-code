//!
//!
//!
//!
//!
mod data;
mod provider;
mod review;
mod settings;
mod ui;
use log::error;

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

    // TODO: move the code into the UI handler
    review::assess_codebase(settings).await?;
    // Run the UI
    // TODO: eventually, this will be the root action for the application
    ui::run();

    Ok(())
}
