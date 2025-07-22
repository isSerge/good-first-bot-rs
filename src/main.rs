#![warn(missing_docs)]
//! A Telegram bot for tracking beginner-friendly GitHub issues.
//!
//! This bot allows users to track repositories and receive notifications for
//! new issues labeled as "good first issue". It provides a simple interface to
//! add, remove, and list tracked repositories.

mod bot_handler;
mod config;
mod dispatcher;
mod github;
mod messaging;
mod pagination;
mod poller;
mod repository;
mod storage;

use std::sync::Arc;

use teloxide::{
    dispatching::dialogue::{SqliteStorage, serializer},
    prelude::*,
};
use tracing_subscriber::EnvFilter;

use crate::{
    bot_handler::BotHandler, config::Config, messaging::TelegramMessagingService,
    poller::GithubPoller, repository::DefaultRepositoryService,
    storage::sqlite::SqliteStorage as ApplicationStorage,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    // Initialize the tracing subscriber
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new(format!("{}={}", module_path!(), "info")))
                .add_directive(format!("dlq_log={}", "error").parse()?),
        )
        .init();

    if let Err(err) = run().await {
        tracing::error!("Error: {err}");
        std::process::exit(1);
    }

    Ok(())
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
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
