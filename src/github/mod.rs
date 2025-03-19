use anyhow::{Context, Result};
use async_trait::async_trait;
use graphql_client::{GraphQLQuery, Response};
use log::debug;
use mockall::automock;
use reqwest::{
    Client,
    header::{AUTHORIZATION, HeaderMap, HeaderValue, USER_AGENT},
};

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
    variables_derives = "Debug"
)]
pub struct Repository;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/github/schema.graphql",
    query_path = "src/github/github.graphql",
    response_derives = "Debug, Default, serde::Serialize, Clone",
    variables_derives = "Debug"
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

        headers.insert(AUTHORIZATION, HeaderValue::from_str(&format!("Bearer {}", github_token))?);
        headers.insert(USER_AGENT, HeaderValue::from_static("github-activity-rs"));

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .context("Failed to build HTTP client")?;
        debug!("HTTP client built successfully.");

        Ok(Self { client, graphql_url: graphql_url.to_string() })
    }
}

#[async_trait]
impl GithubClient for DefaultGithubClient {
    /// Check if a repository exists.
    async fn repo_exists(&self, owner: &str, name: &str) -> Result<bool> {
        debug!("Checking if repository {}/{} exists", owner, name);
        let variables = repository::Variables { owner: owner.to_string(), name: name.to_string() };

        let request = Repository::build_query(variables);
        debug!("GraphQL request: {:?}", request);

        let res = self
            .client
            .post(&self.graphql_url)
            .json(&request)
            .send()
            .await
            .context("Failed to send request")?;

        let response_body: Response<repository::ResponseData> =
            res.json().await.map_err(|e| anyhow::anyhow!("Failed to parse response: {}", e))?;

        debug!("Response body: {:?}", response_body);

        let graphql_errors = response_body.errors.iter().collect::<Vec<_>>();

        if !graphql_errors.is_empty() {
            return Err(anyhow::anyhow!("GraphQL errors: {:?}", graphql_errors));
        }

        let repo_exists = response_body.data.and_then(|data| data.repository).is_some();

        Ok(repo_exists)
    }

    /// Get issues by label.
    async fn repo_issues_by_label(
        &self,
        owner: &str,
        name: &str,
        labels: Vec<String>,
    ) -> Result<Vec<issues::IssuesRepositoryIssuesNodes>> {
        let variables = issues::Variables {
            owner: owner.to_string(),
            name: name.to_string(),
            labels: Some(labels.into_iter().collect()),
            first: Some(10),
        };

        let request = Issues::build_query(variables);
        debug!("GraphQL request: {:?}", request);

        let res = self
            .client
            .post(&self.graphql_url)
            .json(&request)
            .send()
            .await
            .context("Failed to send request")?;

        let response_body: Response<issues::ResponseData> =
            res.json().await.map_err(|e| anyhow::anyhow!("Failed to parse response: {}", e))?;

        debug!("Response body: {:?}", response_body);

        let graphql_errors = response_body.errors.iter().collect::<Vec<_>>();

        if !graphql_errors.is_empty() {
            return Err(anyhow::anyhow!("GraphQL errors: {:?}", graphql_errors));
        }

        let issues = response_body
            .data
            .and_then(|data| data.repository)
            .and_then(|repo| repo.issues)
            .map(|issues| issues.nodes.unwrap_or_default())
            .unwrap_or_default();

        debug!("Issues: {:?}", issues);

        Ok(issues)
    }
}
