use crate::bot_handler::{
    BotHandlerError, BotHandlerResult, CommandState, commands::CommandContext,
};

pub async fn handle(ctx: CommandContext<'_>) -> BotHandlerResult<()> {
    ctx.handler.messaging_service.prompt_for_repo_input(ctx.message.chat.id).await?;
    ctx.dialogue
        .update(CommandState::AwaitingAddRepo)
        .await
        .map_err(BotHandlerError::DialogueError)?;
    Ok(())
}
