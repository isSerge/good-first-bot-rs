use crate::storage::Repository;
use std::collections::HashSet;

/// Returns a formatted string of tracked repositories.
pub fn format_tracked_repos(repos: &HashSet<Repository>) -> String {
    repos
        .iter()
        .map(|r| r.to_string())
        .collect::<Vec<_>>()
        .join("\n")
}
