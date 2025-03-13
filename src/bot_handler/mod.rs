mod commands;
mod utils;

use crate::bot_handler::commands::{
    CommandContext, CommandHandler, add::AddCommand, help::HelpCommand, list::ListCommand,
    remove::RemoveCommand,
};
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
      // Extract command argument if applicable.
      let args = match &cmd {
          Command::Add(arg) | Command::Remove(arg) => Some(arg.clone()),
          _ => None,
      };
  
      // Create the command context. (Dialogue may still be passed if needed.)
      let ctx = CommandContext {
          handler: self,
          message: &msg,
          args,
          dialogue: &dialogue,
      };
  
      // Match on the command and call its handler directly.
      match cmd {
          Command::Help => HelpCommand.handle(ctx).await?,
          Command::List => ListCommand.handle(ctx).await?,
          Command::Add(_) => AddCommand.handle(ctx).await?,
          Command::Remove(_) => RemoveCommand.handle(ctx).await?,
      }
      Ok(())
  }

    /// Handle a reply message when we're waiting for repository input.
    pub async fn handle_reply(
        &self,
        msg: Message,
        dialogue: Dialogue<CommandState, InMemStorage<CommandState>>,
    ) -> Result<()> {
        let args = msg.text().map(|s| s.to_string());
        // Create a common command context.
        let ctx = CommandContext {
            handler: self,
            message: &msg,
            dialogue: &dialogue,
            args,
        };
        // Check if we're waiting for repository input.
        match dialogue.get().await? {
            Some(CommandState::WaitingForRepo { command }) if msg.text().is_some() => {
                match command.as_str() {
                    "add" => AddCommand.handle(ctx).await?,
                    "remove" => RemoveCommand.handle(ctx).await?,
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
}
