use anyhow::{Context, Result};
use graphql_client::{GraphQLQuery, Response};
use log::debug;
use reqwest::Client;
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue, USER_AGENT};

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/github/schema.graphql",
    query_path = "src/github/github.graphql",
    response_derives = "Debug, Default, serde::Serialize, Clone",
    variables_derives = "Debug"
)]
pub struct Repository;

#[derive(Clone)]
pub struct GithubClient {
    client: Client,
}

impl GithubClient {
    pub fn new(github_token: String) -> Result<Self> {
        // Build the HTTP client with the GitHub token.
        let mut headers = HeaderMap::new();

        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", github_token))
                .context("Failed to build authorization header")?,
        );
        headers.insert(USER_AGENT, HeaderValue::from_static("github-activity-rs"));

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .context("Failed to build HTTP client")?;
        debug!("HTTP client built successfully.");

        Ok(Self { client })
    }

    /// Check if a repository exists.
    pub async fn repo_exists(&self, owner: &str, name: &str) -> Result<bool> {
        let variables = repository::Variables {
            owner: owner.to_string(),
            name: name.to_string(),
        };

        let request = Repository::build_query(variables);
        debug!("GraphQL request: {:?}", request);

        let graphql_url = std::env::var("GITHUB_GRAPHQL_URL")
            .unwrap_or_else(|_| "https://api.github.com/graphql".into());

        let res = self
            .client
            .post(&graphql_url)
            .json(&request)
            .send()
            .await
            .context("Failed to send request")?;

        let response_body: Response<repository::ResponseData> =
            res.json().await.context("Failed to parse response")?;
        debug!("GraphQL response: {:?}", response_body);

        let repo_exists = if let Some(data) = response_body.data {
            data.repository.is_some()
        } else {
            return Err(anyhow::anyhow!("No data in GraphQL response"));
        };

        Ok(repo_exists)
    }
}
