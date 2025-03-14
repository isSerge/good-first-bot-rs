use crate::bot_handler::{BotHandler, commands::CommandContext};
use anyhow::Result;
use teloxide::types::Message;

pub async fn handle(ctx: CommandContext<'_>, arg: &str) -> Result<()> {
    if arg.trim().is_empty() {
        ctx.handler
            .prompt_and_wait_for_reply(ctx.message.chat.id, &ctx.dialogue, "remove")
            .await?;
    } else {
        process_remove(ctx.handler, ctx.message, arg).await?;
    }
    Ok(())
}

async fn process_remove(handler: &BotHandler, msg: &Message, repo: &str) -> Result<()> {
    if handler.storage.remove_repository(msg.chat.id, &repo).await {
        handler
            .send_response(msg.chat.id, format!("Removed repo: {}", repo))
            .await?;
    } else {
        handler
            .send_response(msg.chat.id, format!("You are not tracking repo: {}", repo))
            .await?;
    }
    Ok(())
}
