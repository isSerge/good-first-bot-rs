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
use teloxide::dispatching::dialogue::{Dialogue, InMemStorage};
use teloxide::dispatching::{DefaultKey, DpHandlerDescription};
use teloxide::dptree::{deps, filter_map};
use teloxide::prelude::*;
use teloxide::types::Update;

/// Type alias to simplify handler type signatures.
type BotResultHandler = Handler<'static, DependencyMap, Result<(), Error>, DpHandlerDescription>;

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
    let storage = Storage::new();
    let bot = Bot::new(config.telegram_bot_token.clone());
    let github_client = github::GithubClient::new(config.github_token, config.github_graphql_url)?;
    let dialogue_storage = InMemStorage::<CommandState>::new();
    let handler = Arc::new(BotHandler::new(github_client, storage, bot.clone()));
    let mut dispatcher = BotDispatcher::new(handler, dialogue_storage).build(bot);
    debug!("Dispatcher built successfully.");

    dispatcher.dispatch().await;

    Ok(())
}

struct BotDispatcher {
    handler: Arc<BotHandler>,
    dialogue_storage: Arc<InMemStorage<CommandState>>,
}

impl BotDispatcher {
    pub fn new(
        handler: Arc<BotHandler>,
        dialogue_storage: Arc<InMemStorage<CommandState>>,
    ) -> Self {
        Self {
            handler,
            dialogue_storage,
        }
    }

    pub fn build(&self, bot: Bot) -> Dispatcher<Bot, Error, DefaultKey> {
        Dispatcher::builder(
            bot,
            dptree::entry()
                .branch(self.build_commands_branch())
                .branch(self.build_callback_queries_branch())
                .branch(self.build_force_reply_branch()),
        )
        .dependencies(deps![self.dialogue_storage.clone(), self.handler.clone()])
        .enable_ctrlc_handler()
        .build()
    }

    /// Builds the branch for handling text commands.
    fn build_commands_branch(&self) -> BotResultHandler {
        Update::filter_message()
            .filter_command::<Command>()
            .chain(filter_map(extract_dialogue))
            .endpoint(
                |msg: Message,
                 cmd: Command,
                 dialogue: Dialogue<CommandState, InMemStorage<CommandState>>,
                 handler: Arc<BotHandler>| async move {
                    handler.handle_commands(msg, cmd, dialogue).await?;
                    Ok(())
                },
            )
    }

    /// Builds the branch for handling callback queries.
    fn build_callback_queries_branch(&self) -> BotResultHandler {
        Update::filter_callback_query()
            .chain(filter_map(extract_dialogue))
            .endpoint(
                |query: CallbackQuery,
                 dialogue: Dialogue<CommandState, InMemStorage<CommandState>>,
                 handler: Arc<BotHandler>| async move {
                    if let Some(msg) = query.message.as_ref().and_then(|m| m.regular_message()) {
                        if let Some(data) = query.data.as_deref() {
                            if let Some(command) = parse_callback_command(data) {
                                handler
                                    .handle_commands(msg.clone(), command, dialogue)
                                    .await?;
                            }
                        }
                    }
                    Ok(())
                },
            )
    }

    /// Builds the branch for handling messages that are force-reply responses.
    fn build_force_reply_branch(&self) -> BotResultHandler {
        Update::filter_message()
            .filter(|msg: Message| msg.reply_to_message().is_some())
            .chain(filter_map(extract_dialogue))
            .endpoint(
                |msg: Message,
                 dialogue: Dialogue<CommandState, InMemStorage<CommandState>>,
                 handler: Arc<BotHandler>| async move {
                    handler.handle_reply(msg, dialogue).await
                },
            )
    }
}
/// Helper that extracts a dialogue from an update using the provided dialogue storage.
fn extract_dialogue(
    update: Update,
    storage: Arc<InMemStorage<CommandState>>,
) -> Option<Dialogue<CommandState, InMemStorage<CommandState>>> {
    update.chat().map(|chat| Dialogue::new(storage, chat.id))
}

/// Helper that converts callback data into a corresponding command.
fn parse_callback_command(data: &str) -> Option<Command> {
    match data {
        "help" => Some(Command::Help),
        "list" => Some(Command::List),
        "add" => Some(Command::Add(String::new())),
        "remove" => Some(Command::Remove(String::new())),
        _ => None,
    }
}
