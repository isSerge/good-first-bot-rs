mod commands;
mod utils;

use crate::bot_handler::commands::{CommandContext, CommandHandler};
use crate::github;
use crate::storage::{RepoStorage, Repository};
use anyhow::Result;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
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
    #[command(description = "Add a repository by replying with the repository url.")]
    Add,
    #[command(description = "Remove a repository by replying with the repository url.")]
    Remove,
    #[command(description = "List tracked repositories.")]
    List,
}

/// Encapsulates the bot, storage and GitHub client.
pub struct BotHandler {
    github_client: github::GithubClient,
    storage: Arc<dyn RepoStorage>,
    bot: Bot,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub enum CommandState {
    #[default]
    None,
    AwaitingAddRepo,
    AwaitingRemoveRepo,
}

impl BotHandler {
    /// Creates a new `BotHandler` instance.
    pub fn new(
        github_client: github::GithubClient,
        storage: Arc<dyn RepoStorage>,
        bot: Bot,
    ) -> Self {
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
        msg: &Message,
        cmd: Command,
        dialogue: Dialogue<CommandState, InMemStorage<CommandState>>,
    ) -> anyhow::Result<()> {
        let ctx = CommandContext {
            handler: self,
            message: msg,
            dialogue: &dialogue,
        };

        cmd.handle(ctx).await
    }

    /// Handle a reply message when we're waiting for repository input.
    pub async fn handle_reply(
        &self,
        msg: &Message,
        dialogue: &Dialogue<CommandState, InMemStorage<CommandState>>,
    ) -> Result<()> {
        // Check if we're waiting for repository input.
        match dialogue.get().await? {
            Some(CommandState::AwaitingAddRepo) => self.process_add(msg).await?,
            Some(CommandState::AwaitingRemoveRepo) => self.process_remove(msg).await?,
            _ => {}
        }
        dialogue.exit().await?;
        Ok(())
    }

    /// Prompts the user for repository input if there was no repository provided initially.
    async fn prompt_for_repo(&self, chat_id: ChatId) -> Result<()> {
        let prompt = "Please reply with the repository url.";
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
        command: Command,
    ) -> Result<()> {
        self.prompt_for_repo(chat_id).await?;
        let state = match command {
            Command::Add => CommandState::AwaitingAddRepo,
            Command::Remove => CommandState::AwaitingRemoveRepo,
            _ => unreachable!(),
        };
        dialogue.update(state).await?;
        Ok(())
    }

    async fn process_add(&self, msg: &Message) -> Result<()> {
        let repo = match self.parse_repo_from_msg(msg) {
            Ok(repo) => repo,
            Err(e) => {
                self.send_response(msg.chat.id, format!("Error parsing repository: {}", e))
                    .await?;
                return Ok(());
            }
        };

        // Check if the repository exists on GitHub.
        match self
            .github_client
            .repo_exists(&repo.owner, &repo.name)
            .await
        {
            Ok(true) => {
                if self.storage.contains(msg.chat.id, &repo).await? {
                    self.send_response(
                        msg.chat.id,
                        format!(
                            "Repository {} is already in your list",
                            repo.name_with_owner
                        ),
                    )
                    .await?;
                } else {
                    self.storage
                        .add_repository(msg.chat.id, repo.clone())
                        .await?;
                    self.send_response(msg.chat.id, format!("Added repo: {}", repo))
                        .await?;
                }
            }
            Ok(false) => {
                self.send_response(msg.chat.id, "Repository does not exist on GitHub.")
                    .await?;
            }
            Err(e) => {
                self.send_response(msg.chat.id, format!("Error checking repository: {}", e))
                    .await?;
            }
        }
        Ok(())
    }

    async fn process_remove(&self, msg: &Message) -> Result<()> {
        let repo = match self.parse_repo_from_msg(msg) {
            Ok(repo) => repo,
            Err(e) => {
                self.send_response(msg.chat.id, format!("Error parsing repository: {}", e))
                    .await?;
                return Ok(());
            }
        };

        if self
            .storage
            .remove_repository(msg.chat.id, &repo.name_with_owner)
            .await?
        {
            self.send_response(msg.chat.id, format!("Removed repo: {}", repo.name))
                .await?;
        } else {
            self.send_response(
                msg.chat.id,
                format!("You are not tracking repo: {}", repo.name),
            )
            .await?;
        }
        Ok(())
    }

    fn parse_repo_from_msg(&self, msg: &Message) -> anyhow::Result<Repository> {
        msg.text()
            .ok_or_else(|| anyhow::anyhow!("No repository url provided"))
            .and_then(|text| {
                Repository::from_url(text)
                    .map_err(|e| anyhow::anyhow!("Failed to parse repository: {}", e))
            })
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
