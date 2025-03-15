use crate::bot_handler::{BotHandler, commands::CommandContext, utils};
use anyhow::Result;
use teloxide::types::Message;

pub async fn handle(ctx: CommandContext<'_>) -> Result<()> {
    process_list(ctx.handler, ctx.message).await?;
    Ok(())
}

async fn process_list(handler: &BotHandler, msg: &Message) -> Result<()> {
    let user_repos = handler.storage.get_repos_per_user(msg.chat.id).await;

    if user_repos.is_empty() {
        return handler
            .send_response(msg.chat.id, "No repositories tracked.")
            .await;
    }

    let repos_msg = utils::format_tracked_repos(&user_repos);

    handler
        .send_response(
            msg.chat.id,
            format!("Your tracked repositories:\n{}", repos_msg),
        )
        .await
}
