use crate::bot_handler::commands::CommandContext;
use anyhow::Result;

pub async fn handle(ctx: CommandContext<'_>) -> Result<()> {
    ctx.handler
        .messaging_service
        .send_start_msg(ctx.message.chat.id)
        .await?;
    Ok(())
}
