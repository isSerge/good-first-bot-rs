mod utils;

use std::collections::HashSet;

use async_trait::async_trait;
use lazy_static::lazy_static;
use mockall::automock;
use teloxide::{
    prelude::*,
    types::{ChatId, ForceReply, InlineKeyboardButton, InlineKeyboardMarkup, MessageId, ParseMode},
    utils::{command::BotCommands, html},
};
use thiserror::Error;
use url::Url;

use crate::{
    bot_handler::{BotHandlerError, CallbackAction, Command},
    github::issues::IssuesRepositoryIssuesNodes,
    repository::LabelNormalized,
    storage::RepoEntity,
};

#[derive(Debug, Error)]
pub enum MessagingError {
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
    async fn send_list_msg(&self, chat_id: ChatId, repos: Vec<RepoEntity>) -> Result<()>;

    /// Sends a callback query to the user when they click on a button.
    /// The `query_id` is the ID of the callback query, and the `text` is the
    /// text of the message to be sent.
    async fn answer_callback_query(&self, query_id: &str, text: &str) -> Result<()>;

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
    ) -> Result<()>;

    /// Sends a callback query with repository labels.
    /// This includes a list of labels with buttons to toggle them.
    async fn answer_labels_callback_query(
        &self,
        chat_id: ChatId,
        message_id: MessageId,
        labels: &[LabelNormalized],
        repo_name_with_owner: &str,
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
        repos: Vec<RepoEntity>,
    ) -> Result<()>;

    /// Edits the labels message on the user's message after a labels have been
    /// updated.
    async fn edit_labels_msg(
        &self,
        chat_id: ChatId,
        message_id: MessageId,
        labels: &[LabelNormalized],
        repo_name_with_owner: &str,
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
            .parse_mode(ParseMode::Html)
            .reply_markup(keyboard)
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
        let start_text =
            "👋 Welcome! Use buttons below to track repository issues (i.e. 'good first issue', 'bug', 'enhancement', etc.";
        self.send_response_with_keyboard(chat_id, start_text.to_string(), None).await
    }

    async fn send_list_empty_msg(&self, chat_id: ChatId) -> Result<()> {
        self.send_response_with_keyboard(chat_id, "No repositories tracked.".to_string(), None)
            .await
    }

    async fn send_list_msg(&self, chat_id: ChatId, repos: Vec<RepoEntity>) -> Result<()> {
        let keyboard = build_repo_list_keyboard(&repos);
        self.send_response_with_keyboard(
            chat_id,
            "🔍 Your tracked repositories:".to_string(),
            Some(keyboard),
        )
        .await
    }

    async fn answer_callback_query(&self, query_id: &str, text: &str) -> Result<()> {
        self.bot
            .answer_callback_query(query_id)
            .text(text)
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
    ) -> Result<()> {
        let keyboard = build_repo_item_keyboard(repo);
        let text = "📦 Repository details:".to_string();

        self.bot
            .edit_message_text(chat_id, message_id, text)
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
        labels: &[LabelNormalized],
        repo_name_with_owner: &str,
    ) -> Result<()> {
        let keyboard = build_repo_labels_keyboard(labels, repo_name_with_owner);
        let text = if labels.is_empty() {
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

    async fn edit_list_msg(
        &self,
        chat_id: ChatId,
        message_id: MessageId,
        repos: Vec<RepoEntity>,
    ) -> Result<()> {
        // Rebuild the inline keyboard (each row has a repo link and a remove button).
        let new_keyboard = build_repo_list_keyboard(&repos);

        // Edit the original message to update the inline keyboard.
        self.bot
            .edit_message_reply_markup(chat_id, message_id)
            .reply_markup(new_keyboard)
            .await
            .map(|_| ())
            .map_err(MessagingError::TeloxideRequest)
    }

    async fn edit_labels_msg(
        &self,
        chat_id: ChatId,
        message_id: MessageId,
        labels: &[LabelNormalized],
        repo_name_with_owner: &str,
    ) -> Result<()> {
        let keyboard = build_repo_labels_keyboard(labels, repo_name_with_owner);
        let text = if labels.is_empty() {
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
}

fn build_repo_list_keyboard(repos: &[RepoEntity]) -> InlineKeyboardMarkup {
    let buttons: Vec<Vec<InlineKeyboardButton>> = repos
        .iter()
        .map(|repo| {
            // define callback action
            let action =
                utils::serialize_action(&CallbackAction::ViewRepoDetails(&repo.name_with_owner));

            // Repository name with link
            vec![InlineKeyboardButton::callback(repo.name_with_owner.clone(), action)]
        })
        .collect();

    InlineKeyboardMarkup::new(buttons)
}

fn build_repo_item_keyboard(repo: &RepoEntity) -> InlineKeyboardMarkup {
    let id = &repo.name_with_owner;
    // actions
    let back_to_list = utils::serialize_action(&CallbackAction::BackToRepoList);
    let repo_labels = utils::serialize_action(&CallbackAction::ViewRepoLabels(id));
    let remove_repo = utils::serialize_action(&CallbackAction::RemoveRepoPrompt(id));

    // buttons
    let buttons = vec![
        vec![
            // Repository name with link
            InlineKeyboardButton::url(
                repo.name_with_owner.clone(),
                Url::parse(&repo.url()).expect("Failed to parse repository URL"),
            ),
        ],
        vec![
            // Back to list button
            InlineKeyboardButton::callback("🔙 List".to_string(), back_to_list),
            // Manage repo labels button
            InlineKeyboardButton::callback("⚙️ Labels".to_string(), repo_labels),
            // Remove repo action
            InlineKeyboardButton::callback("❌ Remove".to_string(), remove_repo),
        ],
    ];

    InlineKeyboardMarkup::new(buttons)
}

fn build_repo_labels_keyboard(
    labels: &[LabelNormalized],
    id: &str, // repo name with owner
) -> InlineKeyboardMarkup {
    let label_buttons = labels
        .iter()
        .map(|label| {
            // define callback action
            let toggle_action = utils::serialize_action(&CallbackAction::TL(id, &label.name));

            vec![InlineKeyboardButton::callback(
                format!(
                    "{} {} {}({})",
                    if label.is_selected { "✅ " } else { "" },
                    utils::github_color_to_emoji(&label.color),
                    label.name,
                    label.count,
                ),
                toggle_action,
            )]
        })
        .collect::<Vec<_>>();

    // Prepend the back button to the list of buttons
    let go_back = utils::serialize_action(&CallbackAction::BackToRepoDetails(id));
    let mut buttons = vec![vec![InlineKeyboardButton::callback("🔙 Back".to_string(), go_back)]];
    buttons.extend(label_buttons);

    InlineKeyboardMarkup::new(buttons)
}

lazy_static! {
    static ref COMMAND_KEYBOARD: InlineKeyboardMarkup = InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback(
            "ℹ️ Help",
            utils::serialize_action(&CallbackAction::Help)
        ),],
        vec![InlineKeyboardButton::callback(
            "📜 Tracked repositories",
            utils::serialize_action(&CallbackAction::List)
        ),],
        vec![InlineKeyboardButton::callback(
            "➕ Add repository",
            utils::serialize_action(&CallbackAction::Add)
        ),],
    ]);
}
