#![warn(missing_docs)]
//! A Telegram bot for tracking GitHub issues.
//!
//! This bot allows users to track repositories and receive notifications for
//! new issues. It provides a simple interface to
//! add, remove, and list tracked repositories.

/// The main handler for the bot's logic.
pub mod bot_handler;
/// The configuration for the application.
pub mod config;
/// The dispatcher for routing updates to the correct handlers.
pub mod dispatcher;
/// The client for interacting with the GitHub API.
pub mod github;
/// The service for sending messages to the user.
pub mod messaging;
/// A utility for paginating data.
pub mod pagination;
/// The poller for fetching new issues from GitHub.
pub mod poller;
/// The service for managing repositories.
pub mod repository;
/// The storage layer for persisting data.
pub mod storage;

use std::sync::Arc;

use teloxide::{
    dispatching::dialogue::{SqliteStorage, serializer},
    prelude::*,
};

use crate::{
    bot_handler::BotHandler, config::Config, messaging::TelegramMessagingService,
    poller::GithubPoller, repository::DefaultRepositoryService,
    storage::sqlite::SqliteStorage as ApplicationStorage,
};

/// Runs the bot.
pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_env()?;
    let storage = Arc::new(ApplicationStorage::new(&config.database_url).await?);
    let bot = Bot::new(config.telegram_bot_token.clone());
    let github_client = Arc::new(github::DefaultGithubClient::new(
        &config.github_token,
        &config.github_graphql_url,
        config.rate_limit_threshold,
    )?);

    let messaging_service = Arc::new(TelegramMessagingService::new(bot.clone()));

    // Spawn a polling task for issues.
    let github_poller = GithubPoller::new(
        github_client.clone(),
        storage.clone(),
        messaging_service.clone(),
        config.poll_interval,
        config.max_concurrency,
    );

    tokio::spawn(async move {
        if let Err(e) = github_poller.run().await {
            tracing::error!("Error in poller: {e}");
        }
    });

    let dialogue_storage = SqliteStorage::open(&config.database_url, serializer::Json).await?;
    let repo_manager_service = Arc::new(DefaultRepositoryService::new(
        storage.clone(),
        github_client.clone(),
        config.max_repos_per_user,
        config.max_labels_per_repo,
    ));
    let handler =
        Arc::new(BotHandler::new(messaging_service, repo_manager_service, config.max_concurrency));
    let mut dispatcher = dispatcher::BotDispatcher::new(handler, dialogue_storage).build(bot);
    tracing::debug!("Dispatcher built successfully.");

    dispatcher.dispatch().await;

    Ok(())
}
