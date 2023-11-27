//!
//! Settings for setting up:
//!     Service providers {OpenAI, Google, Anthropic, Meta, Other...}
//!     LLM API access
//!     Repository directory/folder location
//!
//!
use config::{Config, ConfigError, File};
use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::env;
use std::fmt;
use std::time::SystemTime;

const SETTINGS_FILE_PATH: &str = "settings"; // TODO: determine the right path when packaged

/// struct to hold the configuration
///
#[derive(Serialize, Deserialize, PartialEq)]
pub(crate) struct Settings {
    // General configuration fields
    pub(crate) providers: Vec<ProviderSettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) chosen_provider: Option<String>,
    pub(crate) default_provider: String,
    pub(crate) output_type: String,
    pub(crate) review_type: i32,
    pub(crate) repository_path: String,
    pub(crate) report_output_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) max_file_count: Option<i32>,

    // Sensitive data
    pub(crate) sensitive: SensitiveSettings,
}

#[derive(Serialize, Deserialize, PartialEq)]
pub(crate) struct ProviderSettings {
    pub(crate) name: String,
    pub(crate) service: String,
    pub(crate) model: String,
    pub(crate) api_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) api_timeout: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) max_tokens: Option<i64>,
}
#[derive(Serialize, Deserialize, PartialEq)]
pub(crate) struct SensitiveSettings {
    pub(crate) api_key: APIKey,
    pub(crate) org_id: String,
    pub(crate) org_name: String,
}
#[derive(Serialize, Deserialize, PartialEq)]
pub(crate) struct APIKey(String); // Sensitive data

/// Custom Debug implementation for Settings
impl fmt::Debug for Settings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Settings")
            .field("providers", &self.providers)
            .field("chosen_provider", &self.chosen_provider)
            .field("default_provider", &self.default_provider)
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
            .field("providers", &self.providers)
            .field("chosen_provider", &self.chosen_provider)
            .field("default_provider", &self.default_provider)
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
/// - `providers`: The set of organizations providing the language model service (e.g., openai, google, anthropic, meta, etc.).
/// - `default_provider`: Default is openai.
/// - `chosen_provider`: The user selected provider from the configured list.
/// - `sensitive settings`: Inc. API key for authentication, org_id and org_name.
/// - `repository_path`: The user selected path to the folder containing repository and code for analysis.
/// - `report_output_path`: The user selected path where analysis output report will be stored.
/// - `output_type`: The user selected format/type of the output (e.g., json, pdf). Default is JSON.
/// - `review_type`: The user selected numeric code indicating the type of review (e.g., 1 for general, 2 for security; default is 1).
///
/// `review_type` and `output_type` have default values, but other fields must be explicitly set.
impl Settings {
    pub(crate) fn new() -> Result<Self, ConfigError> {
        // Determine run mode based on build profile
        let default_run_mode = if cfg!(debug_assertions) {
            // Default to 'development' if in debug mode
            warn!("RUN_MODE not set, defaulting to 'development' (debug build)");
            "development"
        } else {
            // default is production
            info!("RUN_MODE not set, defaulting to 'production' (release build)");
            "production"
        };
        let run_mode = env::var("RUN_MODE").unwrap_or_else(|_| default_run_mode.into());

        //Load the config,
        let config = Config::builder()
            // Start off by merging in the "default" configuration file
            .add_source(File::with_name(&format!("{}/default", SETTINGS_FILE_PATH)).required(false))
            // Default to (optional) 'development' env
            .add_source(
                File::with_name(&format!("{}/{}", SETTINGS_FILE_PATH, run_mode)).required(false),
            )
            .build()?;

        // Deserialize and return the configuration
        config.try_deserialize()
    }

    /// Function gets either the chosen provider or default provider, or gives a ProviderError
    pub(crate) fn get_active_provider(&self) -> Result<&ProviderSettings, ProviderError> {
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
pub(crate) enum ProviderError {
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
///
impl APIKey {
    pub(crate) fn use_key<T, F>(&self, access_context: &str, f: F) -> T
    where
        F: FnOnce(&str) -> T,
    {
        let timestamp = SystemTime::now();
        warn!(
            "APIKey accessed at {:?} in context '{}'.",
            timestamp, access_context
        );

        f(&self.0)
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
