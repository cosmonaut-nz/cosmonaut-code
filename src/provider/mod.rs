//!
//!  Handles the access to the LLM with utility functions for specified actions
//!
pub(crate) mod api;
pub(crate) mod google;
pub(crate) mod lmstudio;
pub(crate) mod openai;
pub(crate) mod prompts;
use crate::provider::prompts::PromptData;
use crate::settings::{ProviderSettings, ServiceSettings, Settings};

use self::api::ProviderCompletionResponse;

/// Sends text contents to an LLM agent to evaluate according to the prompt passed to it.
///
/// # Parameters
///
/// * `settings` - A ['Config'] that contains information for the LLM
/// * `provider` - the configured provider that will handle the review request - e.g. 'openai', 'meta', etc.
/// * `prompt_data` - a preconfigured prompt text to ask the LLM to do a task, including the body of the output_file to be reviewed
///
/// # Returns
///
/// * A response from the LLM ['ProviderResponseMessage']
///
pub(crate) async fn review_or_summarise(
    request_type: RequestType,
    settings: &Settings,
    provider_settings: &ProviderSettings,
    prompt_data: &PromptData,
) -> Result<ProviderCompletionResponse, Box<dyn std::error::Error>> {
    match create_api_provider(provider_settings) {
        Ok(provider_handler) => {
            provider_handler
                .ask_request_of_provider(&request_type, settings, prompt_data)
                .await
        }
        Err(err) => Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("API provider error: {}", err),
        ))),
    }
}
/// Creates an APIProvider according to provider_settings.name
fn create_api_provider(
    provider_settings: &ProviderSettings,
) -> Result<Box<dyn APIProvider>, Box<dyn std::error::Error>> {
    match provider_settings.name.to_lowercase().as_str() {
        "openai" => Ok(Box::new(openai::OpenAIProvider {
            model: provider_settings.get_active_service()?.model.to_string(),
        })),
        "google" => Ok(Box::new(google::GoogleProvider {
            model: provider_settings.get_active_service()?.model.to_string(),
        })),
        "local" => Ok(Box::new(lmstudio::LMStudioProvider {})),
        _ => Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Unsupported provider: {}", provider_settings.name),
        ))),
    }
}
/// An APIProvider trait allowing for multiple API providers to be implemented
/// A gamble on the future (geddit) of Rust here.
#[async_trait::async_trait]
trait APIProvider {
    async fn ask_request_of_provider(
        &self,
        request_type: &RequestType,
        settings: &Settings,
        prompt_data: &PromptData,
    ) -> Result<ProviderCompletionResponse, Box<dyn std::error::Error>>;
}

pub(crate) enum RequestType {
    Review,
    Summarise,
}

/// Returns the provider and model from the settings file
pub(crate) fn get_service_and_model(settings: &Settings) -> Option<String> {
    let provider: &ProviderSettings = get_provider(settings);
    let service: &ServiceSettings = if let Some(chosen_service) = &settings.chosen_service {
        provider.get_service_by_name(chosen_service)?
    } else {
        get_service(provider)
    };
    Some(format!(
        "provider: {}, service: {}, model: {}",
        provider.name, service.name, service.model
    ))
}
/// Gets the currently active provider. If there is a misconfiguration (i.e., a mangled `default.json`) then panics
pub(crate) fn get_provider(settings: &Settings) -> &crate::settings::ProviderSettings {
    let provider: &crate::settings::ProviderSettings = settings.get_active_provider()
                                              .expect("Either a default or chosen provider should be configured in \'default.json\'. \
                                              Either none was found, or the default provider did not match any name in the configured providers list.");
    provider
}
/// Gets the currently active service. If there is a misconfiguration (i.e., a mangled `default.json`) then panics
pub(crate) fn get_service(provider: &ProviderSettings) -> &ServiceSettings {
    provider
        .get_active_service()
        .expect("Either a default or chosen service should be configured in \'default.json\'. \
        Either none was found, or the default service did not match any name in the provider services list.")
}
/// HTTP error codes
#[repr(u16)]
enum HttpErrorCode {
    BadRequest = 400,
    Unauthorized = 401,
    Forbidden = 403,
    NotFound = 404,
    InternalServerError = 500,
    BadGateway = 502,
    ServiceUnavailable = 503,
    GatewayTimeout = 504,
}
/// Extracts the HTTP status code from an error message string
/// Solves where the API wrapper embeds the actual HTTP status code in the error message
fn extract_http_status(error_message: &str) -> Option<u16> {
    if error_message.contains("400") {
        return Some(HttpErrorCode::BadRequest as u16);
    }
    if error_message.contains("401") {
        return Some(HttpErrorCode::Unauthorized as u16);
    }
    if error_message.contains("403") {
        return Some(HttpErrorCode::Forbidden as u16);
    }
    if error_message.contains("404") {
        return Some(HttpErrorCode::NotFound as u16);
    }
    if error_message.contains("500") {
        return Some(HttpErrorCode::InternalServerError as u16);
    }
    if error_message.contains("502") {
        return Some(HttpErrorCode::BadGateway as u16);
    }
    if error_message.contains("503") {
        return Some(HttpErrorCode::ServiceUnavailable as u16);
    }
    if error_message.contains("504") {
        return Some(HttpErrorCode::GatewayTimeout as u16);
    }
    None
}
