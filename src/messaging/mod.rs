mod keyboards;
#[cfg(test)]
mod tests;
mod utils;

use std::collections::HashSet;

use async_trait::async_trait;
use keyboards::{
    COMMAND_KEYBOARD, build_repo_item_keyboard, build_repo_labels_keyboard,
    build_repo_list_keyboard,
};
use mockall::automock;
use teloxide::{
    prelude::*,
    sugar::request::RequestLinkPreviewExt,
    types::{ChatId, ForceReply, InlineKeyboardMarkup, MessageId, ParseMode},
    utils::{command::BotCommands, html},
};
use thiserror::Error;

use crate::{
    bot_handler::{BotHandlerError, Command},
    github::issues::IssuesRepositoryIssuesNodes,
    pagination::Paginated,
    repository::LabelNormalized,
    storage::RepoEntity,
};

/// Represents errors that can occur when sending messages.
#[derive(Debug, Error)]
pub enum MessagingError {
    /// An error from the underlying `teloxide` library.
    #[error("Teloxide API request failed: {0}")]
    TeloxideRequest(#[from] teloxide::RequestError),
}

type Result<T> = std::result::Result<T, MessagingError>;

/// Trait for sending messages to the user.
#[automock]
#[async_trait]
pub trait MessagingService: Send + Sync {
    /// Sends a text message to the provided chat with a keyboard. If no
    /// keyboard is provided, the default command keyboard is used.
    async fn send_response_with_keyboard(
        &self,
        chat_id: ChatId,
        text: String,
        keyboard: Option<InlineKeyboardMarkup>,
    ) -> Result<()>;

    /// Prompts the user for repository input.
    async fn prompt_for_repo_input(&self, chat_id: ChatId) -> Result<()>;

    /// Sends an error message to the provided chat.
    async fn send_error_msg(&self, chat_id: ChatId, error: BotHandlerError) -> Result<()>;

    /// Sends a help message to the user.
    async fn send_help_msg(&self, chat_id: ChatId) -> Result<()>;

    /// Sends a start message to the user.
    async fn send_start_msg(&self, chat_id: ChatId) -> Result<()>;

    /// Sends a message to the user that the repo list is empty.
    async fn send_list_empty_msg(&self, chat_id: ChatId) -> Result<()>;

    /// Sends a message with repo list keyboard.
    async fn send_list_msg(
        &self,
        chat_id: ChatId,
        paginated_repos: Paginated<RepoEntity>,
    ) -> Result<()>;

    /// Sends a callback query to the user when they click on a button.
    /// The `query_id` is the ID of the callback query, and the `text` is the
    /// text of the message to be sent.
    async fn answer_callback_query(&self, query_id: &str, text: &Option<String>) -> Result<()>;

    /// Sends a callback query to the user.
    async fn answer_remove_callback_query(&self, query_id: &str, removed: bool) -> Result<()>;

    /// Sends a callback query with repository details.
    /// This includes a link to the repository, button for managing labels and
    /// remove button. The callback query is sent to the user when they
    /// click on a repository in the list.
    async fn answer_details_callback_query(
        &self,
        chat_id: ChatId,
        message_id: MessageId,
        repo: &RepoEntity,
        labels: &[LabelNormalized],
        from_page: usize,
    ) -> Result<()>;

    /// Sends a callback query with repository labels.
    /// This includes a list of labels with buttons to toggle them.
    async fn answer_labels_callback_query(
        &self,
        chat_id: ChatId,
        message_id: MessageId,
        paginated_labels: &Paginated<LabelNormalized>,
        repo_name_with_owner: &str,
        from_page: usize,
    ) -> Result<()>;

    /// Sends a callback query to toggle the label.
    async fn answer_toggle_label_callback_query(
        &self,
        query_id: &str,
        label_name: &str,
        is_selected: bool,
    ) -> Result<()>;

    /// Edits the list of repositories on the user's message after a repository
    /// has been removed.
    async fn edit_list_msg(
        &self,
        chat_id: ChatId,
        message_id: MessageId,
        paginated_repos: Paginated<RepoEntity>,
    ) -> Result<()>;

    /// Edits the labels message on the user's message after a labels have been
    /// updated.
    async fn edit_labels_msg(
        &self,
        chat_id: ChatId,
        message_id: MessageId,
        paginated_labels: &Paginated<LabelNormalized>,
        repo_name_with_owner: &str,
        from_page: usize,
    ) -> Result<()>;

    /// Sends a message to the user that there are new issues.
    async fn send_new_issues_msg(
        &self,
        chat_id: ChatId,
        repo_name_with_owner: &str,
        issues: Vec<IssuesRepositoryIssuesNodes>,
    ) -> Result<()>;

    /// Sends a summary message after adding repositories.
    /// This message includes the number of successfully added, already
    /// tracked, not found, invalid URLs, and errors.
    async fn send_add_summary_msg(
        &self,
        chat_id: ChatId,
        successfully_added: HashSet<String>,
        already_tracked: HashSet<String>,
        not_found: HashSet<String>,
        invalid_urls: HashSet<String>,
        errors: HashSet<(String, String)>,
    ) -> Result<()>;

    /// Sends an overview message with tracked repositories and their labels.
    /// The overview is a vector of tuples, where each tuple contains a
    /// `RepoEntity` and a vector of label names.
    async fn send_overview_msg(
        &self,
        chat_id: ChatId,
        overview: Vec<(RepoEntity, Vec<String>)>,
    ) -> Result<()>;
}

/// The default implementation of the `MessagingService` trait.
pub struct TelegramMessagingService {
    bot: Bot,
}

impl TelegramMessagingService {
    /// Creates a new `TelegramMessagingService`.
    pub fn new(bot: Bot) -> Self {
        Self { bot }
    }

    // Helper to format text for paginated messages
    fn format_paginated_message_text(
        title: &str,
        paginated_data: &Paginated<impl Sized>,
        item_name_plural: &str,
    ) -> String {
        if paginated_data.total_items == 0 {
            return format!("{title}\n\nNo {item_name_plural} found.");
        }
        format!(
            "{} (Page {} of {})\nTotal {}: {}",
            title,
            paginated_data.page,
            paginated_data.total_pages,
            item_name_plural,
            paginated_data.total_items
        )
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
            .parse_mode(ParseMode::Html)
            .reply_markup(keyboard)
            .disable_link_preview(true)
            .await
            .map(|_| ())
            .map_err(MessagingError::TeloxideRequest)
    }

    async fn prompt_for_repo_input(&self, chat_id: ChatId) -> Result<()> {
        let prompt = "Please reply with repository URLs separated by spaces or new lines.";
        self.bot
            .send_message(chat_id, prompt)
            .reply_markup(ForceReply::new())
            .await
            .map(|_| ())
            .map_err(MessagingError::TeloxideRequest)
    }

    async fn send_error_msg(&self, chat_id: ChatId, error: BotHandlerError) -> Result<()> {
        self.send_response_with_keyboard(chat_id, html::escape(&error.to_string()), None).await
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
        let start_text = "👋 Welcome! Use buttons below to track repository issues (i.e. 'good \
                          first issue', 'bug', 'enhancement', etc.)";
        self.send_response_with_keyboard(chat_id, start_text.to_string(), None).await
    }

    async fn send_list_empty_msg(&self, chat_id: ChatId) -> Result<()> {
        self.send_response_with_keyboard(
            chat_id,
            "Currently no repositories tracked".to_string(),
            None,
        )
        .await
    }

    async fn send_list_msg(
        &self,
        chat_id: ChatId,
        paginated_repos: Paginated<RepoEntity>,
    ) -> Result<()> {
        let keyboard = build_repo_list_keyboard(&paginated_repos);
        let text = Self::format_paginated_message_text(
            "🔍 Your tracked repositories:",
            &paginated_repos,
            "repositories",
        );
        self.send_response_with_keyboard(chat_id, text, Some(keyboard)).await
    }

    async fn answer_callback_query(&self, query_id: &str, text: &Option<String>) -> Result<()> {
        match text {
            Some(text) => self.bot.answer_callback_query(query_id).text(text),
            None => self.bot.answer_callback_query(query_id),
        }
        .await
        .map(|_| ())
        .map_err(MessagingError::TeloxideRequest)
    }

    async fn answer_remove_callback_query(&self, query_id: &str, removed: bool) -> Result<()> {
        let removed_msg = if removed {
            "✅ Repository removed successfully."
        } else {
            "❌ Repository not found."
        };

        self.bot
            .answer_callback_query(query_id)
            .text(removed_msg)
            .await
            .map(|_| ())
            .map_err(MessagingError::TeloxideRequest)
    }

    async fn answer_details_callback_query(
        &self,
        chat_id: ChatId,
        message_id: MessageId,
        repo: &RepoEntity,
        labels: &[LabelNormalized],
        from_page: usize,
    ) -> Result<()> {
        let repo_link = html::link(&repo.url(), &html::escape(&repo.name_with_owner));
        let keyboard = build_repo_item_keyboard(repo, from_page);

        let mut message_parts = vec![
            format!("📦 Repository: {}", repo_link),
            "".to_string(), // Empty line for spacing
        ];

        if labels.is_empty() {
            message_parts.push("⚠️ No labels are being tracked in this repository.".to_string());
        } else {
            message_parts.push("🏷️ Tracked labels:".to_string());
            for label in labels {
                message_parts.push(format!(
                    "- {} {}",
                    utils::github_color_to_emoji(&label.color),
                    html::escape(&label.name)
                ));
            }
        }

        message_parts.push("".to_string()); // Empty line for spacing

        let text = message_parts.join("\n");

        self.bot
            .edit_message_text(chat_id, message_id, text)
            .parse_mode(ParseMode::Html)
            .reply_markup(keyboard)
            .await
            .map(|_| ())
            .map_err(MessagingError::TeloxideRequest)
    }

    async fn answer_toggle_label_callback_query(
        &self,
        query_id: &str,
        label_name: &str,
        is_selected: bool,
    ) -> Result<()> {
        let text = if is_selected {
            format!("✅ Label {label_name} has been added.")
        } else {
            format!("❌ Label {label_name} has been removed.")
        };

        self.bot
            .answer_callback_query(query_id)
            .text(text)
            .await
            .map(|_| ())
            .map_err(MessagingError::TeloxideRequest)
    }

    async fn answer_labels_callback_query(
        &self,
        chat_id: ChatId,
        message_id: MessageId,
        paginated_labels: &Paginated<LabelNormalized>,
        repo_name_with_owner: &str,
        from_page: usize,
    ) -> Result<()> {
        let keyboard =
            build_repo_labels_keyboard(paginated_labels, repo_name_with_owner, from_page);
        let title = format!("🏷️ Manage labels for {}:", html::escape(repo_name_with_owner));
        let text_to_send = Self::format_paginated_message_text(&title, paginated_labels, "labels");

        self.bot
            .edit_message_text(chat_id, message_id, text_to_send)
            .parse_mode(ParseMode::Html)
            .reply_markup(keyboard)
            .await
            .map(|_| ())
            .map_err(MessagingError::TeloxideRequest)
    }

    async fn edit_list_msg(
        &self,
        chat_id: ChatId,
        message_id: MessageId,
        paginated_repos: Paginated<RepoEntity>,
    ) -> Result<()> {
        let new_keyboard = build_repo_list_keyboard(&paginated_repos);
        let text = Self::format_paginated_message_text(
            "🔍 Your tracked repositories:",
            &paginated_repos,
            "repositories",
        );

        self.bot
            .edit_message_text(chat_id, message_id, text)
            .parse_mode(ParseMode::Html)
            .reply_markup(new_keyboard)
            .await
            .map(|_| ())
            .map_err(MessagingError::TeloxideRequest)
    }

    async fn edit_labels_msg(
        &self,
        chat_id: ChatId,
        message_id: MessageId,
        paginated_labels: &Paginated<LabelNormalized>,
        repo_name_with_owner: &str,
        from_page: usize,
    ) -> Result<()> {
        let keyboard =
            build_repo_labels_keyboard(paginated_labels, repo_name_with_owner, from_page);
        let text = if paginated_labels.items.is_empty() {
            "⚠️ No labels available for this repository."
        } else {
            "🏷️ Manage repository labels:"
        };

        self.bot
            .edit_message_text(chat_id, message_id, text)
            .reply_markup(keyboard)
            .await
            .map(|_| ())
            .map_err(MessagingError::TeloxideRequest)
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
            .map_err(MessagingError::TeloxideRequest)
    }

    async fn send_add_summary_msg(
        &self,
        chat_id: ChatId,
        successfully_added: HashSet<String>,
        already_tracked: HashSet<String>,
        not_found: HashSet<String>,
        invalid_urls: HashSet<String>,
        errors: HashSet<(String, String)>,
    ) -> Result<()> {
        let mut summary = Vec::new();
        summary.push("<b>Summary of repository addition:</b>".to_string());

        let format_summary_category = |title: &str, items: &HashSet<String>| {
            if !items.is_empty() {
                Some(format!(
                    "<b>{}:</b>\n{}",
                    html::escape(title),
                    items
                        .iter()
                        .map(|item| format!("- {}", html::escape(item)))
                        .collect::<Vec<_>>()
                        .join("\n")
                ))
            } else {
                None
            }
        };

        if let Some(success) = format_summary_category("✅ Successfully Added", &successfully_added)
        {
            summary.push(success);
        }

        if let Some(already) = format_summary_category("➡️ Already Tracked", &already_tracked) {
            summary.push(already);
        }

        if let Some(not_found) = format_summary_category("❓ Not Found on GitHub", &not_found) {
            summary.push(not_found);
        }

        if let Some(invalid_urls) = format_summary_category("⚠️ Invalid URL", &invalid_urls) {
            summary.push(invalid_urls);
        }

        if !errors.is_empty() {
            let error_messages = errors
                .iter()
                .map(|(repo, error)| format!("- {}: {}", html::escape(repo), html::escape(error)))
                .collect::<Vec<_>>()
                .join("\n");
            summary.push(format!("❌ <b>Errors:</b>\n{error_messages}"));
        }

        // Only the main title
        if summary.len() == 1 {
            summary.push("No valid URLs were processed, or all inputs were empty.".to_string());
        }

        self.send_response_with_keyboard(chat_id, summary.join("\n\n"), None).await
    }

    async fn send_overview_msg(
        &self,
        chat_id: ChatId,
        overview: Vec<(RepoEntity, Vec<String>)>,
    ) -> Result<()> {
        tracing::debug!(
            "Sending overview message to chat {chat_id} with {} repositories.",
            overview.len()
        );
        // It should not be empty since we check in command handler, log warning just in
        // case.
        if overview.is_empty() {
            tracing::warn!("No repositories found for overview.");
        }

        let mut message_parts =
            vec!["📊 Overview of your tracked repositories and labels:".to_string()];
        message_parts.push("".to_string()); // Empty line for spacing

        for (repo, labels) in overview {
            let repo_link = html::link(&repo.url(), &html::escape(&repo.name_with_owner));
            message_parts.push(format!("📦 <b>Repository:</b> {repo_link}"));

            if labels.is_empty() {
                message_parts
                    .push("⚠️ No labels are being tracked in this repository.".to_string());
            } else {
                message_parts.push("🏷️ <b>Tracked labels:</b>".to_string());
                for label in labels {
                    message_parts.push(format!("- {}", html::escape(&label)));
                }
            }
            message_parts.push("".to_string()); // Empty line for spacing
        }

        let text = message_parts.join("\n");
        self.send_response_with_keyboard(chat_id, text, None).await
    }
}
