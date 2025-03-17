use crate::bot_handler::commands::CommandContext;
use anyhow::Result;

pub async fn handle(ctx: CommandContext<'_>) -> Result<()> {
    let user_repos = ctx
        .handler
        .repository_service
        .get_user_repos(ctx.message.chat.id)
        .await?;

    if user_repos.is_empty() {
        return ctx
            .handler
            .messaging_service
            .send_list_empty_msg(ctx.message.chat.id)
            .await;
    }

    ctx.handler
        .messaging_service
        .send_list_msg(ctx.message.chat.id, user_repos)
        .await
}
