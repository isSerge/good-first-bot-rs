use crate::bot_handler::{
    Command,
    commands::{CommandContext, CommandHandler},
};
use anyhow::Result;
use async_trait::async_trait;
use teloxide::utils::command::BotCommands;

pub struct HelpCommand;

#[async_trait]
impl CommandHandler for HelpCommand {
    async fn handle(&self, ctx: CommandContext<'_>) -> Result<()> {
        let help_text = Command::descriptions();
        ctx.handler
            .send_response(ctx.message.chat.id, help_text)
            .await?;
        ctx.dialogue.exit().await?;
        Ok(())
    }
}
