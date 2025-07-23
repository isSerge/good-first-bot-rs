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

    const WAIT_MS: u64 = 40;
    {
        let mut state = client.rate_limit.lock().await;
        state.remaining = threshold;
        state.reset_at = Instant::now() + Duration::from_millis(WAIT_MS);
    }

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

/// Helper: set the shared rate-limit state so the guard will sleep `wait_ms` plus jitter.
async fn prime_state(client: &DefaultGithubClient, remaining: u32, wait_ms: u64) {
    let mut s = client.rate_limit.lock().await;
    s.remaining = remaining;
    s.reset_at = Instant::now() + Duration::from_millis(wait_ms);
}

/// Helper: run the guard once and return how long it actually waited.
async fn measure_sleep(client: &DefaultGithubClient) -> Duration {
    let start = Instant::now();
    client.rate_limit_guard().await;
    start.elapsed()
}

/// 1) Always inside [wait, wait + 10%] (with a little fudge for scheduler noise)
#[tokio::test(flavor = "multi_thread")]
async fn jitter_is_within_bounds() {
    const THRESHOLD: u64 = 5;
    const WAIT_MS: u64 = 50;
    const FUDGE_MS: u64 = 8;

    let client = DefaultGithubClient::new("fake", "https://example/graphql", THRESHOLD)
        .expect("client");

    // Force a sleep path
    prime_state(&client, THRESHOLD as u32, WAIT_MS).await;

    let expected_min = Duration::from_millis(WAIT_MS);
    let expected_max = Duration::from_millis(WAIT_MS + WAIT_MS / 10); // 10% jitter
    let fudge = Duration::from_millis(FUDGE_MS);

    let elapsed = measure_sleep(&client).await;

    assert!(
        elapsed >= expected_min,
        "Returned too fast: {:?} < {:?}",
        elapsed,
        expected_min
    );
    assert!(
        elapsed <= expected_max + fudge,
        "Slept too long: {:?} > {:?}",
        elapsed,
        expected_max + fudge
    );
}

/// 2) We actually *get* jitter sometimes (i.e., not always exactly WAIT_MS).
///    Run the guard a bunch of times and check the spread of durations.
#[tokio::test(flavor = "multi_thread")]
async fn jitter_varies_across_runs() {
    const THRESHOLD: u64 = 3;
    const WAIT_MS: u64 = 40;
    const RUNS: usize = 20;
    const FUDGE_MS: u64 = 8;

    let client = DefaultGithubClient::new("fake", "https://example/graphql", THRESHOLD)
        .expect("client");

    let mut samples = Vec::with_capacity(RUNS);

    for _ in 0..RUNS {
        prime_state(&client, THRESHOLD as u32, WAIT_MS).await;
        samples.push(measure_sleep(&client).await);
    }

    let min = samples.iter().min().cloned().unwrap();
    let max = samples.iter().max().cloned().unwrap();

    let base = Duration::from_millis(WAIT_MS);
    let jitter_span = Duration::from_millis(WAIT_MS / 10);

    // Same bounds check as safety
    for (i, dur) in samples.iter().enumerate() {
        assert!(
            *dur >= base,
            "Run {i}: {:?} < base {:?}",
            dur,
            base
        );
        assert!(
            *dur <= base + jitter_span + Duration::from_millis(FUDGE_MS),
            "Run {i}: {:?} > upper bound {:?}",
            dur,
            base + jitter_span + Duration::from_millis(FUDGE_MS)
        );
    }

    // And now confirm we saw at least ~some spread (non‑deterministic, so we only require >1ms spread).
    assert!(
        max > min + Duration::from_millis(1),
        "Jitter didn't vary enough: min={:?}, max={:?}",
        min,
        max
    );
}

/// 3) When wait == 0, max_jitter == 0 → no sleep at all.
#[tokio::test(flavor = "multi_thread")]
async fn no_jitter_when_wait_is_zero() {
    const THRESHOLD: u64 = 1;
    let client = DefaultGithubClient::new("fake", "https://example/graphql", THRESHOLD)
        .expect("client");

    // Force path where remaining <= threshold but reset_at == now
    let mut s = client.rate_limit.lock().await;
    s.remaining = THRESHOLD as u32;
    s.reset_at = Instant::now(); // so wait = 0
    drop(s);

    // Should return basically immediately
    let elapsed = measure_sleep(&client).await;
    assert!(
        elapsed < Duration::from_millis(2),
        "Guard unexpectedly slept: {:?}",
        elapsed
    );
}