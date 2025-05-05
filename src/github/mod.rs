use std::time::Duration;

use anyhow::{Context, Result};
use async_trait::async_trait;
use backoff::{Error as BackoffError, ExponentialBackoff, future::retry};
use graphql_client::{GraphQLQuery, Response};
use log::{debug, error, warn};
use mockall::automock;
use reqwest::{
    Client,
    header::{AUTHORIZATION, HeaderMap, HeaderValue, USER_AGENT},
};

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

#[automock]
#[async_trait]
pub trait GithubClient: Send + Sync {
    /// Check if a repository exists.
    async fn repo_exists(&self, owner: &str, name: &str) -> Result<bool>;

    /// Get issues by label.
    async fn repo_issues_by_label(
        &self,
        owner: &str,
        name: &str,
        labels: Vec<String>,
    ) -> Result<Vec<issues::IssuesRepositoryIssuesNodes>>;
}

// GraphQL DateTime scalar type.
type DateTime = String;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/github/schema.graphql",
    query_path = "src/github/github.graphql",
    response_derives = "Debug, Default, serde::Serialize, Clone",
    variables_derives = "Debug, Clone"
)]
pub struct Repository;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/github/schema.graphql",
    query_path = "src/github/github.graphql",
    response_derives = "Debug, Default, serde::Serialize, Clone",
    variables_derives = "Debug, Clone"
)]
pub struct Issues;

#[derive(Clone)]
pub struct DefaultGithubClient {
    client: Client,
    graphql_url: String,
}

impl DefaultGithubClient {
    pub fn new(github_token: &str, graphql_url: &str) -> Result<Self> {
        // Build the HTTP client with the GitHub token.
        let mut headers = HeaderMap::new();

        headers.insert(AUTHORIZATION, HeaderValue::from_str(&format!("Bearer {github_token}"))?);
        headers.insert(USER_AGENT, HeaderValue::from_static("github-activity-rs"));

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .context("Failed to build HTTP client")?;
        debug!("HTTP client built successfully.");

        Ok(Self { client, graphql_url: graphql_url.to_string() })
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
    async fn execute_graphql<Q>(&self, variables: Q::Variables) -> anyhow::Result<Q::ResponseData>
    where
        Q: GraphQLQuery,
        Q::Variables: Clone,
        Q::ResponseData: serde::de::DeserializeOwned,
    {
        // closure that Backoff expects
        let operation = || async {
            // 1. Build the request
            let request_body = Q::build_query(variables.clone());

            // 2. Send HTTP
            let resp =
                self.client.post(&self.graphql_url).json(&request_body).send().await.map_err(
                    |e| {
                        warn!("Network error sending GraphQL request: {e}. Retrying...");
                        BackoffError::transient(anyhow::Error::new(e))
                    },
                )?;

            // 3. HTTP-status check
            if !resp.status().is_success() {
                let status = resp.status();
                let text = match resp.text().await {
                    Ok(body) => body,
                    Err(e) => {
                        warn!("Failed to read response text: {e}. Using empty fallback.");
                        String::new()
                    }
                };
                warn!("Non-success HTTP {status}: {text}. Retrying if transient...");
                let err = anyhow::anyhow!("HTTP {}: {}", status, text);
                let be = if status.is_server_error()
                    || status == reqwest::StatusCode::TOO_MANY_REQUESTS
                {
                    BackoffError::transient(err)
                } else {
                    BackoffError::permanent(err)
                };
                return Err(be);
            }

            // 4. Parse JSON
            let body: Response<Q::ResponseData> = resp.json().await.map_err(|e| {
                warn!("Failed to parse JSON: {e}. Retrying...");
                BackoffError::transient(anyhow::Error::new(e))
            })?;

            // 5. GraphQL errors?
            if let Some(errors) = &body.errors {
                if !errors.is_empty() {
                    let retryable = errors.iter().any(is_retryable_graphql_error);
                    let msg = format!("GraphQL errors: {errors:?}");
                    if retryable {
                        warn!("Retryable GraphQL error: {msg}. Retrying...");
                        return Err(BackoffError::transient(anyhow::anyhow!(msg)));
                    } else {
                        error!("Permanent GraphQL error: {msg}");
                        return Err(BackoffError::permanent(anyhow::anyhow!(msg)));
                    }
                }
            }

            // 6. Unwrap the data or permanent-fail
            body.data.ok_or_else(|| {
                error!("GraphQL response had no data field; permanent failure");
                BackoffError::permanent(anyhow::anyhow!("No data in GraphQL response"))
            })
        };

        // kick off the retry loop, then convert any backoff::Error into an
        // anyhow::Error
        retry(Self::backoff_config(), operation).await
    }
}

#[async_trait]
impl GithubClient for DefaultGithubClient {
    /// Check if a repository exists.
    async fn repo_exists(&self, owner: &str, name: &str) -> Result<bool> {
        debug!("Checking if repository {}/{} exists", owner, name);
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
        labels: Vec<String>,
    ) -> Result<Vec<issues::IssuesRepositoryIssuesNodes>> {
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
}
