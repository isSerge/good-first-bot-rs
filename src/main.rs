#![warn(missing_docs)]
//! A Telegram bot for tracking beginner-friendly GitHub issues.
//!
//! This bot allows users to track repositories and receive notifications for new issues labeled as "good first issue".
//! It provides a simple interface to add, remove, and list tracked repositories.

mod bot_handler;
mod github;
mod storage;

use crate::bot_handler::{BotHandler, Command, CommandState};
use crate::storage::Storage;
use anyhow::Context;
use log::debug;
use std::env;
use std::sync::Arc;
use teloxide::dispatching::dialogue::{Dialogue, InMemStorage};
use teloxide::prelude::*;

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
    let storage = Storage::new();
    let bot = Bot::from_env();
    let github_token =
        env::var("GITHUB_TOKEN").context("GITHUB_TOKEN environment variable is required")?;
    debug!("GitHub token retrieved successfully.");

    let github_client =
        github::GithubClient::new(github_token, None).context("Failed to create GitHub client")?;
    debug!("GitHub client created successfully.");

    // Dialogue storage for conversation state.
    let dialogue_storage = InMemStorage::<CommandState>::new();

    let handler = Arc::new(BotHandler::new(github_client, storage, bot.clone()));
    let handler_clone = Arc::clone(&handler);

    Dispatcher::builder(
        bot,
        dptree::entry()
            .branch(
                Update::filter_message()
                    .enter_dialogue::<Message, InMemStorage<CommandState>, CommandState>()
                    .branch(
                        Update::filter_message()
                            .filter_command::<Command>()
                            .endpoint(move |msg: Message, cmd: Command, dialogue: Dialogue<CommandState, InMemStorage<CommandState>>| {
                                let handler = handler.clone();
                                async move { handler.handle_commands(msg, cmd, dialogue).await }
                            })
                    )
                    .branch(
                        Update::filter_message()
                            .endpoint(move |msg: Message, dialogue: Dialogue<CommandState, InMemStorage<CommandState>>| {
                                let handler = handler_clone.clone();
                                async move { handler.handle_reply(msg, dialogue).await }
                            })
                    ),
            ),
    )
    .dependencies(dptree::deps![dialogue_storage])
    .enable_ctrlc_handler()
    .build()
    .dispatch()
    .await;
    debug!("Dispatcher built successfully.");

    Ok(())
}
