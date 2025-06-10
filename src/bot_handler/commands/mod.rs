mod add;
mod help;
mod list;
mod overview;
mod start;

use async_trait::async_trait;
use teloxide::{dispatching::dialogue::InMemStorage, prelude::*};

use crate::bot_handler::{BotHandler, BotHandlerResult, CommandState};

/// CommandContext groups the data needed by all command handlers.
pub struct CommandContext<'a> {
    pub handler: &'a BotHandler,
    pub message: &'a Message,
    pub dialogue: &'a Dialogue<CommandState, InMemStorage<CommandState>>,
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
            super::Command::List => list::handle(ctx).await,
            super::Command::Add => add::handle(ctx).await,
            super::Command::Start => start::handle(ctx).await,
            super::Command::Overview => overview::handle(ctx).await,
        }
    }
}
