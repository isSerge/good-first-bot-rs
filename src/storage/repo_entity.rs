use anyhow::{Result, anyhow};
use std::{fmt, str::FromStr};
use url::Url;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RepoEntity {
    pub owner: String,
    pub name: String,
    pub name_with_owner: String,
}

const GITHUB_URL: &str = "https://github.com";

impl RepoEntity {
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

impl fmt::Display for RepoEntity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.name_with_owner, self.url())
    }
}

impl FromStr for RepoEntity {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_str() {
        let repo = RepoEntity::from_str("rust-lang/rust").unwrap();
        assert_eq!(repo.owner, "rust-lang");
        assert_eq!(repo.name, "rust");
        assert_eq!(repo.name_with_owner, "rust-lang/rust");
    }

    #[test]
    fn test_from_url() {
        let repo = RepoEntity::from_url("https://github.com/rust-lang/rust").unwrap();
        assert_eq!(repo.owner, "rust-lang");
        assert_eq!(repo.name, "rust");
        assert_eq!(repo.name_with_owner, "rust-lang/rust");
    }

    #[test]
    fn test_from_url_invalid() {
        let repo = RepoEntity::from_url("https://gitlab.com/rust-lang/rust");
        assert!(repo.is_err());
    }

    #[test]
    fn test_from_str_invalid() {
        let repo = RepoEntity::from_str("rust-lang");
        assert!(repo.is_err());
    }

    #[test]
    fn test_from_str_invalid_owner() {
        let repo = RepoEntity::from_str("/rust-lang/rust");
        assert!(repo.is_err());
    }

    #[test]
    fn test_from_str_missing_name() {
        let repo = RepoEntity::from_str("rust-lang/");
        assert!(repo.is_err());
    }

    #[test]
    fn test_from_url_with_path() {
        let repo = RepoEntity::from_url("https://github.com/rust-lang/rust/issues").unwrap();

        assert_eq!(repo.owner, "rust-lang");
        assert_eq!(repo.name, "rust");
        assert_eq!(repo.name_with_owner, "rust-lang/rust");
    }

    #[test]
    fn test_from_url_with_query() {
        let repo = RepoEntity::from_url("https://github.com/rust-lang/rust?tab=issues").unwrap();

        assert_eq!(repo.owner, "rust-lang");
        assert_eq!(repo.name, "rust");
        assert_eq!(repo.name_with_owner, "rust-lang/rust");
    }
}
