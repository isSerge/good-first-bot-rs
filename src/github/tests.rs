use super::*;
use std::collections::HashMap;
#[test]
fn test_new_github_client() {
    let client = DefaultGithubClient::new("test_token", "https://api.github.com/graphql", 10);
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

#[tokio::test]
async fn test_update_rate_limit_from_headers() {
    let client =
        DefaultGithubClient::new("fake", "https://api.github.com/graphql", 5).expect("client init");

    // Build fake headers with remaining=3, reset in 60s
    let mut headers = HeaderMap::new();
    headers.insert("X-RateLimit-Remaining", HeaderValue::from_static("3"));
    let reset_ts = (chrono::Utc::now().timestamp() as u64) + 60;
    headers.insert("X-RateLimit-Reset", HeaderValue::from_str(&reset_ts.to_string()).unwrap());

    client.update_rate_limit_from_headers(&headers).await;

    let state = client.rate_limit.lock().await;
    assert_eq!(state.remaining, 3);
    // We expect reset_at ≈ now + 60s (within a small delta)
    let diff = state.reset_at.checked_duration_since(Instant::now()).unwrap();
    assert!(diff >= Duration::from_secs(59) && diff <= Duration::from_secs(61));
}
