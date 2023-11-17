//!
//!
pub mod chat_prompts; // captures prompts for the configured provider services
pub mod data; // data structures for serialisation, etc.
pub mod provider; // handles the various generative AI agents used for file review
pub mod review; // handles the repository contents and review inputs and outputs for a provider AI agent
pub mod settings; // handles configuration of application
