use anyhow::Result;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Repository {
    pub url: String,
    pub name_with_owner: String,
}

impl Repository {
    pub fn from_full_name(full_name: &str) -> Result<Self> {
        let (owner, name) = full_name.split_once('/').ok_or(anyhow::anyhow!(
            "Invalid repository format. Use owner/repo."
        ))?;

        if owner.is_empty() || name.is_empty() {
            return Err(anyhow::anyhow!(
                "Invalid repository format. Use owner/repo."
            ));
        }

        Ok(Self {
            name_with_owner: format!("{}/{}", owner, name),
            url: format!("https://github.com/{}/{}", owner, name),
        })
    }
}
