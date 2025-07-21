use crate::bot_handler::{BotHandlerError, BotHandlerResult, commands::CommandContext};

pub async fn handle(
    ctx: CommandContext<'_>,
    repo_id: &str,
    from_page: usize,
) -> BotHandlerResult<()> {
    let chat_id = ctx.message.chat.id;
    let query = ctx
        .query
        .ok_or_else(|| BotHandlerError::InvalidInput("Callback query is missing".to_string()))?;

    // Attempt to remove the repository.
    let removed = ctx.handler.repository_service.remove_repo(chat_id, repo_id).await?;

    // Answer the callback query to clear the spinner.
    ctx.handler.messaging_service.answer_remove_callback_query(&query.id, removed).await?;

    // If removal was successful, update the inline keyboard on the original
    // message.
    if removed {
        // Get the updated repository list.
        let user_repos = ctx.handler.repository_service.get_user_repos(chat_id, from_page).await?;

        if user_repos.items.is_empty() {
            ctx.handler.messaging_service.send_list_empty_msg(chat_id).await?;
        }

        ctx.handler.messaging_service.edit_list_msg(chat_id, ctx.message.id, user_repos).await?;
    }
    Ok(())
}
