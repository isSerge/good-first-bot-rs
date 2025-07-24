use std::str::FromStr;

use crate::{
    bot_handler::{BotHandlerError, BotHandlerResult, CommandState, Context},
    storage::RepoEntity,
};

pub async fn handle(
    ctx: Context<'_>,
    repo_id: &str,
    page: usize,
    from_page: usize,
    _query_id: &str,
) -> BotHandlerResult<()> {
    let chat_id = ctx.message.chat.id;

    // Extract repository name with owner
    let repo =
        RepoEntity::from_str(repo_id).map_err(|e| BotHandlerError::InvalidInput(e.to_string()))?;

    // Get paginated labels for the repository
    let paginated_labels =
        ctx.handler.repository_service.get_repo_github_labels(chat_id, &repo, page).await?;

    // Answer the callback query to clear the spinner.
    ctx.handler
        .messaging_service
        .answer_labels_callback_query(
            chat_id,
            ctx.message.id,
            &paginated_labels,
            repo_id,
            from_page,
        )
        .await?;

    // Update the dialogue state to ViewingRepoLabels
    ctx.dialogue
        .update(CommandState::ViewingRepoLabels { repo_id: repo.name_with_owner, from_page })
        .await
        .map_err(BotHandlerError::DialogueError)?;

    Ok(())
}
