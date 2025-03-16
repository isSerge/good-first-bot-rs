mod add;
mod help;
mod list;
mod remove;
mod start;

use crate::bot_handler::messaging::MessagingService;
use crate::bot_handler::{BotHandler, CommandState};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use teloxide::dispatching::dialogue::InMemStorage;
use teloxide::prelude::*;

/// CommandContext groups the data needed by all command handlers.
pub struct CommandContext<'a> {
    pub handler: &'a BotHandler,
    pub message: &'a Message,
    pub dialogue: &'a Dialogue<CommandState, InMemStorage<CommandState>>,
    pub messaging_service: Arc<dyn MessagingService>,
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
            super::Command::Add => add::handle(ctx).await,
            super::Command::Remove => remove::handle(ctx).await,
            super::Command::Start => start::handle(ctx).await,
        }
    }
}
