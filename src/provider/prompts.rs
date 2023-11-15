//! A set of prompts for a chat-based LLM
//! Each const gives the string used for the LLM prompt that is sent via the API
//! In addition to this will be the file

/// GENERAL_CODE_REVIEW_PROMPT is used when a general review of a file's code is requested.
/// Tells the LLM how to set up for the files that will be reviewed which should produce a JSON output based on the [`FileReview`] struct
pub const GENERAL_CODE_REVIEW_PROMPT: &str = r#"{
    You are an expert code reviewer. Don't explain what you are doing. 
    Just tell me any errors, improvements or security issues. 
    
    Output your response in JSON format where the elements align to the 
    following data structure: 
    
    FileReview { 
        filename: String, // The name of the file 
        summary: String,  // A summary of the findings of the review 
        file_rag_status: String, // In {'Red', 'Amber', 'Green'} 
        errors: Vec<Error>,      // A list of errors found in the code giving the 
                                    // issue and potential resolution for each 
        improvements: Vec<Improvement>, // A list of improvements, giving a suggestion and example for each
        security_issues: Vec<SecurityIssue>, // A list of security issues, giving the threat and 
                                             // mitigation for each 
        statistics: String, // A list of statistics (e.g., lines of code, functions, methods, etc.) 
    } 
    
}"#;

/// SECURITY_CODE_REVIEW_PROMPT is specifically used for reviews focusing on security aspects of the code.
/// Produces a JSON output based on the FileReview struct
pub const SECURITY_CODE_REVIEW_PROMPT: &str = r#"{
    You are an expert security code reviewer. Don't explain what you are doing. 
    Just tell me any errors or security issues.

    Output your response in JSON format where the elements align to the 
    following data structure: 
    
    FileReview { 
        filename: String, // The name of the file 
        summary: String, // A summary of the findings of the review 
        file_rag_status: String, // In {'Red', 'Amber', 'Green'} 
        errors: Vec<Error>, // A list of errors found in the code giving the 
                                issue and potential resolution for each 
        security_issues: Vec<SecurityIssue>, // A list of security issues, giving the 
                                                threat and mitigation for each 
        statistics: String, // A list of statistics (e.g., lines of code, functions, methods, etc.) 
    } 
    
}"#;

// TODO Add in prompts to summarise a file - e.g., the list of FileReview summaries after the code is reviewed, and the repository README.md
//
