//! This module contains handlers for bot commands.

pub mod add;
pub mod help;
pub mod list;
pub mod overview;
pub mod start;

use async_trait::async_trait;

use crate::bot_handler::{BotHandlerResult, Context};

/// A trait for handling bot commands.
#[async_trait]
pub trait CommandHandler {
    /// Handles a command.
    async fn handle(self, ctx: Context<'_>) -> BotHandlerResult<()>;
}

#[async_trait]
impl CommandHandler for super::Command {
    async fn handle(self, ctx: Context<'_>) -> BotHandlerResult<()> {
        match self {
            super::Command::Help => help::handle(ctx).await,
            super::Command::List => list::handle(ctx, 1).await,
            super::Command::Add => add::handle(ctx).await,
            super::Command::Start => start::handle(ctx).await,
            super::Command::Overview => overview::handle(ctx).await,
        }
    }
}
