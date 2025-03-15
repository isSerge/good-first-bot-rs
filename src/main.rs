#![warn(missing_docs)]
//! A Telegram bot for tracking beginner-friendly GitHub issues.
//!
//! This bot allows users to track repositories and receive notifications for new issues labeled as "good first issue".
//! It provides a simple interface to add, remove, and list tracked repositories.

mod bot_handler;
mod config;
mod dispatcher;
mod github;
mod storage;

use crate::bot_handler::{BotHandler, CommandState};
use crate::config::Config;
use crate::storage::Storage;
use anyhow::Result;
use chrono::DateTime;
use log::debug;
use std::{collections::HashMap, sync::Arc, time::SystemTime};
use teloxide::{dispatching::dialogue::InMemStorage, prelude::*};

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    env_logger::init();

    if let Err(err) = run().await {
        eprintln!("Error: {}", err);
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let config = Config::from_env()?;
    let storage = Arc::new(Storage::new());
    let bot = Bot::new(config.telegram_bot_token.clone());
    let github_client = github::GithubClient::new(config.github_token, config.github_graphql_url)?;

    // This map tracks the last poll time per (chat, repository) pair.
    let mut last_poll_times: HashMap<(ChatId, String), SystemTime> = HashMap::new();
    // Spawn a polling task for issues.
    let github_client_for_poll = github_client.clone();
    let storage_for_poll = storage.clone();

    tokio::spawn(async move {
        loop {
            debug!("Polling issues for repository...");

            let repos = storage_for_poll.get_all_repos().await;

            for (chat_id, repos) in repos {
                for repo in repos {
                    let key = (chat_id, repo.full_name().clone());
                    let last_poll_time = last_poll_times
                        .entry(key.clone())
                        .or_insert(SystemTime::UNIX_EPOCH);
                    let issues = github_client_for_poll
                        .repo_issues_by_label(&repo.owner, &repo.name, "good first issue")
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
                                    "New good first issues in {}: {}",
                                    repo.full_name(),
                                    issues_to_notify.len()
                                );

                                debug!("Sending message to {}", chat_id);
                                debug!("{}", message);
                                // TODO: send message
                                // Update the last poll time for this chat/repo pair to now.
                                last_poll_times.insert(key, SystemTime::now());
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

            // Sleep for 60 seconds before polling again.
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        }
    });

    let dialogue_storage = InMemStorage::<CommandState>::new();
    let handler = Arc::new(BotHandler::new(github_client, storage, bot.clone()));
    let mut dispatcher = dispatcher::BotDispatcher::new(handler, dialogue_storage).build(bot);
    debug!("Dispatcher built successfully.");

    dispatcher.dispatch().await;

    Ok(())
}
