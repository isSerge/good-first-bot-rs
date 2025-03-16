use anyhow::{Result, anyhow};
use std::{fmt, str::FromStr};
use url::Url;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Repository {
    pub owner: String,
    pub name: String,
    pub name_with_owner: String,
}

const GITHUB_URL: &str = "https://github.com";

impl Repository {
    /// Returns the URL of the repository on GitHub.
    pub fn url(&self) -> String {
        format!("{}/{}/{}", GITHUB_URL, self.owner, self.name)
    }

    /// Parses a GitHub URL into a Repository.
    pub fn from_url(url_str: &str) -> Result<Self> {
        let url = Url::parse(url_str).map_err(|e| anyhow!("Invalid URL: {}", e))?;
        if url.domain() != Some("github.com") {
            return Err(anyhow!("URL must be from github.com"));
        }
        let segments: Vec<_> = url
            .path_segments()
            .map(|c| c.collect::<Vec<_>>())
            .unwrap_or_default();
        if segments.len() < 2 {
            return Err(anyhow!(
                "URL must be in the format https://github.com/owner/repo"
            ));
        }
        let owner = segments[0].to_string();
        let name = segments[1].to_string();
        if owner.is_empty() || name.is_empty() {
            return Err(anyhow!("Owner or repository name cannot be empty"));
        }

        let name_with_owner = format!("{}/{}", owner, name);

        Ok(Self {
            owner,
            name,
            name_with_owner,
        })
    }
}

impl fmt::Display for Repository {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.name_with_owner, self.url())
    }
}

impl FromStr for Repository {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (owner, name) = s
            .split_once('/')
            .ok_or_else(|| anyhow!("Invalid repository format, expected 'owner/name'"))?;

        if owner.is_empty() {
            return Err(anyhow!("Owner cannot be empty"));
        }
        if name.is_empty() {
            return Err(anyhow!("Repository name cannot be empty"));
        }
        if name.contains('/') {
            return Err(anyhow!("Repository name cannot contain '/'"));
        }

        let name_with_owner = format!("{}/{}", owner, name);
        Ok(Self {
            owner: owner.to_string(),
            name: name.to_string(),
            name_with_owner,
        })
    }
}
