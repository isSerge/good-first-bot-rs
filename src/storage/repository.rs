use anyhow::{Result, anyhow};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Repository {
    pub owner: String,
    pub name: String,
}

impl Repository {
    /// Creates a new repository from a full name in the format "owner/repo".
    pub fn from_full_name(full_name: &str) -> Result<Self> {
        let parts: Vec<&str> = full_name.split('/').collect();
        if parts.len() != 2 || parts[0].trim().is_empty() || parts[1].trim().is_empty() {
            return Err(anyhow!("Invalid repository format. Use owner/repo."));
        }
        Ok(Self {
            owner: parts[0].trim().to_owned(),
            name: parts[1].trim().to_owned(),
        })
    }

    /// Returns the full repository name in "owner/repo" format.
    pub fn full_name(&self) -> String {
        format!("{}/{}", self.owner, self.name)
    }

    /// Returns the URL of the repository on GitHub.
    pub fn url(&self) -> String {
        format!("https://github.com/{}/{}", self.owner, self.name)
    }
}

impl fmt::Display for Repository {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
      write!(f, "{} ({})", self.full_name(), self.url())
  }
}
