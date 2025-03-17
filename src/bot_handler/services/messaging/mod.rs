use crate::bot_handler::Command;
use crate::github::issues::IssuesRepositoryIssuesNodes;
use crate::storage::Repository;
use anyhow::{Error, Result};
use async_trait::async_trait;
use lazy_static::lazy_static;
use mockall::automock;
use std::collections::HashSet;
use teloxide::types::ChatId;
use teloxide::utils::command::BotCommands;
use teloxide::{
    prelude::*,
    types::{ForceReply, InlineKeyboardButton, InlineKeyboardMarkup, MessageId},
};
use url::Url;

/// Trait for sending messages to the user.
#[automock]
#[async_trait]
pub trait MessagingService: Send + Sync {
    /// Sends a text message to the provided chat with a keyboard. If no keyboard is provided, the default command keyboard is used.
    async fn send_response_with_keyboard(
        &self,
        chat_id: ChatId,
        text: String,
        keyboard: Option<InlineKeyboardMarkup>,
    ) -> Result<()>;

    /// Prompts the user for repository input.
    async fn prompt_for_repo_input(&self, chat_id: ChatId) -> Result<()>;

    /// Sends an error message to the provided chat.
    async fn send_error_msg(&self, chat_id: ChatId, error: Error) -> Result<()>;

    /// Sends a message to the user that the repository is already tracked.
    async fn send_already_tracked_msg(
        &self,
        chat_id: ChatId,
        repo_name_with_owner: String,
    ) -> Result<()>;

    /// Sends a message to the user that the repository has been added.
    async fn send_repo_added_msg(
        &self,
        chat_id: ChatId,
        repo_name_with_owner: String,
    ) -> Result<()>;

    /// Sends a message to the user that the repository does not exist on GitHub.
    async fn send_no_repo_exists_msg(
        &self,
        chat_id: ChatId,
        repo_name_with_owner: String,
    ) -> Result<()>;

    /// Sends a message to the user that the repository has been removed.
    async fn send_repo_removed_msg(
        &self,
        chat_id: ChatId,
        repo_name_with_owner: String,
    ) -> Result<()>;

    /// Sends a message to the user that the repository is not tracked.
    async fn send_repo_not_tracked_msg(
        &self,
        chat_id: ChatId,
        repo_name_with_owner: String,
    ) -> Result<()>;

    /// Sends a help message to the user.
    async fn send_help_msg(&self, chat_id: ChatId) -> Result<()>;

    /// Sends a start message to the user.
    async fn send_start_msg(&self, chat_id: ChatId) -> Result<()>;

    /// Sends a message to the user that the repo list is empty.
    async fn send_list_empty_msg(&self, chat_id: ChatId) -> Result<()>;

    /// Sends a message with repo list keyboard.
    async fn send_list_msg(&self, chat_id: ChatId, repos: HashSet<Repository>) -> Result<()>;

    /// Sends a callback query to the user.
    async fn answer_remove_callback_query(&self, query_id: String, removed: bool) -> Result<()>;

    /// Edits the list of repositories on the user's message after a repository has been removed.
    async fn edit_list_msg(
        &self,
        chat_id: ChatId,
        message_id: MessageId,
        repos: HashSet<Repository>,
    ) -> Result<()>;

    /// Sends a message to the user that there are new issues.
    async fn send_new_issues_msg(
        &self,
        chat_id: ChatId,
        repo_name_with_owner: &str,
        issues: Vec<IssuesRepositoryIssuesNodes>,
    ) -> Result<()>;
}

/// Telegram messaging service.
pub struct TelegramMessagingService {
    bot: Bot,
}

impl TelegramMessagingService {
    pub fn new(bot: Bot) -> Self {
        Self { bot }
    }
}

#[async_trait]
impl MessagingService for TelegramMessagingService {
    async fn send_response_with_keyboard(
        &self,
        chat_id: ChatId,
        text: String,
        keyboard: Option<InlineKeyboardMarkup>,
    ) -> Result<()> {
        // If no keyboard is provided, use the default command keyboard.
        let keyboard = keyboard.unwrap_or(COMMAND_KEYBOARD.clone());

        self.bot
            .send_message(chat_id, text)
            .reply_markup(keyboard)
            .await
            .map(|_| ())
            .map_err(|e| anyhow::anyhow!("Failed to send message: {}", e))
    }

    async fn prompt_for_repo_input(&self, chat_id: ChatId) -> Result<()> {
        let prompt = "Please reply with the repository url.";
        self.bot
            .send_message(chat_id, prompt)
            .reply_markup(ForceReply::new())
            .await
            .map(|_| ())
            .map_err(|e| anyhow::anyhow!("Failed to send message: {}", e))
    }

    async fn send_error_msg(&self, chat_id: ChatId, error: Error) -> Result<()> {
        self.send_response_with_keyboard(chat_id, error.to_string(), None)
            .await
    }

    async fn send_already_tracked_msg(
        &self,
        chat_id: ChatId,
        repo_name_with_owner: String,
    ) -> Result<()> {
        self.send_response_with_keyboard(
            chat_id,
            format!(
                "Repository {} is already in your list",
                repo_name_with_owner
            ),
            None,
        )
        .await
    }

    async fn send_repo_added_msg(
        &self,
        chat_id: ChatId,
        repo_name_with_owner: String,
    ) -> Result<()> {
        self.send_response_with_keyboard(
            chat_id,
            format!("Repository {} added to your list", repo_name_with_owner),
            None,
        )
        .await
    }

    async fn send_no_repo_exists_msg(
        &self,
        chat_id: ChatId,
        repo_name_with_owner: String,
    ) -> Result<()> {
        self.send_response_with_keyboard(
            chat_id,
            format!(
                "Repository {} does not exist on GitHub.",
                repo_name_with_owner
            ),
            None,
        )
        .await
    }

    async fn send_repo_removed_msg(
        &self,
        chat_id: ChatId,
        repo_name_with_owner: String,
    ) -> Result<()> {
        self.send_response_with_keyboard(
            chat_id,
            format!("Repository {} removed from your list", repo_name_with_owner),
            None,
        )
        .await
    }

    async fn send_repo_not_tracked_msg(
        &self,
        chat_id: ChatId,
        repo_name_with_owner: String,
    ) -> Result<()> {
        self.send_response_with_keyboard(
            chat_id,
            format!("Repository {} is not tracked", repo_name_with_owner),
            None,
        )
        .await
    }

    async fn send_help_msg(&self, chat_id: ChatId) -> Result<()> {
        let help_text = Command::descriptions();
        self.send_response_with_keyboard(
            chat_id,
            help_text.to_string(),
            Some(COMMAND_KEYBOARD.clone()),
        )
        .await
    }

    async fn send_start_msg(&self, chat_id: ChatId) -> Result<()> {
        let start_text =
            "Welcome! Use the buttons below to track repositories with good first issues.";
        self.send_response_with_keyboard(chat_id, start_text.to_string(), None)
            .await
    }

    async fn send_list_empty_msg(&self, chat_id: ChatId) -> Result<()> {
        self.send_response_with_keyboard(chat_id, "No repositories tracked.".to_string(), None)
            .await
    }

    async fn send_list_msg(&self, chat_id: ChatId, repos: HashSet<Repository>) -> Result<()> {
        let keyboard = build_repo_list_keyboard(&repos);
        self.send_response_with_keyboard(
            chat_id,
            "Your tracked repositories:".to_string(),
            Some(keyboard),
        )
        .await
    }

    async fn answer_remove_callback_query(&self, query_id: String, removed: bool) -> Result<()> {
        let removed_msg = if removed {
            "Repository removed successfully."
        } else {
            "Repository not found."
        };

        self.bot
            .answer_callback_query(query_id)
            .text(removed_msg)
            .await
            .map(|_| ())
            .map_err(|e| anyhow::anyhow!("Failed to answer callback query: {}", e))
    }

    async fn edit_list_msg(
        &self,
        chat_id: ChatId,
        message_id: MessageId,
        repos: HashSet<Repository>,
    ) -> Result<()> {
        // Rebuild the inline keyboard (each row has a repo link and a remove button).
        let new_keyboard = build_repo_list_keyboard(&repos);

        // Edit the original message to update the inline keyboard.
        self.bot
            .edit_message_reply_markup(chat_id, message_id)
            .reply_markup(new_keyboard)
            .await
            .map(|_| ())
            .map_err(|e| anyhow::anyhow!("Failed to edit message: {}", e))
    }

    async fn send_new_issues_msg(
        &self,
        chat_id: ChatId,
        repo_name_with_owner: &str,
        issues: Vec<IssuesRepositoryIssuesNodes>,
    ) -> Result<()> {
        let message = format!(
            "🚨 New issues in {}:\n\n{}",
            repo_name_with_owner,
            issues
                .iter()
                .map(|issue| format!("- {}: {}", issue.title, issue.url))
                .collect::<Vec<_>>()
                .join("\n")
        );

        self.bot
            .send_message(chat_id, message)
            .await
            .map(|_| ())
            .map_err(|e| anyhow::anyhow!("Failed to send message: {}", e))
    }
}

fn build_repo_list_keyboard(repos: &HashSet<Repository>) -> InlineKeyboardMarkup {
    let buttons: Vec<Vec<InlineKeyboardButton>> = repos
        .iter()
        .map(|repo| {
            vec![
                // Left button: repository name (with a no-op or details callback)
                InlineKeyboardButton::url(
                    repo.name_with_owner.clone(),
                    Url::parse(&repo.url()).expect("Failed to parse repository URL"),
                ),
                // Right button: remove action
                InlineKeyboardButton::callback(
                    "❌".to_string(),
                    format!("remove:{}", repo.name_with_owner),
                ),
            ]
        })
        .collect();

    InlineKeyboardMarkup::new(buttons)
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
