use crate::bot_handler::{BotHandlerResult, commands::Context};

pub async fn handle(ctx: Context<'_>) -> BotHandlerResult<()> {
    ctx.handler.messaging_service.send_help_msg(ctx.message.chat.id).await?;

    Ok(())
}
