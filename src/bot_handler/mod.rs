mod commands;
mod utils;

use crate::bot_handler::commands::{CommandContext, CommandHandler, add, remove};
use crate::github;
use crate::storage::Storage;
use anyhow::Result;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use teloxide::{
    dispatching::dialogue::{Dialogue, InMemStorage},
    prelude::*,
    types::{ForceReply, InlineKeyboardButton, InlineKeyboardMarkup, Message},
    utils::command::BotCommands,
};

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Available commands:")]
pub enum Command {
    #[command(description = "Start the bot and show welcome message.")]
    Start,
    #[command(description = "Show this help text.")]
    Help,
    #[command(description = "Add a repository (e.g., owner/repo).")]
    Add(String),
    #[command(description = "Remove a repository.")]
    Remove(String),
    #[command(description = "List tracked repositories.")]
    List,
}

/// Encapsulates the bot, storage and GitHub client.
pub struct BotHandler {
    github_client: github::GithubClient,
    storage: Storage,
    bot: Bot,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub enum CommandState {
    #[default]
    None,
    WaitingForRepo {
        command: String,
    },
}

impl BotHandler {
    /// Creates a new `BotHandler` instance.
    pub fn new(github_client: github::GithubClient, storage: Storage, bot: Bot) -> Self {
        Self {
            github_client,
            storage,
            bot,
        }
    }

    /// Sends a text message to the provided chat.
    async fn send_response(&self, chat_id: ChatId, text: impl ToString) -> Result<()> {
        self.bot
            .send_message(chat_id, text.to_string())
            .reply_markup(COMMAND_KEYBOARD.clone())
            .await
            .map(|_| ())
            .map_err(|e| anyhow::anyhow!("Failed to send message: {}", e))
    }

    /// Dispatches the incoming command to the appropriate handler.
    pub async fn handle_commands(
        &self,
        msg: Message,
        cmd: Command,
        dialogue: Dialogue<CommandState, InMemStorage<CommandState>>,
    ) -> anyhow::Result<()> {
        let ctx = CommandContext {
            handler: self,
            message: &msg,
            dialogue: &dialogue,
        };

        cmd.handle(ctx).await
    }

    /// Handle a reply message when we're waiting for repository input.
    pub async fn handle_reply(
        &self,
        msg: Message,
        dialogue: Dialogue<CommandState, InMemStorage<CommandState>>,
    ) -> Result<()> {
        // Create a common command context.
        let ctx = CommandContext {
            handler: self,
            message: &msg,
            dialogue: &dialogue,
        };
        // Check if we're waiting for repository input.
        match dialogue.get().await? {
            Some(CommandState::WaitingForRepo { command }) if msg.text().is_some() => {
                let repo_name = msg.text().unwrap();
                match command.as_str() {
                    "add" => add::handle(ctx, repo_name).await?,
                    "remove" => remove::handle(ctx, repo_name).await?,
                    _ => self.send_response(msg.chat.id, "Unknown command").await?,
                }
            }
            Some(_) => {
                self.send_response(msg.chat.id, "No text found in your reply.")
                    .await?;
            }
            None => {
                // Do nothing
            }
        }
        dialogue.exit().await?;
        Ok(())
    }

    /// Prompts the user for repository input if there was no repository provided initially.
    async fn prompt_for_repo(&self, chat_id: ChatId) -> Result<()> {
        let prompt = "Please reply with the repository in the format owner/repo.";
        self.bot
            .send_message(chat_id, prompt)
            .reply_markup(ForceReply::new())
            .await
            .map(|_| ())
            .map_err(|e| anyhow::anyhow!("Failed to send message: {}", e))
    }

    /// Prompts the user for repository input and sets the state to waiting for repository input.
    async fn prompt_and_wait_for_reply(
        &self,
        chat_id: ChatId,
        dialogue: &Dialogue<CommandState, InMemStorage<CommandState>>,
        command: &str,
    ) -> Result<()> {
        self.prompt_for_repo(chat_id).await?;
        dialogue
            .update(CommandState::WaitingForRepo {
                command: command.into(),
            })
            .await?;
        Ok(())
    }
}

lazy_static! {
    static ref COMMAND_KEYBOARD: InlineKeyboardMarkup = InlineKeyboardMarkup::new(vec![
        vec![
            InlineKeyboardButton::callback("Help", "help"),
            InlineKeyboardButton::callback("List", "list"),
        ],
        vec![
            InlineKeyboardButton::callback("Add", "add"),
            InlineKeyboardButton::callback("Remove", "remove"),
        ],
    ]);
}
