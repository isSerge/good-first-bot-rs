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

#[tokio::test(flavor = "multi_thread")]
async fn rate_limit_guard_sleeps_when_below_threshold_and_before_reset() {
    // -------- Arrange --------
    let threshold = 5;
    let client = DefaultGithubClient::new("fake_token", "https://example.com/graphql", threshold as u64)
        .expect("client");

    // Force a short wait window
    const WAIT_MS: u64 = 40;
    {
        let mut state = client.rate_limit.lock().await;
        state.remaining = threshold; // at or below threshold
        state.reset_at = Instant::now() + Duration::from_millis(WAIT_MS);
    }

    // Bounds: wait .. wait + wait/10  (+ a little fudge for scheduler noise)
    let expected_min = Duration::from_millis(WAIT_MS);
    let expected_max = Duration::from_millis(WAIT_MS + WAIT_MS / 10);
    let fudge = Duration::from_millis(10);

    // -------- Act --------
    let start = Instant::now();
    client.rate_limit_guard().await;
    let elapsed = start.elapsed();

    // -------- Assert --------
    assert!(
        elapsed >= expected_min,
        "Guard returned too fast: {:?} < {:?}", elapsed, expected_min
    );
    assert!(
        elapsed <= expected_max + fudge,
        "Guard slept too long: {:?} > {:?}", elapsed, expected_max + fudge
    );
}