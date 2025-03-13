use crate::bot_handler::{
    BotHandler,
    commands::{CommandContext, CommandHandler},
    utils,
};
use anyhow::Result;
use async_trait::async_trait;
use teloxide::{prelude::*, types::Message};

pub struct ListCommand;

#[async_trait]
impl CommandHandler for ListCommand {
    async fn handle(&self, ctx: CommandContext<'_>) -> Result<()> {
        handle_list_command(ctx.handler, ctx.message).await?;
        Ok(())
    }
}

async fn handle_list_command(handler: &BotHandler, msg: &Message) -> ResponseResult<()> {
    let user_repos = handler.storage.get_repositories(msg.chat.id).await;

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
