//!
//! Settings for setting up:
//!     Service providers {OpenAI, Google, Anthropic, Meta, Other...}
//!     LLM API access
//!     Repository directory/folder location
//!
//!
use config::{Config, ConfigError, File};
use log::warn;
use serde::{Deserialize, Serialize};
use std::env;
use std::fmt;
use std::time::SystemTime;

const SETTINGS_FILE_PATH: &str = "settings";
const APP_ENV_PREFIX: &str = "COSMOCODE";

/// struct to hold the configuration
///
#[derive(Serialize, Deserialize, PartialEq)]
pub struct Settings {
    // General configuration fields
    pub providers: Vec<ProviderSettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chosen_provider: Option<String>,
    pub default_provider: String,
    pub output_type: String,
    pub review_type: i32,
    pub repository_type: String,
    pub repository_path: String,
    pub report_output_path: String,

    // Sensitive data
    pub sensitive: SensitiveSettings,
}

#[derive(Serialize, Deserialize, PartialEq)]
pub struct ProviderSettings {
    pub name: String,
    pub service: String,
    pub model: String,
    pub api_url: String,
    pub api_timeout: u64,
    pub max_tokens: i64,
}
#[derive(Serialize, Deserialize, PartialEq)]
pub struct SensitiveSettings {
    pub api_key: APIKey,
    pub org_id: String,
    pub org_name: String,
}
#[derive(Serialize, Deserialize, PartialEq)]
pub struct APIKey(String); // Sensitive data

/// Custom Debug implementation for Settings
impl fmt::Debug for Settings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Settings")
            .field("providers", &self.providers)
            .field("chosen_provider", &self.chosen_provider)
            .field("default_provider", &self.default_provider)
            .field("output_type", &self.output_type)
            .field("review_type", &self.review_type)
            .field("repository_type", &self.review_type)
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
            .field("providers", &self.providers)
            .field("chosen_provider", &self.chosen_provider)
            .field("default_provider", &self.default_provider)
            .field("output_type", &self.output_type)
            .field("review_type", &self.review_type)
            .field("repository_type", &self.review_type)
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
/// - `providers`: The set of organizations providing the language model service (e.g., openai, google, anthropic, meta, etc.).
/// - `default_provider`: Default is openai.
/// - `chosen_provider`: The user selected provider from the configured list.
/// - `sensitive settings`: Inc. API key for authentication, org_id and org_name.
/// - `repository_type`: The type of repository, e.g., 'java-server', or 'javascript-web', etc.
/// - `repository_path`: The user selected path to the folder containing repository and code for analysis.
/// - `report_output_path`: The user selected path where analysis output report will be stored.
/// - `output_type`: The user selected format/type of the output (e.g., json, pdf). Default is JSON.
/// - `review_type`: The user selected numeric code indicating the type of review (e.g., 1 for general, 2 for security; default is 1).
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

    /// Function gets either the chosen provider or default provider, or gives a ProviderError
    pub fn get_active_provider(&self) -> Result<&ProviderSettings, ProviderError> {
        let provider_name = self
            .chosen_provider
            .as_ref()
            .unwrap_or(&self.default_provider);
        self.providers
            .iter()
            .find(|p| p.name == *provider_name)
            .ok_or_else(|| ProviderError::NotFound(provider_name.clone()))
    }
}
/// Custom Debug implementation for ProviderSettings
impl fmt::Debug for ProviderSettings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProviderSettings")
            .field("name", &self.name)
            .field("service", &self.service)
            .field("model", &self.model)
            .field("api_url", &self.api_url)
            .field("api_timeout", &self.api_timeout)
            .field("max_tokens", &self.max_tokens)
            .finish()
    }
}
/// Custom Display implementation for ProviderSettings
impl fmt::Display for ProviderSettings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProviderSettings")
            .field("name", &self.name)
            .field("service", &self.service)
            .field("model", &self.model)
            .field("api_url", &self.api_url)
            .field("api_timeout", &self.api_timeout)
            .field("max_tokens", &self.max_tokens)
            .finish()
    }
}
#[derive(Debug)]
pub enum ProviderError {
    NotFound(String),
}
/// Custom error for misconfiguration of provider
impl fmt::Display for ProviderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProviderError::NotFound(name) => write!(f, "ProviderSettings not found: {}", name),
        }
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
/// Note: `fn new(key: String) -> Self` is used by Settings::new, however the compiler moans, so added dead_code allowance
impl APIKey {
    #[allow(dead_code)]
    pub fn new(key: String) -> Self {
        APIKey(key)
    }

    // Function to access the sensitive data when absolutely necessary
    pub fn get(&self, access_context: &str) -> &str {
        let timestamp = SystemTime::now();
        warn!(
            "APIKey accessed at {:?} in context '{}'.",
            timestamp, access_context
        );
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
