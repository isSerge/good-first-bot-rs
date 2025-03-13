pub mod add;
pub mod help;
pub mod list;
pub mod remove;

use crate::bot_handler::{BotHandler, CommandState};
use anyhow::Result;
use async_trait::async_trait;
use teloxide::dispatching::dialogue::InMemStorage;
use teloxide::prelude::*;

/// A common trait for command handlers.
#[async_trait]
pub trait CommandHandler {
    async fn handle(&self, ctx: CommandContext<'_>) -> Result<()>;
}

/// CommandContext groups the data needed by all command handlers.
pub struct CommandContext<'a> {
    pub handler: &'a BotHandler,
    pub message: &'a Message,
    pub dialogue: &'a Dialogue<CommandState, InMemStorage<CommandState>>,
    pub args: Option<String>,
}
