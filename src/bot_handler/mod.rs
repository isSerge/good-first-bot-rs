mod commands;
mod utils;

use crate::bot_handler::commands::{CommandContext, CommandHandler, add, remove};
use crate::github;
use crate::storage::Storage;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use teloxide::{
    dispatching::dialogue::{Dialogue, InMemStorage},
    prelude::*,
    types::{ForceReply, Message},
    utils::command::BotCommands,
};

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Available commands:")]
pub enum Command {
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
    async fn send_response(&self, chat_id: ChatId, text: impl ToString) -> ResponseResult<()> {
        self.bot
            .send_message(chat_id, text.to_string())
            .await
            .map(|_| ())
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
                match command.as_str() {
                    "add" => add::handle(ctx, msg.text().unwrap()).await?,
                    "remove" => remove::handle(ctx, msg.text().unwrap()).await?,
                    _ => self.send_response(msg.chat.id, "Unknown command").await?,
                }
                dialogue.exit().await?;
            }
            Some(_) => {
                self.send_response(msg.chat.id, "No text found in your reply.")
                    .await?;
                dialogue.exit().await?;
            }
            None => {
                // Do nothing
            }
        }
        Ok(())
    }

    /// Prompts the user for repository input if there was no repository provided initially.
    async fn prompt_for_repo(&self, chat_id: ChatId) -> ResponseResult<()> {
        let prompt = "Please reply with the repository in the format owner/repo.";
        self.bot
            .send_message(chat_id, prompt)
            .reply_markup(ForceReply::new())
            .await
            .map(|_| ())
    }

    /// Prompts the user for repository input and sets the state to waiting for repository input.
    async fn prompt_and_set_state(
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
