use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CallbackAction<'a> {
    #[serde(rename = "vrd")]
    ViewRepoDetails(&'a str, usize), // ViewRepoDetails("owner/repo", from_page)
    #[serde(rename = "vrl")]
    ViewRepoLabels(&'a str, usize, usize), // ViewRepoLabels("owner/repo", labels_page, from_page)
    #[serde(rename = "rrp")]
    RemoveRepoPrompt(&'a str),
    #[serde(rename = "tl")]
    ToggleLabel(&'a str, usize, usize), // ("label", labels_page, from_page)
    #[serde(rename = "brd")]
    BackToRepoDetails(&'a str, usize), // BackToRepoDetails("owner/repo", from_page)
    #[serde(rename = "lrp")]
    ListReposPage(usize), // ListReposPage(page)
    #[serde(rename = "brl")]
    BackToRepoList(usize), // BackToRepoList(page)
    // Command keyboard actions, should be handled as commands:
    CmdHelp,
    CmdList, // List all repos, default page 1
    CmdAdd,
    CmdOverview,
}
