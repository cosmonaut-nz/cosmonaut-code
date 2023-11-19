//! The codebase is composed of three separate modules.
//!
/// Handles the various generative AI agents used for file review
/// Includes capturing specific prompts for the configured provider services
pub mod provider;
/// Handles the overall review of a repository, including extracting repository contents
/// and review inputs and outputs for a 'provider' AI agent.
/// Includes data structures for serialisation, etc.
pub mod review;
/// Handles the configuration of the application via data structures
/// Includes the secure handling of sensitive data items such as API keys
pub mod settings;
