#![warn(missing_docs)]
//! A Telegram bot for tracking beginner-friendly GitHub issues.
//!
//! This bot allows users to track repositories and receive notifications for new issues labeled as "good first issue".
//! It provides a simple interface to add, remove, and list tracked repositories.

mod bot_handler;
mod config;
mod github;
mod storage;

use crate::bot_handler::{BotHandler, Command, CommandState};
use crate::config::Config;
use crate::storage::Storage;
use anyhow::{Error, Ok, Result};
use log::debug;
use std::sync::Arc;
use teloxide::dispatching::DpHandlerDescription;
use teloxide::dispatching::dialogue::{Dialogue, InMemStorage};
use teloxide::dptree::filter_map;
use teloxide::prelude::*;
use teloxide::types::Update;

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
    let config = Config::from_env()?;
    let storage = Storage::new();
    let bot = Bot::new(config.telegram_bot_token.clone());
    let github_client = github::GithubClient::new(config.github_token, config.github_graphql_url)?;
    let dialogue_storage = InMemStorage::<CommandState>::new();
    let handler = Arc::new(BotHandler::new(github_client, storage, bot.clone()));

    let commands_branch = build_commands_branch();
    let callback_queries_branch = build_callback_queries_branch();
    let force_reply_branch = build_force_reply_branch();

    Dispatcher::builder(
        bot,
        dptree::entry()
            .branch(commands_branch)
            .branch(callback_queries_branch)
            // Branch for handling only force-reply texts.
            .branch(force_reply_branch),
    )
    .dependencies(dptree::deps![dialogue_storage, handler])
    .enable_ctrlc_handler()
    .build()
    .dispatch()
    .await;
    debug!("Dispatcher built successfully.");

    Ok(())
}

fn build_commands_branch()
-> Handler<'static, DependencyMap, Result<(), Error>, DpHandlerDescription> {
    Update::filter_message()
        .filter_command::<Command>()
        .chain(filter_map(
            |update: Update, storage: Arc<InMemStorage<CommandState>>| {
                update.chat().map(|chat| Dialogue::new(storage, chat.id))
            },
        ))
        .endpoint(
            move |msg: Message,
                  cmd: Command,
                  dialogue: Dialogue<CommandState, InMemStorage<CommandState>>,
                  handler: Arc<BotHandler>| {
                async move {
                    handler.handle_commands(msg, cmd, dialogue).await?;
                    Ok(())
                }
            },
        )
}

fn build_callback_queries_branch()
-> Handler<'static, DependencyMap, Result<(), Error>, DpHandlerDescription> {
    Update::filter_callback_query()
        // Insert the dialogue extractor for callback queries.
        .chain(filter_map(
            |update: Update, storage: Arc<InMemStorage<CommandState>>| {
                update.chat().map(|chat| Dialogue::new(storage, chat.id))
            },
        ))
        .endpoint(
            move |query: CallbackQuery,
                  dialogue: Dialogue<CommandState, InMemStorage<CommandState>>,
                  handler: Arc<BotHandler>| {
                async move {
                    if let Some(msg) = query.message.as_ref().and_then(|m| m.regular_message()) {
                        if let Some(data) = query.data {
                            // Map the callback data to the appropriate command.
                            let command = match data.as_str() {
                                "help" => Command::Help,
                                "list" => Command::List,
                                "add" => Command::Add(String::new()),
                                "remove" => Command::Remove(String::new()),
                                _ => return Ok(()),
                            };
                            // Pass the accessible message (cloned) to the command handler.
                            handler
                                .handle_commands(msg.clone(), command, dialogue)
                                .await?;
                        }
                    }
                    Ok(())
                }
            },
        )
}

fn build_force_reply_branch()
-> Handler<'static, DependencyMap, Result<(), Error>, DpHandlerDescription> {
    Update::filter_message()
        .filter(|msg: Message| msg.reply_to_message().is_some())
        // Insert the dialogue extractor
        .chain(filter_map(
            |update: Update, storage: Arc<InMemStorage<CommandState>>| {
                update.chat().map(|chat| Dialogue::new(storage, chat.id))
            },
        ))
        .endpoint(
            move |msg: Message,
                  dialogue: Dialogue<CommandState, InMemStorage<CommandState>>,
                  handler: Arc<BotHandler>| {
                async move { handler.handle_reply(msg, dialogue).await }
            },
        )
}
