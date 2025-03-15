use anyhow::{Result, anyhow};
use std::{fmt, str::FromStr};
use url::Url;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Repository {
    pub owner: String,
    pub name: String,
    pub full_name: String,
}

impl Repository {
    /// Returns the URL of the repository on GitHub.
    pub fn url(&self) -> String {
        format!("https://github.com/{}/{}", self.owner, self.name)
    }
}

impl fmt::Display for Repository {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.full_name, self.url())
    }
}

impl FromStr for Repository {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let url = Url::parse(s).map_err(|e| anyhow!("Invalid URL: {}", e))?;
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

        let full_name = format!("{}/{}", owner, name);

        Ok(Self {
            owner,
            name,
            full_name,
        })
    }
}
