use serde::{Deserialize, Serialize};

/// Represents the actions that can be triggered by an inline keyboard button.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CallbackAction<'a> {
    /// View the details of a specific repository.
    #[serde(rename = "vrd")]
    ViewRepoDetails(&'a str, usize), // ("owner/repo", from_page)
    /// View the labels for a specific repository.
    #[serde(rename = "vrl")]
    ViewRepoLabels(&'a str, usize, usize), // ("owner/repo", labels_page, from_page)
    /// Prompt the user to confirm removing a repository.
    #[serde(rename = "rrp")]
    RemoveRepoPrompt(&'a str),
    /// Toggle a label for a repository.
    #[serde(rename = "tl")]
    ToggleLabel(&'a str, usize, usize), // ("label", labels_page, from_page)
    /// Go back to the repository details view.
    #[serde(rename = "brd")]
    BackToRepoDetails(&'a str, usize), // ("owner/repo", from_page)
    /// Paginate through the list of repositories.
    #[serde(rename = "lrp")]
    ListReposPage(usize), // (page)
    /// Go back to the main repository list view.
    #[serde(rename = "brl")]
    BackToRepoList(usize), // (page)
    /// A command to show the help message, triggered from a button.
    CmdHelp,
    /// A command to list all repositories, triggered from a button.
    CmdList,
    /// A command to add a new repository, triggered from a button.
    CmdAdd,
    /// A command to show the overview, triggered from a button.
    CmdOverview,
}
