pub mod add;
pub mod help;
pub mod list;
pub mod overview;
pub mod start;
pub mod view_repo;
pub mod remove;
pub mod toggle_label;
pub mod view_labels;

use async_trait::async_trait;
use teloxide::prelude::*;

use crate::bot_handler::{BotHandler, BotHandlerResult, CommandState, DialogueStorage};

/// CommandContext groups the data needed by all command handlers.
pub struct CommandContext<'a> {
    pub handler: &'a BotHandler,
    pub message: &'a Message,
    pub dialogue: &'a Dialogue<CommandState, DialogueStorage>,
    pub query: Option<&'a CallbackQuery>,
}

#[async_trait]
pub trait CommandHandler {
    async fn handle(self, ctx: CommandContext<'_>) -> BotHandlerResult<()>;
}

// Simplified CommandHandler implementation
#[async_trait]
impl CommandHandler for super::Command {
    async fn handle(self, ctx: CommandContext<'_>) -> BotHandlerResult<()> {
        match self {
            super::Command::Help => help::handle(ctx).await,
            super::Command::List => list::handle(ctx, 1).await,
            super::Command::Add => add::handle(ctx).await,
            super::Command::Start => start::handle(ctx).await,
            super::Command::Overview => overview::handle(ctx).await,
        }
    }
}
