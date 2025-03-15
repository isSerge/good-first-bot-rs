use crate::github;
use crate::storage::Storage;
use anyhow::Result;
use chrono::DateTime;
use lazy_static::lazy_static;
use log::debug;
use std::{collections::HashMap, sync::Arc, time::Duration, time::SystemTime};
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
    pub async fn run(&mut self) -> Result<()> {
        let mut interval = tokio::time::interval(Duration::from_secs(self.poll_interval));

        loop {
            interval.tick().await;

            debug!("Polling issues for repository...");

            let repos = self.storage.get_all_repos().await;

            for (chat_id, repos) in repos {
                for repo in repos {
                    let key = (chat_id, repo.full_name().clone());
                    let last_poll_time = self
                        .last_poll_times
                        .entry(key.clone())
                        .or_insert(SystemTime::UNIX_EPOCH);
                    let issues = self
                        .github_client
                        .repo_issues_by_label(
                            &repo.owner,
                            &repo.name,
                            GOOD_FIRST_ISSUE_LABELS.to_vec(),
                        )
                        .await;

                    match issues {
                        Result::Ok(issues) => {
                            let issues_to_notify: Vec<
                                &github::issues::IssuesRepositoryIssuesNodes,
                            > = issues
                                .iter()
                                .filter(|issue| {
                                    let created_at =
                                        DateTime::parse_from_rfc3339(&issue.created_at);

                                    if let Ok(created_at) = created_at {
                                        let issue_time = SystemTime::from(created_at);
                                        issue_time > *last_poll_time
                                    } else {
                                        false
                                    }
                                })
                                .collect();

                            if !issues_to_notify.is_empty() {
                                let message = format!(
                                    "🚨 New issues in {}:\n\n{}",
                                    repo.full_name(),
                                    issues_to_notify
                                        .iter()
                                        .map(|issue| format!("- {}: {}", issue.title, issue.url))
                                        .collect::<Vec<_>>()
                                        .join("\n")
                                );

                                self.bot.send_message(chat_id, message).await?;

                                // Update the last poll time for this chat/repo pair to now.
                                self.last_poll_times.insert(key, SystemTime::now());
                            } else {
                                debug!("No new issues to notify for {}", repo.full_name());
                            }
                        }
                        Result::Err(e) => {
                            println!("Error polling issues: {:?}", e);
                            // TODO: handle error
                        }
                    }
                }
            }
        }
    }
}
