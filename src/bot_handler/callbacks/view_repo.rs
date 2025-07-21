use std::str::FromStr;

use crate::{
    bot_handler::{BotHandlerError, BotHandlerResult, Context, CommandState},
    storage::RepoEntity,
};

pub async fn handle(
    ctx: Context<'_>,
    repo_id: &str,
    from_page: usize,
) -> BotHandlerResult<()> {
    let chat_id = ctx.message.chat.id;

    // Extract repository name with owner
    let repo =
        RepoEntity::from_str(repo_id).map_err(|e| BotHandlerError::InvalidInput(e.to_string()))?;

    // Get all repo labels
    let repo_labels = ctx
        .handler
        .repository_service
        .get_repo_github_labels(chat_id, &repo, from_page)
        .await?
        .items
        .into_iter()
        .filter(|l| l.is_selected)
        .collect::<Vec<_>>();

    // Answer the callback query to clear the spinner.
    ctx.handler
        .messaging_service
        .answer_details_callback_query(chat_id, ctx.message.id, &repo, &repo_labels, from_page)
        .await?;

    // Reset the dialogue state
    ctx.dialogue.update(CommandState::None).await.map_err(BotHandlerError::DialogueError)?;

    Ok(())
}
