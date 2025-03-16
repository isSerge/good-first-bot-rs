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
    /// Sends a text message to the provided chat with a keyboard. If no keyboard is provided, the default command keyboard is used.
    async fn send_response_with_keyboard(
        &self,
        chat_id: ChatId,
        text: String,
        keyboard: Option<InlineKeyboardMarkup>,
    ) -> Result<()>;
    /// Prompts the user for repository input.
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
