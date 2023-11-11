// src/config/mod.rs
use std::env;

pub struct Config {
    pub openai_org_id: String,
    pub openai_org_name: String,
    pub openai_api_key: String,
    pub openai_model: String,
    // TODO Hold these in the .env file for now. Later these will be entered via the UI
    pub folder_path: String,
    pub review_type: String,
}

impl Config {
    pub fn load() -> Self {
        dotenv::dotenv().ok();

        let openai_org_id =
            env::var("OPENAI_ORGANIZATION_ID").expect("OPENAI_ORGANIZATION_ID must be set");
        let openai_org_name =
            env::var("OPENAI_ORGANIZATION_NAME").expect("OPENAI_ORGANIZATION_NAME must be set");
        let openai_api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set");
        let openai_model = env::var("OPENAI_MODEL").expect("OPENAI_MODEL must be set");

        // TODO These are from the .env file for now. Later these will be entered via the UI
        let folder_path = env::var("FOLDER_PATH").expect("FOLDER_PATH must be set");
        let review_type = env::var("REVIEW_TYPE").expect("REVIEW_TYPE must be set");

        Config {
            openai_org_id,
            openai_org_name,
            openai_api_key,
            openai_model,
            folder_path,
            review_type,
        }
    }
}
