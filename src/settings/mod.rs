//!
//! Settings for setting up:
//!     Service providers {OpenAI, Google, Anthropic, Meta, Other...}
//!     LLM API access
//!     Repository directory/folder location
//!
//!
use config::FileFormat;
use config::{Config, ConfigError, File};
use inquire::formatter::StringFormatter;
use inquire::Text;
use log::warn;
use serde::{Deserialize, Serialize};
use std::env;
use std::fmt;
use std::time::SystemTime;

const DEFAULT_CONFIG: &str = include_str!("../../settings/default.json");
pub(crate) const ENV_SENSITIVE_SETTINGS_PATH: &str = "SENSITIVE_SETTINGS_PATH";

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) org_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) org_name: Option<String>,
}
#[derive(Serialize, Deserialize, PartialEq)]
pub(crate) struct APIKey(String); // Sensitive data!

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
/// The Settings are loaded via a [`Config`]::builder() from iterative sources:
///     1. From the 'DEFAULT_CONFIG' that is loaded at compile time
///     2. From an evironment variable, "SENSITIVE_SETTINGS_PATH" that points to a `json` file, and not present, then
///     3. From user commandline input for the required configuration and sensitive data such as 'api_key'
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
        let local_settings_path: Option<String> = env::var(ENV_SENSITIVE_SETTINGS_PATH).ok();
        let config_builder: config::ConfigBuilder<config::builder::DefaultState> =
            Config::builder().add_source(File::from_str(DEFAULT_CONFIG, FileFormat::Json));

        let config_builder: config::ConfigBuilder<config::builder::DefaultState> =
                // try to get sensitive configuration data can be sourced as an env variable
            if let Some(path) = local_settings_path {
                config_builder.add_source(
                    File::with_name(&path)
                        .required(false)
                        .format(FileFormat::Json),
                )
            } else { // TODO: when wired into UI this will need to be handled via UI, can likely remove
                let formatter: StringFormatter = &|s| {
                    let mut c = s.chars();
                    match c.next() {
                        None => String::from("No key given"),
                        Some(_f) => {
                            String::from("")
                            + "*".repeat(s.len() - 1).as_str()
                        }
                    }
                };
                // Prompt user for settings via commandline
                let repository_path = Text::new("Enter the path to a valid git repository").prompt();
                let report_path = Text::new("Enter the path to where you'd like the report").prompt();
                // TODO Drop down for configured providers
                let provider_name = Text::new("Enter the provider you'd like to use").prompt();
                let api_key = Text::new("Enter your provider API key").with_formatter(formatter).prompt();

                // Build a config object with user-provided settings
                config_builder
                    .set_default(
                        "repository_path",
                        repository_path.unwrap(),
                    )?
                    .set_default("report_output_path", report_path.unwrap())?
                    .set_default("chosen_provider", provider_name.unwrap())?
                    .set_default("sensitive.api_key", api_key.unwrap())?
            };
        let config = config_builder.build()?;

        config.try_deserialize::<Settings>()
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
