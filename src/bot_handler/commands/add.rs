use crate::bot_handler::{BotHandler, commands::CommandContext, utils};
use crate::storage::Repository;
use anyhow::Result;
use teloxide::types::Message;

pub async fn handle(ctx: CommandContext<'_>, arg: &str) -> Result<()> {
    if arg.trim().is_empty() {
        ctx.handler.prompt_and_set_state(ctx.message.chat.id, &ctx.dialogue, "add").await?;
    } else {
        process_add(ctx.handler, ctx.message, arg).await?;
    }
    Ok(())
}

async fn process_add(
    handler: &BotHandler,
    msg: &Message,
    repo_name_with_owner: &str,
) -> Result<()> {
    if let Some((owner, repo_name)) = utils::parse_repo_name(repo_name_with_owner) {
        let repo = Repository::from_full_name(repo_name_with_owner)?;
        let repo_url = repo.url.clone();

        // Check if the repository exists on GitHub.
        match handler.github_client.repo_exists(owner, repo_name).await {
            Ok(true) => {
                if handler.storage.contains(msg.chat.id, &repo).await {
                    handler
                        .send_response(
                            msg.chat.id,
                            format!(
                                "Repository {} is already in your list",
                                repo_name_with_owner
                            ),
                        )
                        .await?;
                } else {
                    handler.storage.add_repository(msg.chat.id, repo).await;
                    handler
                        .send_response(
                            msg.chat.id,
                            format!("Added repo: {} ({})", repo_name_with_owner, repo_url),
                        )
                        .await?;
                }
            }
            Ok(false) => {
                handler
                    .send_response(msg.chat.id, "Repository does not exist on GitHub.")
                    .await?;
            }
            Err(e) => {
                handler
                    .send_response(msg.chat.id, format!("Error checking repository: {}", e))
                    .await?;
            }
        }
    } else {
        handler
            .send_response(msg.chat.id, "Invalid repository format. Use owner/repo.")
            .await?;
    }
    Ok(())
}
