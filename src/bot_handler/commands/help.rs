use crate::bot_handler::{Command, commands::CommandContext};
use anyhow::Result;
use teloxide::utils::command::BotCommands;

pub async fn handle(ctx: CommandContext<'_>) -> Result<()> {
    let help_text = Command::descriptions();
    ctx.messaging_service
        .send_response_with_keyboard(ctx.message.chat.id, help_text.to_string())
        .await?;
    Ok(())
}
