use crate::bot_handler::{commands::CommandContext, BotHandlerResult};

pub async fn handle(ctx: CommandContext<'_>) -> BotHandlerResult<()> {
    ctx.handler
        .messaging_service
        .send_help_msg(ctx.message.chat.id)
        .await?;

    Ok(())
}
