#![allow(missing_docs)]
#[cfg(test)]
mod tests;

use std::{
    collections::HashSet,
    sync::Arc,
    time::{Duration, Instant},
};

use async_trait::async_trait;
use backoff::{Error as BackoffError, ExponentialBackoff, future::retry};
use graphql_client::{GraphQLQuery, Response};
use mockall::automock;
use rand::{Rng, rng};
use reqwest::{
    Client,
    header::{AUTHORIZATION, HeaderMap, HeaderValue, USER_AGENT},
};
use thiserror::Error;
use tokio::sync::Mutex;

#[derive(Debug)]
struct RateLimitState {
    remaining: u32,
    reset_at: Instant,
}

/// Represents errors that can occur when interacting with the GitHub API.
#[derive(Debug, Error)]
pub enum GithubError {
    /// A network or HTTP request error from the underlying `reqwest` client.
    #[error("Network or HTTP request error: {source}")]
    RequestError {
        /// The source `reqwest` error.
        #[from]
        source: reqwest::Error,
    },
    /// An error representing an invalid HTTP header value.
    #[error("Invalid HTTP header value: {0}")]
    InvalidHeader(#[from] reqwest::header::InvalidHeaderValue),
    /// An error from the GraphQL API.
    #[error("GraphQL API error: {0}")]
    GraphQLApiError(String),
    /// An error during JSON serialization or deserialization.
    #[error("Failed to (de)serialize JSON: {source}")]
    SerializationError {
        /// The source `serde_json` error.
        #[from]
        source: serde_json::Error,
    },
    /// An error indicating that the GitHub API rate limit has been exceeded.
    #[error("GitHub API rate limited")]
    RateLimited,
    /// An error indicating that the request was not authorized.
    #[error("GitHub authentication failed")]
    Unauthorized,

    /// An error indicating that a required header could not be parsed.
    #[error("Failed to parse header: {0}")]
    HeaderError(String),
}

// Helper function to check if a GraphQL error is retryable
fn is_retryable_graphql_error(error: &graphql_client::Error) -> bool {
    error
        .extensions
        .as_ref()
        .and_then(|ext| ext.get("code"))
        .and_then(|code| code.as_str())
        .map(|code| code == "RATE_LIMITED")
        .unwrap_or(false)
}

/// A trait for interacting with the GitHub API.
#[automock]
#[async_trait]
pub trait GithubClient: Send + Sync {
    /// Check if a repository exists.
    async fn repo_exists(&self, owner: &str, name: &str) -> Result<bool, GithubError>;

    /// Get issues by label.
    async fn repo_issues_by_label(
        &self,
        owner: &str,
        name: &str,
        labels: HashSet<String>,
    ) -> Result<Vec<issues::IssuesRepositoryIssuesNodes>, GithubError>;

    /// Get repo labels
    async fn repo_labels(
        &self,
        owner: &str,
        name: &str,
    ) -> Result<Vec<labels::LabelsRepositoryLabelsNodes>, GithubError>;
}

// GraphQL DateTime scalar type.
type DateTime = String;

/// GraphQL query for checking if a repository exists.
#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/github/schema.graphql",
    query_path = "src/github/github.graphql",
    response_derives = "Debug, Default, serde::Serialize, Clone",
    variables_derives = "Debug, Clone"
)]
pub struct Repository;

/// GraphQL query for fetching issues.
#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/github/schema.graphql",
    query_path = "src/github/github.graphql",
    response_derives = "Debug, Default, serde::Serialize, Clone",
    variables_derives = "Debug, Clone"
)]
pub struct Issues;

/// GraphQL query for fetching labels.
#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/github/schema.graphql",
    query_path = "src/github/github.graphql",
    response_derives = "Debug, Default, serde::Serialize, Clone",
    variables_derives = "Debug, Clone"
)]
pub struct Labels;

/// The default implementation of the `GithubClient` trait.
#[derive(Clone)]
pub struct DefaultGithubClient {
    client: Client,
    graphql_url: String,
    rate_limit: Arc<Mutex<RateLimitState>>,
    rate_limit_threshold: u64,
}

impl DefaultGithubClient {
    /// Creates a new `DefaultGithubClient`.
    pub fn new(
        github_token: &str,
        graphql_url: &str,
        rate_limit_threshold: u64,
    ) -> Result<Self, GithubError> {
        // Build the HTTP client with the GitHub token.
        let mut headers = HeaderMap::new();

        headers.insert(AUTHORIZATION, HeaderValue::from_str(&format!("Bearer {github_token}"))?);
        headers.insert(USER_AGENT, HeaderValue::from_static("github-activity-rs"));

        let client = reqwest::Client::builder().default_headers(headers).build()?;
        let initial_state = RateLimitState { remaining: u32::MAX, reset_at: Instant::now() };
        tracing::debug!("HTTP client built successfully.");

        Ok(Self {
            client,
            graphql_url: graphql_url.to_string(),
            rate_limit: Arc::new(Mutex::new(initial_state)),
            rate_limit_threshold,
        })
    }

    /// Re-usable configuration for exponential backoff.
    fn backoff_config() -> ExponentialBackoff {
        ExponentialBackoff {
            initial_interval: Duration::from_secs(1),
            max_interval: Duration::from_secs(30),
            max_elapsed_time: Some(Duration::from_secs(60)),
            multiplier: 2.0,
            ..Default::default()
        }
    }

    /// Build, send, parse, retry, and unwrap a GraphQL query of type `Q`.
    async fn execute_graphql<Q>(
        &self,
        variables: Q::Variables,
    ) -> Result<Q::ResponseData, GithubError>
    where
        Q: GraphQLQuery,
        Q::Variables: Clone,
        Q::ResponseData: serde::de::DeserializeOwned,
    {
        // closure that Backoff expects
        let operation = || async {
            // 0. Rate limit guard
            self.rate_limit_guard().await;

            // 1. Build the request
            let request_body = Q::build_query(variables.clone());

            // 2. Send HTTP
            let resp =
                self.client.post(&self.graphql_url).json(&request_body).send().await.map_err(
                    |e| {
                        tracing::warn!("Network error sending GraphQL request: {e}. Retrying...");
                        BackoffError::transient(GithubError::RequestError { source: e })
                    },
                )?;

            //3 Update rate limit state from headers
            if let Err(e) = self.update_rate_limit_from_headers(resp.headers()).await {
                // Option A: warn and continue
                tracing::warn!("Could not update rate-limit info: {}", e);
            }

            // 4. HTTP-status check
            if !resp.status().is_success() {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_else(|e| {
                    tracing::warn!(
                        "Failed to read response text for HTTP error {status}: {e}. Using empty \
                         fallback."
                    );
                    format!("Status: {status}, No response body available.")
                });
                tracing::warn!(
                    "Non-success HTTP {status}: {}. Retrying if transient...",
                    text.chars().take(200).collect::<String>()
                );

                // Map HTTP status to specific GithubError variants
                let github_err = match status {
                    reqwest::StatusCode::UNAUTHORIZED => GithubError::Unauthorized,
                    reqwest::StatusCode::FORBIDDEN => {
                        if text.to_lowercase().contains("rate limit")
                            || text.to_lowercase().contains("secondary rate")
                        {
                            GithubError::RateLimited
                        } else {
                            GithubError::GraphQLApiError(format!(
                                "HTTP Forbidden ({status}): {text}"
                            ))
                        }
                    }
                    reqwest::StatusCode::NOT_FOUND =>
                        GithubError::GraphQLApiError(format!("HTTP Not Found ({status}): {text}")),
                    _ => GithubError::GraphQLApiError(format!("HTTP Error ({status}): {text}")),
                };

                let be = match github_err {
                    GithubError::RateLimited => BackoffError::transient(github_err),
                    _ if status.is_server_error()
                        || status == reqwest::StatusCode::TOO_MANY_REQUESTS =>
                        BackoffError::transient(github_err),
                    _ => BackoffError::permanent(github_err),
                };
                return Err(be);
            }

            // 5. Parse JSON
            let body: Response<Q::ResponseData> = resp.json().await.map_err(|e| {
                tracing::warn!("Failed to parse JSON: {e}. Retrying...");
                BackoffError::transient(GithubError::GraphQLApiError(format!(
                    "JSON parse error: {e}"
                )))
            })?;

            // 6. GraphQL errors?
            if let Some(errors) = &body.errors {
                let is_rate_limit_error = errors.iter().any(|e| {
                    e.message.to_lowercase().contains("rate limit") || is_retryable_graphql_error(e)
                });

                let msg = format!("GraphQL API reported errors: {errors:?}");

                if is_rate_limit_error {
                    tracing::warn!("Retryable GraphQL API error: {msg}. Retrying...");
                    return Err(BackoffError::transient(GithubError::RateLimited));
                } else {
                    tracing::error!("Permanent GraphQL API error: {msg}");
                    return Err(BackoffError::permanent(GithubError::GraphQLApiError(msg)));
                }
            }

            // 7. Unwrap the data or permanent-fail
            body.data.ok_or_else(|| {
                tracing::error!("GraphQL response had no data field; permanent failure");
                BackoffError::permanent(GithubError::GraphQLApiError(
                    "GraphQL response had no data field and no errors reported".to_string(),
                ))
            })
        };

        // kick off the retry loop
        retry(Self::backoff_config(), operation).await
    }

    /// Rate limit guard that sleeps until the rate limit resets if we're close
    /// to the threshold.
    async fn rate_limit_guard(&self) {
        let (remaining, reset_at) = {
            let state = self.rate_limit.lock().await;
            (state.remaining, state.reset_at)
        };

        // define a safety threshold
        let threshold = self.rate_limit_threshold as u32;
        if remaining <= threshold {
            let now = Instant::now();
            if now < reset_at {
                let wait = reset_at - now;
                tracing::info!(
                    "Approaching rate limit ({} left). Sleeping {:?} until reset...",
                    remaining,
                    wait
                );

                // Sleep until the rate limit resets
                //added a jitter to avoid thundering herd problem
                let max_jitter = wait.as_millis() as u64 / 10;
                let jitter_ms = rng().random_range(0..=max_jitter);
                tokio::time::sleep(wait + Duration::from_millis(jitter_ms)).await;
            }
        }
    }

    /// Update the rate limit state from the response headers.
    async fn update_rate_limit_from_headers(&self, headers: &HeaderMap) -> Result<(), GithubError> {
        // Names are case-insensitive in HeaderMap
        let rem_val = headers.get("X-RateLimit-Remaining").ok_or_else(|| {
            let msg = "Missing X-RateLimit-Remaining header".to_string();
            tracing::error!("{}", msg);
            GithubError::HeaderError(msg)
        })?;
        let rem_str = rem_val.to_str().map_err(|e| {
            let msg = format!("Invalid X-RateLimit-Remaining value: {e}");
            tracing::error!("{}", msg);
            GithubError::HeaderError(msg)
        })?;
        let remaining = rem_str.parse::<u32>().map_err(|e| {
            let msg = format!("Cannot parse remaining as u32: {e}");
            tracing::error!("{}", msg);
            GithubError::HeaderError(msg)
        })?;

        let reset_val = headers.get("X-RateLimit-Reset").ok_or_else(|| {
            let msg = "Missing X-RateLimit-Reset header".to_string();
            tracing::error!("{}", msg);
            GithubError::HeaderError(msg)
        })?;
        let reset_str = reset_val.to_str().map_err(|e| {
            let msg = format!("Invalid X-RateLimit-Reset value: {e}");
            tracing::error!("{}", msg);
            GithubError::HeaderError(msg)
        })?;
        let reset_unix = reset_str.parse::<u64>().map_err(|e| {
            let msg = format!("Cannot parse reset timestamp as u64: {e}");
            tracing::error!("{}", msg);
            GithubError::HeaderError(msg)
        })?;

        // All good — update the shared state
        let mut state = self.rate_limit.lock().await;
        state.remaining = remaining;
        let reset_in = reset_unix.saturating_sub(chrono::Utc::now().timestamp() as u64);
        state.reset_at = Instant::now() + Duration::from_secs(reset_in);

        tracing::debug!("Rate limit updated: {} remaining, resets in {}s", remaining, reset_in);
        Ok(())
    }
}

#[async_trait]
impl GithubClient for DefaultGithubClient {
    /// Check if a repository exists.
    async fn repo_exists(&self, owner: &str, name: &str) -> Result<bool, GithubError> {
        tracing::debug!("Checking if repository {}/{} exists", owner, name);
        let data = self
            .execute_graphql::<Repository>(repository::Variables {
                owner: owner.to_string(),
                name: name.to_string(),
            })
            .await?;

        Ok(data.repository.is_some())
    }

    /// Get issues by label.
    async fn repo_issues_by_label(
        &self,
        owner: &str,
        name: &str,
        labels: HashSet<String>,
    ) -> Result<Vec<issues::IssuesRepositoryIssuesNodes>, GithubError> {
        let data = self
            .execute_graphql::<Issues>(issues::Variables {
                owner: owner.to_string(),
                name: name.to_string(),
                labels: Some(labels.into_iter().collect()),
                first: Some(10),
            })
            .await?;

        Ok(data.repository.and_then(|r| r.issues).and_then(|i| i.nodes).unwrap_or_default())
    }

    /// Get repo labels
    async fn repo_labels(
        &self,
        owner: &str,
        name: &str,
    ) -> Result<Vec<labels::LabelsRepositoryLabelsNodes>, GithubError> {
        let data = self
            .execute_graphql::<Labels>(labels::Variables {
                owner: owner.to_string(),
                name: name.to_string(),
            })
            .await?;

        Ok(data.repository.and_then(|r| r.labels).and_then(|l| l.nodes).unwrap_or_default())
    }
}
