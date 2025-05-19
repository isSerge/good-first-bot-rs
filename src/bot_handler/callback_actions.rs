use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CallbackAction<'a> {
    ViewRepoDetails(&'a str),
    ViewRepoLabels(&'a str),
    RemoveRepoPrompt(&'a str),
    /// TL(&'a str, &'a str) means "Toggle Label" with the first string being
    /// the repo name with owner and the second being the label name.
    TL(&'a str, &'a str),
    BackToRepoDetails(&'a str),
    BackToRepoList,
}
