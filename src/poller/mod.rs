#[cfg(test)]
mod tests;

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::{Duration, SystemTime},
};

use chrono::DateTime;
use teloxide::prelude::*;
use thiserror::Error;

use crate::{
    github::{GithubClient, GithubError, issues},
    messaging::{MessagingError, MessagingService},
    storage::{RepoEntity, RepoStorage, StorageError},
};

#[derive(Debug, Error)]
pub enum PollerError {
    #[error("Failed to poll GitHub issues")]
    Github(#[from] GithubError),
    #[error("Failed to access storage")]
    Storage(#[from] StorageError),
    #[error("Failed to send message to Telegram")]
    Messaging(#[from] MessagingError),
}

type Result<T> = std::result::Result<T, PollerError>;

/// A poller for polling issues from GitHub and sending messages to Telegram.
pub struct GithubPoller {
    github_client: Arc<dyn GithubClient>,
    storage: Arc<dyn RepoStorage>,
    messaging_service: Arc<dyn MessagingService>,
    // The interval to poll GitHub for new issues.
    poll_interval: u64,
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
        tracing::debug!("Starting GitHub poller");

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
            tracing::debug!("Polling issues for chat: {chat_id}");
            for repo in repos {
                // TODO: consider using tokio::spawn to poll repos concurrently
                self.poll_user_repo(chat_id, repo).await?;
            }
        }

        Ok(())
    }

    /// Poll a single repo for a single user.
    async fn poll_user_repo(&mut self, chat_id: ChatId, repo: RepoEntity) -> Result<()> {
        tracing::debug!("Polling issues for repository: {}", repo.name_with_owner);

        let tracked_labels =
            self.storage.get_tracked_labels(chat_id, &repo).await.map_err(PollerError::Storage)?;

        // If there are no tracked labels, skip this repo
        if tracked_labels.is_empty() {
            tracing::debug!("No tracked labels for repository: {}", repo.name_with_owner);
            return Ok(());
        }

        // Get the last poll time for this repo
        let last_poll_time = self.storage.get_last_poll_time(chat_id, &repo).await?;
        let last_poll_time = last_poll_time
            .map(|t| SystemTime::UNIX_EPOCH + Duration::from_secs(t as u64))
            .unwrap_or(SystemTime::UNIX_EPOCH);

        let issues = self
            .github_client
            .repo_issues_by_label(&repo.owner, &repo.name, tracked_labels)
            .await
            .map_err(PollerError::Github);

        match issues {
            Ok(issues) => {
                let issues_to_notify = Self::filter_new_issues(issues, &last_poll_time);

                if !issues_to_notify.is_empty() {
                    tracing::debug!("Sending new issues message to chat: {chat_id}");

                    let msg_result = self
                        .messaging_service
                        .send_new_issues_msg(chat_id, &repo.name_with_owner, issues_to_notify)
                        .await;

                    // If sending the message fails, log the error and return without updating the
                    // last poll time
                    if let Err(e) = msg_result {
                        tracing::error!(
                            "Failed to send new issues message for repo {}: {e:?}. Will be \
                             retried next cycle",
                            repo.name_with_owner
                        );
                        return Ok(());
                    }

                    // If the message was sent successfully, update the last poll time
                    let set_last_poll_result =
                        self.storage.set_last_poll_time(chat_id, &repo).await;

                    if let Err(e) = set_last_poll_result {
                        tracing::error!(
                            "Failed to update last poll time for repo {}: {e:?}",
                            repo.name_with_owner
                        );
                    } else {
                        tracing::debug!(
                            "Sent notifications and updated last poll time for repo {} in chat {}",
                            repo.name_with_owner,
                            chat_id
                        );
                    }
                } else {
                    tracing::debug!("No new issues to notify for {}", repo.name_with_owner);
                }
            }
            Err(e) => match e {
                PollerError::Github(github_error) => match github_error {
                    GithubError::GraphQLApiError(msg) => {
                        tracing::error!(
                            "A GraphQL API error occurred while polling repo {} (chat {}): {}. \
                             Skipping this repo for this cycle.",
                            repo.name_with_owner,
                            chat_id,
                            msg
                        );
                    }
                    GithubError::RateLimited => {
                        tracing::warn!(
                            "Rate limit exceeded while polling issues for repository {}. Will \
                             retry later.",
                            repo.name_with_owner
                        );
                    }
                    GithubError::RequestError { source } => {
                        tracing::warn!(
                            "A network/HTTP request error occurred for repo {} (chat {}): {}. \
                             Skipping this repo for this cycle.",
                            repo.name_with_owner,
                            chat_id,
                            source
                        );
                    }
                    GithubError::Unauthorized
                    | GithubError::InvalidHeader(_)
                    | GithubError::SerializationError { .. } => {
                        tracing::error!(
                            "Fatal error while polling issues for repository {}: {github_error:?}",
                            repo.name_with_owner
                        );
                        return Err(PollerError::Github(github_error));
                    }
                },
                unexpected_error => {
                    tracing::error!(
                        "Unexpected error type {:?} at GitHub fetch stage for repo {} (chat {}). \
                         Propagating.",
                        unexpected_error,
                        repo.name_with_owner,
                        chat_id
                    );
                    return Err(unexpected_error);
                }
            },
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
