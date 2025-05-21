use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CallbackAction<'a> {
    ViewRepoDetails(&'a str),
    ViewRepoLabels(&'a str, usize), // ViewRepoLabels("owner/repo", page)
    RemoveRepoPrompt(&'a str),
    // TODO: this might exceed 64 bytes TG limit if repo name is too long, consider better approach
    /// TL(&'a str, &'a str) means "Toggle Label"
    TL(&'a str, &'a str, usize), // TL("owner/repo", "label", page)
    BackToRepoDetails(&'a str),
    ListReposPage(usize), // ListReposPage(page)
    BackToRepoList, // default page 1
    // Command keyboard actions, should be handled as commands:
    Help,
    List, // List all repos, default page 1
    Add,
}
