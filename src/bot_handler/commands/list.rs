use crate::bot_handler::{BotHandlerResult, commands::Context};

pub async fn handle(ctx: Context<'_>, page: usize) -> BotHandlerResult<()> {
    let user_repos =
        ctx.handler.repository_service.get_user_repos(ctx.message.chat.id, page).await?;

    if user_repos.items.is_empty() {
        ctx.handler.messaging_service.send_list_empty_msg(ctx.message.chat.id).await?;
        return Ok(());
    }

    ctx.handler.messaging_service.send_list_msg(ctx.message.chat.id, user_repos).await?;

    Ok(())
}
