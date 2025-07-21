use std::collections::HashSet;

use futures::{StreamExt, stream};

use crate::{
    bot_handler::{BotHandlerError, BotHandlerResult, CommandState, commands::CommandContext},
    storage::RepoEntity,
};

pub async fn handle(ctx: CommandContext<'_>) -> BotHandlerResult<()> {
    ctx.handler.messaging_service.prompt_for_repo_input(ctx.message.chat.id).await?;
    ctx.dialogue
        .update(CommandState::AwaitingAddRepo)
        .await
        .map_err(BotHandlerError::DialogueError)?;
    Ok(())
}

// An enum to represent the result of adding a repository.
enum AddRepoResult {
    Success(String),
    AlreadyTracked(String),
    NotFound(String),
    InvalidUrl(String),
    Error(String, String),
}

// A struct to hold the summary of the add operation.
#[derive(Default)]
struct AddSummary {
    successfully_added: HashSet<String>,
    already_tracked: HashSet<String>,
    not_found: HashSet<String>,
    invalid_urls: HashSet<String>,
    errors: HashSet<(String, String)>,
}

/// Handle the reply message when we're waiting for repository input.
/// It processes the input, checks each URL, and adds the repositories accordingly.
pub async fn handle_reply(
    ctx: CommandContext<'_>,
    text: &str,
) -> BotHandlerResult<()> {
    // Split the input by newlines or whitespaces and create owned Strings
    let urls: Vec<String> =
        text.split_whitespace().filter(|s| !s.is_empty()).map(String::from).collect();

    if urls.is_empty() {
        ctx.handler
            .messaging_service
            .send_error_msg(
                ctx.message.chat.id,
                BotHandlerError::InvalidInput("Invalid repository URL".to_string()),
            )
            .await?;
        return Ok(());
    }

    let summary = stream::iter(urls)
        .map(|url| async move {
            let repo = match RepoEntity::from_url(&url) {
                Ok(repo) => repo,
                Err(_) => return AddRepoResult::InvalidUrl(url),
            };

            match ctx.handler.repository_service.repo_exists(&repo.owner, &repo.name).await {
                Ok(true) => match ctx
                    .handler
                    .repository_service
                    .add_repo(ctx.message.chat.id, repo.clone())
                    .await
                {
                    Ok(true) => AddRepoResult::Success(repo.name_with_owner),
                    Ok(false) => AddRepoResult::AlreadyTracked(repo.name_with_owner),
                    Err(e) => AddRepoResult::Error(repo.name_with_owner, e.to_string()),
                },
                Ok(false) => AddRepoResult::NotFound(repo.name_with_owner),
                Err(e) => AddRepoResult::Error(repo.name_with_owner, e.to_string()),
            }
        })
        .buffer_unordered(ctx.handler.max_concurrency)
        .fold(AddSummary::default(), |mut summary, res| async move {
            match res {
                AddRepoResult::Success(name) => {
                    summary.successfully_added.insert(name);
                }
                AddRepoResult::AlreadyTracked(name) => {
                    summary.already_tracked.insert(name);
                }
                AddRepoResult::NotFound(name) => {
                    summary.not_found.insert(name);
                }
                AddRepoResult::InvalidUrl(url) => {
                    summary.invalid_urls.insert(url);
                }
                AddRepoResult::Error(name, e) => {
                    summary.errors.insert((name, e));
                }
            }
            summary
        })
        .await;

    ctx.handler
        .messaging_service
        .send_add_summary_msg(
            ctx.message.chat.id,
            summary.successfully_added,
            summary.already_tracked,
            summary.not_found,
            summary.invalid_urls,
            summary.errors,
        )
        .await?;

    Ok(())
}
