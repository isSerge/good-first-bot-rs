use crate::bot_handler::{Command, commands::CommandContext};
use anyhow::Result;
use teloxide::utils::command::BotCommands;

pub async fn handle(ctx: CommandContext<'_>) -> Result<()> {
    let help_text = Command::descriptions();
    ctx.handler
        .send_response(ctx.message.chat.id, help_text)
        .await?;
    Ok(())
}
