//! Static const for configuration
//!
use std::time::Duration;

/// Used to set the 'reqwest::Client' timeout to 30 seconds
pub const API_TIMEOUT: Duration = Duration::from_secs(30);

pub mod openai {
    pub const PROVIDER_NAME: &str = "openai";
}
