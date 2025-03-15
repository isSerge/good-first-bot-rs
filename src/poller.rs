use crate::github;
use crate::storage::Repository;
use crate::storage::Storage;
use anyhow::Result;
use chrono::DateTime;
use lazy_static::lazy_static;
use log::debug;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
    time::SystemTime,
};
use teloxide::{Bot, prelude::*, types::ChatId};

/// A poller for polling issues from GitHub and sending messages to Telegram.
pub struct GithubPoller {
    github_client: github::GithubClient,
    storage: Arc<Storage>,
    bot: Bot,
    // The interval to poll GitHub for new issues.
    poll_interval: u64,
    // This map tracks the last poll time per (chat, repository) pair.
    last_poll_times: HashMap<(ChatId, String), SystemTime>,
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
        github_client: github::GithubClient,
        storage: Arc<Storage>,
        bot: Bot,
        poll_interval: u64,
    ) -> Self {
        Self {
            github_client,
            storage,
            bot,
            poll_interval,
            last_poll_times: HashMap::new(),
        }
    }

    /// Run the Poller.
    // TODO: consider processing repos concurrently (if rate limit allows)
    pub async fn run(&mut self) -> Result<()> {
        debug!("Starting GitHub poller");

        let mut interval = tokio::time::interval(Duration::from_secs(self.poll_interval));

        loop {
            interval.tick().await;
            let repos_by_chat_id = self.storage.get_all_repos().await;
            self.poll_all_repos(repos_by_chat_id).await?;
        }
    }

    /// Poll all repos for all users.
    async fn poll_all_repos(
        &mut self,
        repos_by_chat_id: HashMap<ChatId, HashSet<Repository>>,
    ) -> Result<()> {
        for (chat_id, repos) in repos_by_chat_id {
            debug!("Polling issues for chat: {}", chat_id);
            for repo in repos {
                self.poll_user_repo(chat_id, repo).await?;
            }
        }

        Ok(())
    }

    /// Poll a single repo for a single user.
    async fn poll_user_repo(&mut self, chat_id: ChatId, repo: Repository) -> Result<()> {
        debug!("Polling issues for repository: {}", repo.full_name);

        let key = (chat_id, repo.full_name.clone());

        let last_poll_time = self
            .last_poll_times
            .entry(key.clone())
            .or_insert(SystemTime::UNIX_EPOCH);

        let issues = self
            .github_client
            .repo_issues_by_label(&repo.owner, &repo.name, GOOD_FIRST_ISSUE_LABELS.to_vec())
            .await;

        match issues {
            Result::Ok(issues) => {
                let issues_to_notify = Self::filter_new_issues(issues, last_poll_time);

                if !issues_to_notify.is_empty() {
                    debug!("Sending new issuesmessage to chat: {}", chat_id);

                    let message = Self::format_message(repo.full_name, issues_to_notify);

                    self.bot.send_message(chat_id, message).await?;

                    // Update the last poll time for this chat/repo pair to now.
                    self.last_poll_times.insert(key, SystemTime::now());
                } else {
                    debug!("No new issues to notify for {}", repo.full_name);
                }
            }
            Result::Err(e) => {
                log::error!("Error polling issues: {:?}", e);
                // TODO: handle error
            }
        }

        Ok(())
    }

    fn filter_new_issues(
        issues: Vec<github::issues::IssuesRepositoryIssuesNodes>,
        last_poll_time: &SystemTime,
    ) -> Vec<github::issues::IssuesRepositoryIssuesNodes> {
        issues
            .into_iter()
            .filter(|issue| {
                DateTime::parse_from_rfc3339(&issue.created_at)
                    .map(|dt| SystemTime::from(dt) > *last_poll_time)
                    .unwrap_or(false)
            })
            .collect()
    }

    fn format_message(
        repo_full_name: String,
        issues: Vec<github::issues::IssuesRepositoryIssuesNodes>,
    ) -> String {
        format!(
            "🚨 New issues in {}:\n\n{}",
            repo_full_name,
            issues
                .iter()
                .map(|issue| format!("- {}: {}", issue.title, issue.url))
                .collect::<Vec<_>>()
                .join("\n")
        )
    }
}
