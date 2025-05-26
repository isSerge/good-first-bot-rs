use crate::bot_handler::{BotHandlerResult, Command, commands::CommandContext};

pub async fn handle(ctx: CommandContext<'_>) -> BotHandlerResult<()> {
    ctx.handler.prompt_and_wait_for_reply(ctx.message.chat.id, ctx.dialogue, Command::Add).await
}
