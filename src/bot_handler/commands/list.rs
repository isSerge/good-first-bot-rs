use crate::bot_handler::{commands::CommandContext, utils};
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
            .send_response_with_keyboard(
                ctx.message.chat.id,
                "No repositories tracked.".to_string(),
            )
            .await;
    }

    let repos_msg = utils::format_tracked_repos(&user_repos);

    ctx.handler
        .messaging_service
        .send_response_with_keyboard(
            ctx.message.chat.id,
            format!("Your tracked repositories:\n{}", repos_msg),
        )
        .await
}
