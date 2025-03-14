pub mod add;
mod help;
mod list;
pub mod remove;
mod start;

use crate::bot_handler::{BotHandler, CommandState};
use anyhow::Result;
use async_trait::async_trait;
use teloxide::dispatching::dialogue::InMemStorage;
use teloxide::prelude::*;

/// CommandContext groups the data needed by all command handlers.
pub struct CommandContext<'a> {
    pub handler: &'a BotHandler,
    pub message: &'a Message,
    pub dialogue: &'a Dialogue<CommandState, InMemStorage<CommandState>>,
}

#[async_trait]
pub trait CommandHandler {
    async fn handle(self, ctx: CommandContext<'_>) -> Result<()>;
}

// Simplified CommandHandler implementation
#[async_trait]
impl CommandHandler for super::Command {
    async fn handle(self, ctx: CommandContext<'_>) -> Result<()> {
        match self {
            super::Command::Help => help::handle(ctx).await,
            super::Command::List => list::handle(ctx).await,
            super::Command::Add(arg) => add::handle(ctx, &arg).await,
            super::Command::Remove(arg) => remove::handle(ctx, &arg).await,
            super::Command::Start => start::handle(ctx).await,
        }
    }
}
