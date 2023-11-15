//!
//! Settings for setting up:
//!     Service provider {OpenAI, Google, Other...}
//!     LLM API access
//!     Repository directory/folder location
//!
//!
use config::{Config, ConfigError, File};
use log::warn;
use serde::Deserialize;
use std::env;
use std::fmt;

const SETTINGS_FILE_PATH: &str = "settings";
const APP_ENV_PREFIX: &str = "COSMOCODE";

/// struct to hold the configuration
///
#[derive(Deserialize)]
pub struct Settings {
    // General configuration fields
    pub provider: Provider,
    pub output_type: String,
    pub review_type: i32,
    pub repository_path: String,
    pub report_output_path: String,

    // Sensitive data
    pub sensitive: SensitiveSettings,
}

#[derive(Deserialize)]
pub struct Provider {
    pub name: String,
    pub service: String,
    pub model: String,
    pub api_url: String,
    pub max_tokens: i64,
}
#[derive(Deserialize)]
pub struct SensitiveSettings {
    pub api_key: APIKey,
    pub org_id: String,
    pub org_name: String,
}
#[derive(Deserialize)]
pub struct APIKey(String); // Sensitive data

/// Custom Debug implementation for Settings
impl fmt::Debug for Settings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Settings")
            .field("provider", &self.provider)
            .field("output_type", &self.output_type)
            .field("review_type", &self.review_type)
            .field("repository_path", &self.repository_path)
            .field("report_output_path", &self.report_output_path)
            .field("sensitive", &"*** sensitive data hidden ***")
            .finish()
    }
}
/// Custom Display implementation for Settings
impl fmt::Display for Settings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Settings")
            .field("provider", &self.provider)
            .field("output_type", &self.output_type)
            .field("review_type", &self.review_type)
            .field("repository_path", &self.repository_path)
            .field("report_output_path", &self.report_output_path)
            .field("sensitive", &"*** sensitive data hidden ***")
            .finish()
    }
}
/// Configuration settings for the code review application.
///
/// This struct contains various settings used to configure
/// the behavior of the code review and analysis process.
///
/// # Fields
/// - `provider`: The organization providing the language model service (e.g., openai, etc.).
/// - `sensitive settings`: Inc. API key for authentication, org_id and org_name.
/// - `repository_path`: Path to the folder containing repository and code for analysis.
/// - `report_output_path`: Path where analysis output report will be stored.
/// - `output_type`: The format/type of the output (e.g., json, csv). Default is JSON.
/// - `review_type`: Numeric code indicating the type of review (e.g., 1 for general, 2 for security).
///
/// `review_type` and `output_type` have default values, but other fields must be explicitly set.
impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        // Enable dev mode usage
        let run_mode = env::var("RUN_MODE").unwrap_or_else(|_| {
            warn!("RUN_MODE not set, defaulting to 'development'");
            "development".into()
        });
        //Load the config,
        let config = Config::builder()
            // Start off by merging in the "default" configuration file
            .add_source(File::with_name(&format!("{}/default", SETTINGS_FILE_PATH)).required(false))
            .add_source(
                config::Environment::with_prefix(APP_ENV_PREFIX)
                    .try_parsing(true)
                    .separator("_"),
            )
            // Default to (optional) 'development' env
            .add_source(
                File::with_name(&format!("{}/{}", SETTINGS_FILE_PATH, run_mode)).required(false),
            )
            .build()?;

        // Deserialize and return the configuration
        config.try_deserialize()
    }
}
/// Custom Debug implementation for Provider
impl fmt::Debug for Provider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Provider")
            .field("name", &self.name)
            .field("service", &self.service)
            .field("model", &self.model)
            .field("api_url", &self.api_url)
            .field("max_tokens", &self.max_tokens)
            .finish()
    }
}
/// Custom Display implementation for Provider
impl fmt::Display for Provider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Provider")
            .field("name", &self.name)
            .field("service", &self.service)
            .field("model", &self.model)
            .field("api_url", &self.api_url)
            .field("max_tokens", &self.max_tokens)
            .finish()
    }
}
/// Custom Debug implementation for SensitiveSettings to prevent accidental printing of secret
impl fmt::Debug for SensitiveSettings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "*** sensitive data hidden ***")
    }
}
/// Custom Display implementation for SensitiveSettings to prevent accidental printing of secret
impl fmt::Display for SensitiveSettings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "*** sensitive data hidden ***")
    }
}
/// Locking up the APIKey to prevent accidental display
/// Note: `fn new(key: String) -> Self` is used by Settings::new, however the compiler moans
impl APIKey {
    #[allow(dead_code)]
    pub fn new(key: String) -> Self {
        APIKey(key)
    }

    // Function to access the sensitive data when absolutely necessary
    pub fn get(&self) -> &String {
        &self.0
    }
}
/// Custom Debug implementation for APIKey to prevent accidental printing of secret
impl fmt::Debug for APIKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "*** sensitive data hidden ***")
    }
}
/// Custom Display implementation for APIKey to prevent accidental printing of secret
impl fmt::Display for APIKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "*** sensitive data hidden ***")
    }
}
