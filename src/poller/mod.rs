#[cfg(test)]
mod tests;

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::{Duration, SystemTime},
};

use anyhow::Result;
use chrono::DateTime;
use lazy_static::lazy_static;
use log::debug;
use teloxide::prelude::*;

use crate::{
    github::{GithubClient, issues},
    messaging::MessagingService,
    storage::{RepoEntity, RepoStorage},
};

// TODO: consider replacing polling with webhooks
/// A poller for polling issues from GitHub and sending messages to Telegram.
pub struct GithubPoller {
    github_client: Arc<dyn GithubClient>,
    storage: Arc<dyn RepoStorage>,
    messaging_service: Arc<dyn MessagingService>,
    // The interval to poll GitHub for new issues.
    poll_interval: u64,
}

lazy_static! {
    static ref GOOD_FIRST_ISSUE_LABELS: Vec<String> = vec![
        "good first issue".to_string(),
        "beginner-friendly".to_string(),
        "help wanted".to_string(),
    ];
}

impl GithubPoller {
    /// Create a new GithubPoller.
    pub fn new(
        github_client: Arc<dyn GithubClient>,
        storage: Arc<dyn RepoStorage>,
        messaging_service: Arc<dyn MessagingService>,
        poll_interval: u64,
    ) -> Self {
        Self { github_client, storage, messaging_service, poll_interval }
    }

    /// Run the Poller.
    pub async fn run(&mut self) -> Result<()> {
        debug!("Starting GitHub poller");

        let mut interval = tokio::time::interval(Duration::from_secs(self.poll_interval));

        loop {
            interval.tick().await;
            let repos_by_chat_id = self.storage.get_all_repos().await?;
            self.poll_all_repos(repos_by_chat_id).await?;
        }
    }

    /// Poll all repos for all users.
    async fn poll_all_repos(
        &mut self,
        repos_by_chat_id: HashMap<ChatId, HashSet<RepoEntity>>,
    ) -> Result<()> {
        for (chat_id, repos) in repos_by_chat_id {
            debug!("Polling issues for chat: {}", chat_id);
            for repo in repos {
                // TODO: consider using tokio::spawn to poll repos concurrently
                self.poll_user_repo(chat_id, repo).await?;
            }
        }

        Ok(())
    }

    /// Poll a single repo for a single user.
    async fn poll_user_repo(&mut self, chat_id: ChatId, repo: RepoEntity) -> Result<()> {
        debug!("Polling issues for repository: {}", repo.name_with_owner);

        // Get the last poll time for this repo
        let last_poll_time = self.storage.get_last_poll_time(chat_id, &repo).await?;
        let last_poll_time = last_poll_time
            .map(|t| SystemTime::UNIX_EPOCH + Duration::from_secs(t as u64))
            .unwrap_or(SystemTime::UNIX_EPOCH);

        let issues = self
            .github_client
            .repo_issues_by_label(&repo.owner, &repo.name, GOOD_FIRST_ISSUE_LABELS.to_vec())
            .await;

        match issues {
            Result::Ok(issues) => {
                let issues_to_notify = Self::filter_new_issues(issues, &last_poll_time);

                if !issues_to_notify.is_empty() {
                    debug!("Sending new issuesmessage to chat: {}", chat_id);

                    self.messaging_service
                        .send_new_issues_msg(chat_id, &repo.name_with_owner, issues_to_notify)
                        .await?;

                    // Update the last poll time for this chat/repo pair to now.
                    self.storage.set_last_poll_time(chat_id, &repo).await?;
                } else {
                    debug!("No new issues to notify for {}", repo.name_with_owner);
                }
            }
            Result::Err(e) => {
                // just log the error and keep going for now
                // TODO: handle specific errors
                log::error!("Error polling issues: {:?}", e);
            }
        }

        Ok(())
    }

    fn filter_new_issues(
        issues: Vec<issues::IssuesRepositoryIssuesNodes>,
        last_poll_time: &SystemTime,
    ) -> Vec<issues::IssuesRepositoryIssuesNodes> {
        issues
            .into_iter()
            .filter(|issue| {
                DateTime::parse_from_rfc3339(&issue.created_at)
                    .map(|dt| SystemTime::from(dt) > *last_poll_time)
                    .unwrap_or(false)
            })
            .collect()
    }
}
