mod commands;
#[cfg(test)]
mod tests;

use crate::bot_handler::commands::{CommandContext, CommandHandler};
use crate::messaging::MessagingService;
use crate::repository::RepositoryService;
use crate::storage::RepoEntity;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use teloxide::{
    dispatching::dialogue::{Dialogue, InMemStorage},
    prelude::*,
    types::Message,
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
    messaging_service: Arc<dyn MessagingService>,
    repository_service: Arc<dyn RepositoryService>,
}

/// The state of the command.
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
        messaging_service: Arc<dyn MessagingService>,
        repository_service: Arc<dyn RepositoryService>,
    ) -> Self {
        Self {
            messaging_service,
            repository_service,
        }
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
        let text = msg.text();
        let dialogue_state = dialogue.get().await?;
        // Check if we're waiting for repository input.
        match (dialogue_state, text) {
            (Some(CommandState::AwaitingAddRepo), Some(text)) => {
                self.process_add(text, msg.chat.id).await?
            }
            (Some(CommandState::AwaitingRemoveRepo), Some(text)) => {
                self.process_remove(text, msg.chat.id).await?
            }
            _ => {
                // Should not happen, because force reply does not accept empty input and there are only two possible states, but just in case
                self.messaging_service
                    .send_error_msg(msg.chat.id, anyhow::anyhow!("Invalid input"))
                    .await?;
            }
        }
        dialogue.exit().await?;
        Ok(())
    }

    /// Handle a callback query to remove a repository when the user clicks the remove button on the inline keyboard.
    pub async fn handle_remove_callback_query(&self, query: CallbackQuery) -> Result<()> {
        if let Some(data) = query.data {
            // Extract repository name with owner
            let repo_name_with_owner = data.trim_start_matches("remove:").to_string();

            if let Some(message) = query.message {
                let chat_id = message.chat().id;

                // Attempt to remove the repository.
                let removed = self
                    .repository_service
                    .remove_repo(chat_id, &repo_name_with_owner)
                    .await?;

                // Answer the callback query to clear the spinner.
                self.messaging_service
                    .answer_remove_callback_query(query.id, removed)
                    .await?;

                // If removal was successful, update the inline keyboard on the original message.
                if removed {
                    // Get the updated repository list.
                    let user_repos = self.repository_service.get_user_repos(chat_id).await?;

                    self.messaging_service
                        .edit_list_msg(chat_id, message.id(), user_repos)
                        .await?;
                }
            }
        }
        Ok(())
    }

    /// Prompts the user for repository input and sets the state to waiting for repository input.
    async fn prompt_and_wait_for_reply(
        &self,
        chat_id: ChatId,
        dialogue: &Dialogue<CommandState, InMemStorage<CommandState>>,
        command: Command,
    ) -> Result<()> {
        self.messaging_service
            .prompt_for_repo_input(chat_id)
            .await?;
        let state = match command {
            Command::Add => CommandState::AwaitingAddRepo,
            Command::Remove => CommandState::AwaitingRemoveRepo,
            _ => unreachable!(),
        };
        dialogue.update(state).await?;
        Ok(())
    }

    /// Add a repository to the user's list.
    async fn process_add(&self, text: &str, chat_id: ChatId) -> Result<()> {
        // Parse the repository from the text.
        let repo = match RepoEntity::from_url(text) {
            Ok(repo) => repo,
            Err(e) => {
                self.messaging_service.send_error_msg(chat_id, e).await?;
                return Ok(());
            }
        };

        // Check if the repository exists on GitHub.
        let repo_exists = self
            .repository_service
            .repo_exists(&repo.owner, &repo.name)
            .await;

        // Check if the repository exists on GitHub.
        match repo_exists {
            Ok(true) => {
                let is_already_tracked = self
                    .repository_service
                    .contains_repo(chat_id, &repo)
                    .await?;

                if is_already_tracked {
                    self.messaging_service
                        .send_already_tracked_msg(chat_id, repo.name_with_owner)
                        .await?;
                } else {
                    self.repository_service
                        .add_repo(chat_id, repo.clone())
                        .await?;
                    self.messaging_service
                        .send_repo_added_msg(chat_id, repo.name_with_owner)
                        .await?;
                }
            }
            Ok(false) => {
                self.messaging_service
                    .send_no_repo_exists_msg(chat_id, repo.name_with_owner)
                    .await?;
            }
            Err(e) => {
                self.messaging_service.send_error_msg(chat_id, e).await?;
            }
        }
        Ok(())
    }

    /// Remove a repository from the user's list.
    async fn process_remove(&self, text: &str, chat_id: ChatId) -> Result<()> {
        // Parse the repository from the text.
        let repo = match RepoEntity::from_url(text) {
            Ok(repo) => repo,
            Err(e) => {
                self.messaging_service.send_error_msg(chat_id, e).await?;
                return Ok(());
            }
        };

        let repo_removed = self
            .repository_service
            .remove_repo(chat_id, &repo.name_with_owner)
            .await?;

        if repo_removed {
            self.messaging_service
                .send_repo_removed_msg(chat_id, repo.name_with_owner)
                .await?;
        } else {
            self.messaging_service
                .send_repo_not_tracked_msg(chat_id, repo.name_with_owner)
                .await?;
        }
        Ok(())
    }
}
