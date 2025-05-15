use log::debug;

use crate::bot_handler::{BotHandlerResult, Command, commands::CommandContext};

pub async fn handle(ctx: CommandContext<'_>) -> BotHandlerResult<()> {
    debug!("Prompting for repository input");
    ctx.handler.prompt_and_wait_for_reply(ctx.message.chat.id, ctx.dialogue, Command::Remove).await
}
