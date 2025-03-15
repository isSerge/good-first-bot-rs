#![warn(missing_docs)]
//! A Telegram bot for tracking beginner-friendly GitHub issues.
//!
//! This bot allows users to track repositories and receive notifications for new issues labeled as "good first issue".
//! It provides a simple interface to add, remove, and list tracked repositories.

mod bot_handler;
mod config;
mod dispatcher;
mod github;
mod poller;
mod storage;

use crate::bot_handler::{BotHandler, CommandState};
use crate::config::Config;
use crate::poller::GithubPoller;
use crate::storage::Storage;
use anyhow::Result;
use log::debug;
use std::sync::Arc;
use teloxide::{dispatching::dialogue::InMemStorage, prelude::*};

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    env_logger::init();

    if let Err(err) = run().await {
        log::error!("Error: {}", err);
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let config = Config::from_env()?;
    let storage = Arc::new(Storage::new());
    let bot = Bot::new(config.telegram_bot_token.clone());
    let github_client =
        github::GithubClient::new(&config.github_token, &config.github_graphql_url)?;

    // Spawn a polling task for issues.
    let mut github_poller = GithubPoller::new(
        github_client.clone(),
        storage.clone(),
        bot.clone(),
        config.poll_interval,
    );

    tokio::spawn(async move {
        if let Err(e) = github_poller.run().await {
            log::error!("Error in poller: {}", e);
        }
    });

    let dialogue_storage = InMemStorage::<CommandState>::new();
    let handler = Arc::new(BotHandler::new(github_client, storage, bot.clone()));
    let mut dispatcher = dispatcher::BotDispatcher::new(handler, dialogue_storage).build(bot);
    debug!("Dispatcher built successfully.");

    dispatcher.dispatch().await;

    Ok(())
}
