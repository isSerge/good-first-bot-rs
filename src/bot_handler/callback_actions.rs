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
    #[serde(rename = "tl")]
    ToggleLabel(&'a str, usize), // ("label", page)
    #[serde(rename = "brd")]
    BackToRepoDetails(&'a str),
    #[serde(rename = "lrp")]
    ListReposPage(usize), // ListReposPage(page)
    #[serde(rename = "brl")]
    BackToRepoList, // default page 1
    // Command keyboard actions, should be handled as commands:
    CmdHelp,
    CmdList, // List all repos, default page 1
    CmdAdd,
    CmdOverview,
}
