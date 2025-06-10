use futures::future;

use crate::bot_handler::{BotHandlerResult, commands::CommandContext};

pub async fn handle(ctx: CommandContext<'_>) -> BotHandlerResult<()> {
    tracing::debug!("Handling overview command for chat: {}", ctx.message.chat.id);

    let user_repos = ctx.handler.repository_service.get_user_repos(ctx.message.chat.id, 1).await?;

    // Check if the user has any repositories
    if user_repos.items.is_empty() {
        ctx.handler.messaging_service.send_list_empty_msg(ctx.message.chat.id).await?;
        return Ok(());
    }

    // Fetch repos and labels
    let overview_futures = user_repos.items.iter().map(|r| async move {
        let repo_labels =
            ctx.handler.repository_service.get_user_repo_labels(ctx.message.chat.id, r).await;

        match repo_labels {
            Ok(labels) => (r.clone(), labels),
            Err(_) => {
                tracing::error!("Failed to fetch labels for repo: {}", r.name_with_owner);
                (r.clone(), Vec::new())
            }
        }
    });

    let overview = future::join_all(overview_futures).await;

    ctx.handler.messaging_service.send_overview_msg(ctx.message.chat.id, overview).await?;

    Ok(())
}
