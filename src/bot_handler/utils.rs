use crate::storage::Repository;
use std::collections::HashSet;

/// Returns a formatted string of tracked repositories.
pub fn format_tracked_repos(repos: &HashSet<Repository>) -> String {
    repos
        .iter()
        .map(|r| format!("{} ({})", r.name_with_owner, r.url))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Parses a repository string in "owner/repo" format.
pub fn parse_repo_name(repo_name_with_owner: &str) -> Option<(&str, &str)> {
    repo_name_with_owner.split_once('/')
}
