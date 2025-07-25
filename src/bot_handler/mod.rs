//! This module provides the main bot handler for processing commands and
//! callback queries.
#[allow(missing_docs)]
pub mod callback_actions;
#[allow(missing_docs)]
pub mod callbacks;
#[allow(missing_docs)]
pub mod commands;
#[cfg(test)]
mod tests;

use std::sync::Arc;

pub use callback_actions::CallbackAction;
use serde::{Deserialize, Serialize};
use teloxide::{
    dispatching::dialogue::{Dialogue, SqliteStorage, SqliteStorageError, serializer::Json},
    prelude::*,
    types::Message,
    utils::command::BotCommands,
};
use thiserror::Error;

use crate::{
    bot_handler::commands::CommandHandler,
    messaging::{MessagingError, MessagingService},
    repository::{RepositoryService, RepositoryServiceError},
};

type DialogueStorage = SqliteStorage<Json>;

/// Context groups the data needed by all command and callback handlers.
pub struct Context<'a> {
    /// A reference to the main `BotHandler`.
    pub handler: &'a BotHandler,
    /// The message that triggered the handler.
    pub message: &'a Message,
    /// The dialogue for managing command state.
    pub dialogue: &'a Dialogue<CommandState, DialogueStorage>,
    /// The callback query, if the handler was triggered by one.
    pub query: Option<&'a CallbackQuery>,
}

/// Represents errors that can occur in the bot handler.
#[derive(Error, Debug)]
pub enum BotHandlerError {
    /// Represents an error with invalid user input.
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Represents an error with the dialogue storage.
    #[error("Failed to get or update dialogue: {0}")]
    DialogueError(#[from] SqliteStorageError<serde_json::Error>),

    /// Represents an error sending a message.
    #[error("Failed to send message: {0}")]
    SendMessageError(#[from] MessagingError),

    /// Represents an internal error from the repository service.
    #[error("Internal error: {0}")]
    InternalError(RepositoryServiceError),

    /// Represents an error when a user exceeds a limit.
    #[error("Limit exceeded: {0}")]
    LimitExceeded(String),
}

impl From<RepositoryServiceError> for BotHandlerError {
    fn from(err: RepositoryServiceError) -> Self {
        match err {
            RepositoryServiceError::LimitExceeded(msg) => BotHandlerError::LimitExceeded(msg),
            _ => BotHandlerError::InternalError(err),
        }
    }
}

/// A convenience type alias for `Result<T, BotHandlerError>`.
pub type BotHandlerResult<T> = Result<T, BotHandlerError>;

/// Represents the available bot commands.
#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Available commands:")]
pub enum Command {
    /// Start the bot and show a welcome message.
    #[command(description = "Start the bot and show welcome message.")]
    Start,
    /// Show the help message.
    #[command(description = "Show this help text.")]
    Help,
    /// Add a repository to track.
    #[command(description = "Add a repository by replying with the repository url.")]
    Add,
    /// List the repositories being tracked.
    #[command(description = "List tracked repositories.")]
    List,
    /// Show an overview of all tracked repositories and their labels.
    #[command(description = "Show an overview of tracked repositories.")]
    Overview,
}

/// Encapsulates the bot, storage and GitHub client.
pub struct BotHandler {
    messaging_service: Arc<dyn MessagingService>,
    repository_service: Arc<dyn RepositoryService>,
    max_concurrency: usize,
}

/// The state of the command.
#[derive(Clone, Default, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum CommandState {
    /// The default state, where no command is active.
    #[default]
    None,
    /// The bot is waiting for the user to reply with repository URLs.
    AwaitingAddRepo,
    /// The user is viewing the labels for a specific repository.
    ViewingRepoLabels {
        /// The full name of the repository (e.g., "owner/repo").
        repo_id: String,
        /// The page number of the repository list the user came from.
        from_page: usize,
    },
}

impl BotHandler {
    /// Creates a new `BotHandler` instance.
    pub fn new(
        messaging_service: Arc<dyn MessagingService>,
        repository_service: Arc<dyn RepositoryService>,
        max_concurrency: usize,
    ) -> Self {
        Self { messaging_service, repository_service, max_concurrency }
    }

    /// Dispatches the incoming command to the appropriate handler.
    pub async fn handle_commands(
        &self,
        msg: &Message,
        cmd: Command,
        dialogue: Dialogue<CommandState, DialogueStorage>,
    ) -> BotHandlerResult<()> {
        let ctx = Context { handler: self, message: msg, dialogue: &dialogue, query: None };
        cmd.handle(ctx).await
    }

    /// Handle a reply message when we're waiting for repository input.
    pub async fn handle_reply(
        &self,
        msg: &Message,
        dialogue: &Dialogue<CommandState, DialogueStorage>,
    ) -> BotHandlerResult<()> {
        let text = msg.text();
        let dialogue_state = dialogue.get().await.map_err(BotHandlerError::DialogueError)?;
        // Check if we're waiting for repository input.
        match (dialogue_state, text) {
            (Some(CommandState::AwaitingAddRepo), Some(text)) => {
                let ctx = Context { handler: self, message: msg, dialogue, query: None };
                commands::add::handle_reply(ctx, text).await?;
            }
            _ => {
                // Should not happen, because force reply does not accept empty input and there
                // are only two possible states, but just in case
                self.messaging_service
                    .send_error_msg(
                        msg.chat.id,
                        BotHandlerError::InvalidInput("Invalid state".to_string()),
                    )
                    .await?;
            }
        }
        dialogue.exit().await.map_err(BotHandlerError::DialogueError)?;
        Ok(())
    }

    /// Handles an incoming callback query.
    pub async fn handle_callback_query(
        &self,
        query: &CallbackQuery,
        dialogue: Dialogue<CommandState, DialogueStorage>,
    ) -> BotHandlerResult<()> {
        let query_id = query.id.clone();

        if let Some(data_str) = &query.data.as_deref() {
            let action = serde_json::from_str::<CallbackAction>(data_str)
                .map_err(|e| BotHandlerError::InvalidInput(e.to_string()))?;

            // Answer the callback query to clear the spinner.
            self.messaging_service.answer_callback_query(&query_id, &None).await?;

            let ctx = Context {
                handler: self,
                message: query.message.as_ref().and_then(|m| m.regular_message()).ok_or(
                    BotHandlerError::InvalidInput("Callback query has no message".to_string()),
                )?,
                dialogue: &dialogue,
                query: Some(query),
            };

            match action {
                CallbackAction::ViewRepoDetails(repo_id, from_page) => {
                    callbacks::view_repo::handle(ctx, repo_id, from_page, &query_id).await?;
                }
                CallbackAction::BackToRepoDetails(repo_id, from_page) => {
                    callbacks::view_repo::handle(ctx, repo_id, from_page, &query_id).await?;
                }
                CallbackAction::ViewRepoLabels(repo_id, page, from_page) => {
                    callbacks::view_labels::handle(ctx, repo_id, page, from_page, &query_id)
                        .await?;
                }
                CallbackAction::RemoveRepoPrompt(repo_id) => {
                    callbacks::remove::handle(ctx, repo_id, 1).await?;
                }
                CallbackAction::ToggleLabel(label, label_page, _) => {
                    callbacks::toggle_label::handle(ctx, label, label_page).await?;
                }
                CallbackAction::BackToRepoList(page) => {
                    callbacks::list::handle(ctx, page).await?;
                }
                CallbackAction::ListReposPage(page) => {
                    callbacks::list::handle(ctx, page).await?;
                }
                CallbackAction::CmdHelp => commands::help::handle(ctx).await?,
                CallbackAction::CmdList => commands::list::handle(ctx, 1).await?,
                CallbackAction::CmdAdd => commands::add::handle(ctx).await?,
                CallbackAction::CmdOverview => commands::overview::handle(ctx).await?,
            };
        } else {
            tracing::warn!("Callback query has no data");
            if let Some(message) = &query.message {
                self.messaging_service
                    .send_error_msg(
                        message.chat().id,
                        BotHandlerError::InvalidInput("Invalid callback data".to_string()),
                    )
                    .await?;
            }
        }
        Ok(())
    }
}
