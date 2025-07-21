use crate::bot_handler::{
    BotHandlerError, BotHandlerResult, CommandState, commands::CommandContext,
};

pub async fn handle(ctx: CommandContext<'_>, page: usize) -> BotHandlerResult<()> {
    let user_repos =
        ctx.handler.repository_service.get_user_repos(ctx.message.chat.id, page).await?;

    if user_repos.items.is_empty() {
        ctx.handler.messaging_service.send_list_empty_msg(ctx.message.chat.id).await?;
        return Ok(());
    }

    // Edit message if dialogue state is not None, otherwise send a new message.
    if let Some(dialogue_state) =
        ctx.dialogue.get().await.map_err(BotHandlerError::DialogueError)?
    {
        if dialogue_state != CommandState::None {
            ctx.handler
                .messaging_service
                .edit_list_msg(ctx.message.chat.id, ctx.message.id, user_repos)
                .await?;
            ctx.dialogue
                .update(CommandState::None)
                .await
                .map_err(BotHandlerError::DialogueError)?;
        } else {
            ctx.handler.messaging_service.send_list_msg(ctx.message.chat.id, user_repos).await?;
        }
    } else {
        ctx.handler.messaging_service.send_list_msg(ctx.message.chat.id, user_repos).await?;
    }

    Ok(())
}
