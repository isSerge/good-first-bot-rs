use crate::bot_handler::{BotHandler, CommandState, commands::CommandContext};
use anyhow::Result;
use teloxide::types::Message;

pub async fn handle(ctx: CommandContext<'_>, arg: &str) -> Result<()> {
    if arg.trim().is_empty() {
        ctx.handler.prompt_for_repo(ctx.message.chat.id).await?;
        ctx.dialogue
            .update(CommandState::WaitingForRepo {
                command: "remove".into(),
            })
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
