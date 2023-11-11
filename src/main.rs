//!
//!
//!
//!
//!
mod config;
mod model;
mod ui;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = config::Config::load();
    // TODO: move the code into the UI handler
    model::run_code_review(config).await?;
    ui::run();

    Ok(())
}
