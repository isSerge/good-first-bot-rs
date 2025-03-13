#![warn(missing_docs)]
//! A Telegram bot for tracking beginner-friendly GitHub issues.
//!
//! This bot allows users to track repositories and receive notifications for new issues labeled as "good first issue".
//! It provides a simple interface to add, remove, and list tracked repositories.

mod bot_handler;
mod github;
mod storage;

use crate::bot_handler::{BotHandler, Command};
use crate::storage::Storage;
use anyhow::Context;
use log::debug;
use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use teloxide::prelude::*;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    env_logger::init();

    if let Err(err) = run().await {
        eprintln!("Error: {}", &err);
        std::process::exit(1);
    }
}

async fn run() -> anyhow::Result<()> {
    let storage: Storage = Arc::new(Mutex::new(HashMap::new()));
    let bot = Bot::from_env();
    let github_token =
        env::var("GITHUB_TOKEN").context("GITHUB_TOKEN environment variable is required")?;
    debug!("GitHub token retrieved successfully.");

    let github_client =
        github::GithubClient::new(github_token).context("Failed to create GitHub client")?;
    debug!("GitHub client created successfully.");

    let handler = BotHandler::new(github_client, storage, bot.clone());

    Dispatcher::builder(
        bot,
        dptree::entry().branch(
            Update::filter_message()
                .filter_command::<Command>()
                .endpoint(move |msg: Message, cmd: Command| {
                    let handler = handler.clone();
                    async move { handler.handle_commands(msg, cmd).await }
                }),
        ),
    )
    .enable_ctrlc_handler()
    .build()
    .dispatch()
    .await;
    debug!("Dispatcher built successfully.");

    Ok(())
}
