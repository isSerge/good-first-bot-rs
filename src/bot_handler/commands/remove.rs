use crate::bot_handler::{
    BotHandler, CommandState,
    commands::{CommandContext, CommandHandler},
};
use anyhow::Result;
use async_trait::async_trait;
use teloxide::types::Message;

pub struct RemoveCommand;

#[async_trait]
impl CommandHandler for RemoveCommand {
    async fn handle(&self, ctx: CommandContext<'_>) -> Result<()> {
        if ctx.args.as_ref().map_or(true, |s| s.trim().is_empty()) {
            ctx.handler.prompt_for_repo(ctx.message.chat.id).await?;
            ctx.dialogue
                .update(CommandState::WaitingForRepo {
                    command: "remove".into(),
                })
                .await?;
        } else if let Some(repo) = ctx.args {
            process_remove(&ctx.handler, &ctx.message, repo).await?;
        }
        Ok(())
    }
}

async fn process_remove(handler: &BotHandler, msg: &Message, repo: String) -> Result<()> {
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
