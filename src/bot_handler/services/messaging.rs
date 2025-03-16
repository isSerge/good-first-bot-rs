use anyhow::Result;
use async_trait::async_trait;
use lazy_static::lazy_static;
use teloxide::types::ChatId;
use teloxide::{
    prelude::*,
    types::{ForceReply, InlineKeyboardButton, InlineKeyboardMarkup},
};

/// Trait for sending messages to the user.
#[async_trait]
pub trait MessagingService: Send + Sync {
    async fn send_response_with_keyboard(&self, chat_id: ChatId, text: String) -> Result<()>;
    async fn prompt_for_repo_input(&self, chat_id: ChatId) -> Result<()>;
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
    /// Sends a text message to the provided chat with a keyboard.
    async fn send_response_with_keyboard(&self, chat_id: ChatId, text: String) -> Result<()> {
        self.bot
            .send_message(chat_id, text)
            .reply_markup(COMMAND_KEYBOARD.clone())
            .await
            .map(|_| ())
            .map_err(|e| anyhow::anyhow!("Failed to send message: {}", e))
    }

    /// Prompts the user for repository input.
    async fn prompt_for_repo_input(&self, chat_id: ChatId) -> Result<()> {
        let prompt = "Please reply with the repository url.";
        self.bot
            .send_message(chat_id, prompt)
            .reply_markup(ForceReply::new())
            .await
            .map(|_| ())
            .map_err(|e| anyhow::anyhow!("Failed to send message: {}", e))
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
