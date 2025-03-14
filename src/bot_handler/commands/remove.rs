use crate::bot_handler::{BotHandler, commands::CommandContext};
use crate::storage::Repository;
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

async fn process_remove(
    handler: &BotHandler,
    msg: &Message,
    repo_name_with_owner: &str,
) -> Result<()> {
    // Try to parse the repository.
    let repo = match repo_name_with_owner.parse::<Repository>() {
        Ok(repo) => repo,
        Err(e) => {
            // Send a message to the chat if parsing fails.
            handler
                .send_response(msg.chat.id, format!("Failed to parse repository: {}", e))
                .await?;
            return Ok(());
        }
    };
    if handler
        .storage
        .remove_repository(msg.chat.id, &repo.name)
        .await
    {
        handler
            .send_response(msg.chat.id, format!("Removed repo: {}", repo.name))
            .await?;
    } else {
        handler
            .send_response(
                msg.chat.id,
                format!("You are not tracking repo: {}", repo.name),
            )
            .await?;
    }
    Ok(())
}
