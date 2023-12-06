//!
//!  
//! builds the pre-requisite definitions for the usage of the GitHub Linguist data
//!
use linguist_build::{
    Config, Definition, Kind, Location, GITHUB_LINGUIST_DOCUMENTATION_URL,
    GITHUB_LINGUIST_HEURISTICS_URL, GITHUB_LINGUIST_LANGUAGES_URL, GITHUB_LINGUIST_VENDORS_URL,
};

/// Build definitions for the generated files from the linguist-rs crate
fn main() {
    Config::new()
        .add_definition(Definition {
            name: "languages.rs".to_string(),
            kind: Kind::Languages,
            location: Location::URL(GITHUB_LINGUIST_LANGUAGES_URL.to_string()),
        })
        .add_definition(Definition {
            name: "vendors.rs".to_string(),
            kind: Kind::Vendors,
            location: Location::URL(GITHUB_LINGUIST_VENDORS_URL.to_string()),
        })
        .add_definition(Definition {
            name: "heuristics.rs".to_string(),
            kind: Kind::Heuristics,
            location: Location::URL(GITHUB_LINGUIST_HEURISTICS_URL.to_string()),
        })
        .add_definition(Definition {
            name: "documentation.rs".to_string(),
            kind: Kind::Documentation,
            location: Location::URL(GITHUB_LINGUIST_DOCUMENTATION_URL.to_string()),
        })
        .generate();
}
