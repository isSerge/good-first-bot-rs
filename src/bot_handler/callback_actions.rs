use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CallbackAction<'a> {
    #[serde(rename = "vrd")]
    ViewRepoDetails(&'a str),
    #[serde(rename = "vrl")]
    ViewRepoLabels(&'a str, usize), // ViewRepoLabels("owner/repo", page)
    #[serde(rename = "rrp")]
    RemoveRepoPrompt(&'a str),
    // TODO: this might exceed 64 bytes TG limit if repo name is too long, consider better approach
    /// TL(&'a str, &'a str) means "Toggle Label"
    #[serde(rename = "tl")]
    ToggleLabel(&'a str, &'a str, usize), // TL("owner/repo", "label", page)
    #[serde(rename = "brd")]
    BackToRepoDetails(&'a str),
    #[serde(rename = "lrp")]
    ListReposPage(usize), // ListReposPage(page)
    #[serde(rename = "brl")]
    BackToRepoList, // default page 1
    // Command keyboard actions, should be handled as commands:
    Help,
    List, // List all repos, default page 1
    Add,
}
