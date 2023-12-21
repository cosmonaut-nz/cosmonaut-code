//! Settings for setting up:
//!     Service providers {OpenAI, Google, Anthropic, Meta, Other...}
//!     LLM API access
//!     Repository directory/folder location
// TODO refactor so that the settings are self-contained and are safe once loaded via the 'new' function
use config::FileFormat;
use config::{Config, ConfigError, File};
use serde::{Deserialize, Serialize};
use std::env;
use std::fmt;

use crate::review::report::OutputType;

const DEFAULT_CONFIG: &str = include_str!("../../settings/default.json");
pub(crate) const ENV_SENSITIVE_SETTINGS_PATH: &str = "SENSITIVE_SETTINGS_PATH";

#[derive(Serialize, Deserialize, PartialEq)]
pub(crate) struct Settings {
    pub(crate) providers: Vec<ProviderSettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) chosen_provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) chosen_service: Option<String>,
    pub(crate) default_provider: String,
    #[serde(default)]
    pub(crate) output_type: OutputType,
    #[serde(default)]
    pub(crate) review_type: ReviewType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) review_cycles: Option<i32>,
    pub(crate) repository_path: String,
    pub(crate) report_output_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) developer_mode: Option<DeveloperMode>,
    pub(crate) sensitive: SensitiveSettings,
}
/// Custom Debug implementation for Settings
impl fmt::Debug for Settings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Settings")
            .field("providers", &self.providers)
            .field("chosen_provider", &self.chosen_provider)
            .field("chosen_service", &self.chosen_service)
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
            .field("chosen_service", &self.chosen_service)
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
///     2. From an evironment variable, "SENSITIVE_SETTINGS_PATH" that points to a `json` file, and not present
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
        let config_builder: config::ConfigBuilder<config::builder::DefaultState> =
            Config::builder().add_source(File::from_str(DEFAULT_CONFIG, FileFormat::Json));

        let local_settings_path: Option<String> = env::var(ENV_SENSITIVE_SETTINGS_PATH).ok();
        let config_builder: config::ConfigBuilder<config::builder::DefaultState> = if let Some(
            path,
        ) =
            local_settings_path
        {
            config_builder.add_source(
                File::with_name(&path)
                    .required(false)
                    .format(FileFormat::Json),
            )
        } else {
            return Err(ConfigError::Message("No settings.json file found. Please set the environment variable 'SENSITIVE_SETTINGS_PATH' to point to a valid settings file.".to_string()));
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
    #[cfg(debug_assertions)]
    pub(crate) fn is_developer_mode(&self) -> bool {
        self.developer_mode.is_some()
    }
}
///
#[derive(Serialize, Deserialize, PartialEq)]
pub(crate) struct ProviderSettings {
    pub(crate) name: String,
    pub(crate) services: Vec<ServiceSettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) chosen_service: Option<String>,
    pub(crate) default_service: String,
    pub(crate) api_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) api_timeout: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) max_tokens: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) max_retries: Option<i64>,
}
impl ProviderSettings {
    pub(crate) fn get_active_service(&self) -> Result<&ServiceSettings, ServiceError> {
        self.get_service_by_name(
            self.chosen_service
                .as_deref()
                .unwrap_or(&self.default_service),
        )
        .ok_or_else(|| {
            ServiceError::NotFound(
                self.chosen_service
                    .clone()
                    .unwrap_or(self.default_service.clone()),
            )
        })
    }
    /// Gets a service by name
    pub(crate) fn get_service_by_name(&self, name: &str) -> Option<&ServiceSettings> {
        self.services.iter().find(|s| s.name == name)
    }
}
/// Custom Debug implementation for ProviderSettings
impl fmt::Debug for ProviderSettings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProviderSettings")
            .field("name", &self.name)
            .field("services", &self.services)
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
            .field("services", &self.services)
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
impl std::error::Error for ProviderError {}
/// Custom error for misconfiguration of provider
impl fmt::Display for ProviderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProviderError::NotFound(name) => write!(f, "ProviderSettings not found: {}", name),
        }
    }
}
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub(crate) struct ServiceSettings {
    pub(crate) name: String,
    pub(crate) model: String,
}
pub(crate) enum ServiceError {
    NotFound(String),
}
impl fmt::Display for ServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServiceError::NotFound(name) => write!(f, "ServiceSettings not found: {}", name),
        }
    }
}
impl std::fmt::Debug for ServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(arg0) => f.debug_tuple("NotFound").field(arg0).finish(),
        }
    }
}
impl std::error::Error for ServiceError {}
impl fmt::Debug for SensitiveSettings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "*** sensitive data hidden ***")
    }
}
#[derive(Debug, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ReviewType {
    #[default]
    General,
    Security,
    CodeStats,
}

/// An [`Option`]al set of settings to control the output of the programme for development purposes
/// #Fields
///
/// - 'max_file_count': To improve development feedback loop time on big repos, allows sampling.
/// - 'verbose_data_output': a flag to produce a full 'json' file, even if the [`OutputType`] is 'html' or other
/// - 'developer_path': Provides a developer path through the code.
/// - 'test_json_path': the path to a previous [`crate::review::data::RepositoryReview`] serialized to a file.
#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub(crate) struct DeveloperMode {
    pub(crate) max_file_count: Option<i32>,
    #[serde(default = "default_false")]
    pub(crate) verbose_data_output: bool,
    #[serde(default = "default_false")]
    pub(crate) test_path: bool,
    pub(crate) test_file: Option<String>,
}
#[derive(Serialize, Deserialize, PartialEq)]
pub(crate) struct SensitiveSettings {
    pub(crate) api_key: APIKey,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) org_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) org_name: Option<String>,
}
/// Custom Display implementation for SensitiveSettings to prevent accidental printing of secret
impl fmt::Display for SensitiveSettings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "*** sensitive data hidden ***")
    }
}
#[derive(Serialize, Deserialize, PartialEq)]
pub(crate) struct APIKey(String); // Sensitive data!
/// Locking up the APIKey to prevent accidental display
impl APIKey {
    pub(crate) fn use_key<T, F>(&self, f: F) -> T
    where
        F: FnOnce(&str) -> T,
    {
        f(&self.0)
    }
}
/// Helper to enable a default 'false' value for a boolean field
fn default_false() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_provider_settings_serialization() {
        let services = vec![ServiceSettings {
            name: "GPT-3".to_string(),
            model: "text-davinci-003".to_string(),
        }];
        let provider = ProviderSettings {
            name: "openai".to_string(),
            services,
            chosen_service: None,
            default_service: "gpt-3.5".to_string(),
            api_url: "https://api.openai.com".to_string(),
            api_timeout: Some(60),
            max_tokens: Some(2048),
            max_retries: Some(5),
        };

        let serialized = serde_json::to_string(&provider).unwrap();
        assert!(serialized.contains("openai"));
    }

    #[test]
    fn test_provider_settings_deserialization() {
        let json = r#"{
            "name": "openai",
            "services": [
                {
                    "name": "gpt-4",
                    "model": "gpt-4-1106-preview"
                },
                {
                    "name": "gpt-3.5",
                    "model": "gpt-3.5-turbo-1106"
                }
            ],
            "default_service": "gpt-3.5",
            "api_url": "https://api.openai.com/v1/chat/completions",
            "max_retries": 3
        }"#;

        let provider: ProviderSettings = serde_json::from_str(json).unwrap();
        assert_eq!(provider.name, "openai");
    }

    #[test]
    fn test_api_key_use() {
        let api_key = APIKey("secret".to_string());
        let result = api_key.use_key(|key| key.to_uppercase());

        assert_eq!(result, "SECRET");
    }

    #[test]
    fn test_settings_new_with_env() {
        // Create a temporary directory
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("settings.json");

        // Write settings data to a temporary file
        let mut file = File::create(&file_path).unwrap();
        writeln!(
            file,
            r#"{{
                "default_provider": "TestProvider",
                "output_type": "json",
                "review_type": "general",
                "repository_path": "test/repo/path",
                "report_output_path": "test/report/path",
                "sensitive": {{
                    "api_key": "testkey"
                }}
            }}"#
        )
        .unwrap();

        std::env::set_var(ENV_SENSITIVE_SETTINGS_PATH, file_path.to_str().unwrap());

        let settings = Settings::new().unwrap();

        assert_eq!(settings.default_provider, "TestProvider");
        assert_eq!(settings.output_type, OutputType::Json);
        assert_eq!(settings.review_type, ReviewType::General);
        assert_eq!(settings.repository_path, "test/repo/path");
        assert_eq!(settings.report_output_path, "test/report/path");
        assert_eq!(settings.sensitive.api_key.0, "testkey");

        dir.close().unwrap();

        std::env::remove_var(ENV_SENSITIVE_SETTINGS_PATH);
    }

    #[test]
    fn test_loading_developer_mode_from_json() {
        let json_data = r#"
        {
            "providers": [],
            "chosen_provider": null,
            "default_provider": "SomeProvider",
            "default_service": "SomeService",
            "output_type": "pdf",
            "review_type": "general",
            "repository_path": "some/path",
            "report_output_path": "some/output/path",
            "sensitive": { 
                "api_key": "some_key_value",
                "org_id": "some_org_value",
                "org_name": "some_org_name_value" 
            },
            "developer_mode": {
                "max_file_count": 10,
                "verbose_data_output": true
            }
        }
        "#;

        let settings: Settings =
            serde_json::from_str(json_data).expect("JSON was not well-formatted");
        assert!(settings.developer_mode.is_some());
        let dev_settings = settings.developer_mode.unwrap();
        assert_eq!(dev_settings.max_file_count, Some(10));
        assert!(dev_settings.verbose_data_output);
    }

    #[test]
    fn test_get_active_provider() {
        let services = vec![ServiceSettings {
            name: "GPT-3".to_string(),
            model: "gpt-3.5".to_string(),
        }];
        let settings = Settings {
            providers: vec![ProviderSettings {
                name: "openai".to_string(),
                services,
                chosen_service: None,
                default_service: "gpt-3.5".to_string(),
                api_url: "https://api.openai.com".to_string(),
                api_timeout: Some(60),
                max_tokens: Some(2048),
                max_retries: Some(5),
            }],
            chosen_provider: None,
            chosen_service: None,
            default_provider: "openai".to_string(),
            output_type: OutputType::Json,
            review_type: ReviewType::General,
            review_cycles: None,
            repository_path: "path/to/repo".to_string(),
            report_output_path: "path/to/report".to_string(),
            sensitive: SensitiveSettings {
                api_key: APIKey("secret".to_string()),
                org_id: None,
                org_name: None,
            },
            developer_mode: None,
        };
        let provider = settings.get_active_provider().unwrap();
        assert_eq!(provider.name, "openai");
    }
}
