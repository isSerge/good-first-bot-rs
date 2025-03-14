use anyhow::{Result, anyhow};
use std::{fmt, str::FromStr};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Repository {
    pub owner: String,
    pub name: String,
}

impl Repository {
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

impl FromStr for Repository {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let error = anyhow!("Invalid repository format. Use owner/repo.");
        if let Some((owner, name)) = s.split_once('/') {
            if owner.trim().is_empty() || name.trim().is_empty() {
                Err(error)
            } else {
                Ok(Self {
                    owner: owner.trim().to_owned(),
                    name: name.trim().to_owned(),
                })
            }
        } else {
            Err(error)
        }
    }
}
