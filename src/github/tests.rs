use std::collections::HashMap;

use super::*;

#[test]
fn test_new_github_client() {
    let client = DefaultGithubClient::new("test_token", "https://api.github.com/graphql");
    assert!(client.is_ok());
}

#[test]
fn test_is_retryable_graphql_error() {
    let mut extensions = HashMap::new();
    extensions.insert("code".to_string(), serde_json::Value::String("RATE_LIMITED".to_string()));

    let error = graphql_client::Error {
        message: "Rate limited".to_string(),
        locations: None,
        path: None,
        extensions: Some(extensions),
    };

    assert!(is_retryable_graphql_error(&error));
}

#[test]
fn test_is_not_retryable_graphql_error() {
    let mut extensions = HashMap::new();
    extensions.insert("code".to_string(), serde_json::Value::String("OTHER_ERROR".to_string()));

    let error = graphql_client::Error {
        message: "Some other error".to_string(),
        locations: None,
        path: None,
        extensions: Some(extensions),
    };

    assert!(!is_retryable_graphql_error(&error));
}
