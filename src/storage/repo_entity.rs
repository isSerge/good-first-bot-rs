use std::{fmt, str::FromStr};

use thiserror::Error;
use url::Url;

/// Represents errors that can occur when parsing a `RepoEntity`.
#[derive(Error, Debug, Clone)]
pub enum RepoEntityError {
    /// An error indicating that the URL is invalid.
    #[error("Invalid URL: {0}")]
    Url(String),
    /// An error indicating that the repository format is invalid.
    #[error("Invalid repository format: {0}")]
    Format(String),
    /// An error indicating that the owner or repository name is empty.
    #[error("Owner or repository name cannot be empty")]
    NameWithOwner,
}

type Result<T> = std::result::Result<T, RepoEntityError>;

/// Represents a GitHub repository.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RepoEntity {
    /// The owner of the repository.
    pub owner: String,
    /// The name of the repository.
    pub name: String,
    /// The name of the repository with the owner (e.g., "owner/repo").
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
        let url = Url::parse(url_str).map_err(|_| RepoEntityError::Url(url_str.to_string()))?;
        if url.domain() != Some("github.com") {
            return Err(RepoEntityError::Url(url_str.to_string()));
        }
        let segments: Vec<_> =
            url.path_segments().map(|c| c.collect::<Vec<_>>()).unwrap_or_default();
        if segments.len() < 2 {
            return Err(RepoEntityError::Url(url_str.to_string()));
        }
        let owner = segments[0].to_string();
        let name = segments[1].to_string();
        if owner.is_empty() || name.is_empty() {
            return Err(RepoEntityError::NameWithOwner);
        }

        let name_with_owner = format!("{owner}/{name}");

        Ok(Self { owner, name, name_with_owner })
    }
}

impl fmt::Display for RepoEntity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.name_with_owner, self.url())
    }
}

impl FromStr for RepoEntity {
    type Err = RepoEntityError;

    fn from_str(s: &str) -> Result<Self> {
        let (owner, name) =
            s.split_once('/').ok_or_else(|| RepoEntityError::Format(s.to_string()))?;

        if owner.is_empty() || name.is_empty() {
            return Err(RepoEntityError::NameWithOwner);
        }

        if name.contains('/') {
            return Err(RepoEntityError::Format("Name contains '/'".to_string()));
        }

        let name_with_owner = format!("{owner}/{name}");
        Ok(Self { owner: owner.to_string(), name: name.to_string(), name_with_owner })
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

    #[test]
    fn test_from_url_not_github_domain() {
        let result = RepoEntity::from_url("https://gitlab.com/foo/bar");
        assert!(
            matches!(result, Err(RepoEntityError::Url(s)) if s == "https://gitlab.com/foo/bar")
        );
    }

    #[test]
    fn test_from_str_missing_slash() {
        let result = RepoEntity::from_str("ownerrepo");
        assert!(matches!(result, Err(RepoEntityError::Format(s)) if s == "ownerrepo"));
    }

    #[test]
    fn test_from_str_empty_owner() {
        let result = RepoEntity::from_str("/repo");
        assert!(matches!(result, Err(RepoEntityError::NameWithOwner)));
    }

    #[test]
    fn test_from_str_empty_name() {
        let result = RepoEntity::from_str("owner/");
        assert!(matches!(result, Err(RepoEntityError::NameWithOwner)));
    }

    #[test]
    fn test_from_str_name_contains_slash() {
        let result = RepoEntity::from_str("owner/repo/extra");
        assert!(matches!(result, Err(RepoEntityError::Format(s)) if s == "Name contains '/'"));
    }
}
