use anyhow::Result;

use crate::bot_handler::commands::CommandContext;

pub async fn handle(ctx: CommandContext<'_>) -> Result<()> {
    ctx.handler.messaging_service.send_help_msg(ctx.message.chat.id).await?;
    Ok(())
}
