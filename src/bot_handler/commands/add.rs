use crate::bot_handler::{BotHandler, commands::CommandContext};
use crate::storage::Repository;
use anyhow::Result;
use teloxide::types::Message;

pub async fn handle(ctx: CommandContext<'_>, arg: &str) -> Result<()> {
    if arg.trim().is_empty() {
        ctx.handler
            .prompt_and_wait_for_reply(ctx.message.chat.id, &ctx.dialogue, "add")
            .await?;
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
    // Try to parse the repository.
    let repo = match repo_name_with_owner.parse::<Repository>() {
        Ok(repo) => repo,
        Err(e) => {
            // Send a message to the chat if parsing fails.
            handler
                .send_response(msg.chat.id, format!("Failed to parse repository: {}", e))
                .await?;
            return Ok(());
        }
    };

    // Check if the repository exists on GitHub.
    match handler
        .github_client
        .repo_exists(&repo.owner, &repo.name)
        .await
    {
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
                handler
                    .storage
                    .add_repository(msg.chat.id, repo.clone())
                    .await;
                handler
                    .send_response(msg.chat.id, format!("Added repo: {}", repo.to_string()))
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
    Ok(())
}
