mod commands;
#[cfg(test)]
mod tests;

use std::{collections::HashSet, str::FromStr, sync::Arc};

use serde::{Deserialize, Serialize};
use teloxide::{
    dispatching::dialogue::{Dialogue, InMemStorage, InMemStorageError},
    prelude::*,
    types::Message,
    utils::command::BotCommands,
};
use thiserror::Error;

use crate::{
    bot_handler::commands::{CommandContext, CommandHandler},
    messaging::{MessagingError, MessagingService},
    repository::{RepositoryService, RepositoryServiceError},
    storage::RepoEntity,
};

#[derive(Error, Debug)]
pub enum BotHandlerError {
    #[error("Invalid input")]
    InvalidInput,
    #[error("Failed to get or update dialogue: {0}")]
    DialogueError(#[from] InMemStorageError),
    #[error("Failed to send message: {0}")]
    SendMessageError(#[from] MessagingError),
    #[error("Internal error: {0}")]
    InternalError(#[from] RepositoryServiceError),
}

pub type BotHandlerResult<T> = Result<T, BotHandlerError>;

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
        Self { messaging_service, repository_service }
    }

    /// Dispatches the incoming command to the appropriate handler.
    pub async fn handle_commands(
        &self,
        msg: &Message,
        cmd: Command,
        dialogue: Dialogue<CommandState, InMemStorage<CommandState>>,
    ) -> BotHandlerResult<()> {
        let ctx = CommandContext { handler: self, message: msg, dialogue: &dialogue };

        cmd.handle(ctx).await
    }

    /// Handle a reply message when we're waiting for repository input.
    pub async fn handle_reply(
        &self,
        msg: &Message,
        dialogue: &Dialogue<CommandState, InMemStorage<CommandState>>,
    ) -> BotHandlerResult<()> {
        let text = msg.text();
        let dialogue_state = dialogue.get().await.map_err(BotHandlerError::DialogueError)?;
        // Check if we're waiting for repository input.
        match (dialogue_state, text) {
            (Some(CommandState::AwaitingAddRepo), Some(text)) =>
                self.process_add(text, msg.chat.id).await?,
            (Some(CommandState::AwaitingRemoveRepo), Some(text)) =>
                self.process_remove(text, msg.chat.id).await?,
            _ => {
                // Should not happen, because force reply does not accept empty input and there
                // are only two possible states, but just in case
                self.messaging_service
                    .send_error_msg(msg.chat.id, BotHandlerError::InvalidInput)
                    .await
                    .map_err(BotHandlerError::from)?;
            }
        }
        dialogue.exit().await?;
        Ok(())
    }

    /// Handle a callback query to remove a repository when the user clicks the
    /// remove button on the inline keyboard.
    pub async fn handle_remove_callback_query(
        &self,
        query: &CallbackQuery,
    ) -> BotHandlerResult<()> {
        if let Some(data) = &query.data {
            // Extract repository name with owner
            let repo_name_with_owner = data.trim_start_matches("remove:").to_string();

            if let Some(message) = &query.message {
                let chat_id = message.chat().id;

                // Attempt to remove the repository.
                let removed = self
                    .repository_service
                    .remove_repo(chat_id, &repo_name_with_owner)
                    .await
                    .map_err(BotHandlerError::InternalError)?;

                // Answer the callback query to clear the spinner.
                self.messaging_service.answer_remove_callback_query(&query.id, removed).await?;

                // If removal was successful, update the inline keyboard on the original
                // message.
                if removed {
                    // Get the updated repository list.
                    let user_repos = self.repository_service.get_user_repos(chat_id).await?;

                    self.messaging_service.edit_list_msg(chat_id, message.id(), user_repos).await?;
                }
            }
        }
        Ok(())
    }

    pub async fn handle_details_callback_query(
        &self,
        query: &CallbackQuery,
    ) -> BotHandlerResult<()> {
        if let Some(data) = &query.data {
            // Extract repository name with owner
            let repo_name_with_owner = data.trim_start_matches("details:").to_string();
            let repo = RepoEntity::from_str(&repo_name_with_owner)
                .map_err(|_| BotHandlerError::InvalidInput)?;

            if let Some(message) = &query.message {
                let chat_id = message.chat().id;

                // Answer the callback query to clear the spinner.
                self.messaging_service
                    .answer_details_callback_query(chat_id, message.id(), &repo)
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn handle_labels_callback_query(
        &self,
        query: &CallbackQuery,
    ) -> BotHandlerResult<()> {
        if let Some(data) = &query.data {
            if let Some(message) = &query.message {
                let chat_id = message.chat().id;
                
                let repo_name_with_owner = data.trim_start_matches("labels:").to_string();
                let repo = RepoEntity::from_str(&repo_name_with_owner)
                    .map_err(|_| BotHandlerError::InvalidInput)?;
    
                let labels = self
                    .repository_service
                    .get_repo_labels(chat_id, &repo)
                    .await
                    .map_err(BotHandlerError::InternalError)?;

                // Answer the callback query to clear the spinner.
                self.messaging_service
                    .answer_labels_callback_query(
                        chat_id,
                        message.id(),
                        &labels,
                        &repo_name_with_owner,
                    )
                    .await?;
            }
        }
        Ok(())
    }

    /// Prompts the user for repository input and sets the state to waiting for
    /// repository input.
    async fn prompt_and_wait_for_reply(
        &self,
        chat_id: ChatId,
        dialogue: &Dialogue<CommandState, InMemStorage<CommandState>>,
        command: Command,
    ) -> BotHandlerResult<()> {
        self.messaging_service
            .prompt_for_repo_input(chat_id)
            .await
            .map_err(BotHandlerError::from)?;
        let state = match command {
            Command::Add => CommandState::AwaitingAddRepo,
            Command::Remove => CommandState::AwaitingRemoveRepo,
            _ => unreachable!(),
        };
        dialogue.update(state).await.map_err(BotHandlerError::DialogueError)?;
        Ok(())
    }

    /// Add single or multiple repositories to the user's list.
    async fn process_add(&self, urls: &str, chat_id: ChatId) -> BotHandlerResult<()> {
        // Split the input by newlines or whitespaces
        let urls = urls.split_whitespace().collect::<Vec<_>>();

        // Check if the user provided any URLs
        if urls.is_empty() {
            self.messaging_service.send_error_msg(chat_id, BotHandlerError::InvalidInput).await?;
            return Ok(());
        }

        // Track the results for summary message
        let mut successfully_added = HashSet::<String>::new();
        let mut already_tracked = HashSet::<String>::new();
        let mut not_found = HashSet::<String>::new();
        let mut invalid_urls = HashSet::<String>::new();
        let mut errors = HashSet::<(String, String)>::new();

        // Process each URL separately
        for url in urls {
            // Trim the URL to remove leading and trailing whitespace
            if url.trim().is_empty() {
                continue;
            }

            // Parse the repository from the text.
            let repo = match RepoEntity::from_url(url) {
                Ok(repo) => repo,
                Err(_) => {
                    invalid_urls.insert(url.to_string());
                    continue;
                }
            };

            // Check if the repository exists on GitHub.
            let repo_exists = self.repository_service.repo_exists(&repo.owner, &repo.name).await;

            // Check if the repository exists on GitHub.
            match repo_exists {
                Ok(true) => {
                    let is_already_tracked =
                        self.repository_service.contains_repo(chat_id, &repo).await;

                    match is_already_tracked {
                        Ok(true) => {
                            already_tracked.insert(repo.name_with_owner);
                        }
                        Ok(false) => {
                            let add = self.repository_service.add_repo(chat_id, repo.clone()).await;

                            match add {
                                Ok(_) => {
                                    successfully_added.insert(repo.name_with_owner);
                                }
                                Err(e) => {
                                    errors.insert((repo.name_with_owner, e.to_string()));
                                }
                            }
                        }
                        Err(e) => {
                            errors.insert((repo.name_with_owner, e.to_string()));
                        }
                    }
                }
                Ok(false) => {
                    not_found.insert(repo.name_with_owner);
                }
                Err(e) => {
                    errors.insert((repo.name_with_owner, e.to_string()));
                }
            }
        }

        // Send add summary message
        self.messaging_service
            .send_add_summary_msg(
                chat_id,
                successfully_added,
                already_tracked,
                not_found,
                invalid_urls,
                errors,
            )
            .await?;

        Ok(())
    }

    /// Remove a repository from the user's list.
    async fn process_remove(&self, text: &str, chat_id: ChatId) -> BotHandlerResult<()> {
        // Parse the repository from the text.
        let repo = match RepoEntity::from_url(text) {
            Ok(repo) => repo,
            Err(_) => {
                self.messaging_service
                    .send_error_msg(chat_id, BotHandlerError::InvalidInput)
                    .await?;
                return Ok(());
            }
        };

        let repo_removed =
            self.repository_service.remove_repo(chat_id, &repo.name_with_owner).await?;

        if repo_removed {
            self.messaging_service.send_repo_removed_msg(chat_id, repo.name_with_owner).await?;
        } else {
            self.messaging_service.send_repo_not_tracked_msg(chat_id, repo.name_with_owner).await?;
        }
        Ok(())
    }
}
