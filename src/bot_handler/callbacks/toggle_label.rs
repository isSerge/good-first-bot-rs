use std::str::FromStr;

use futures::{TryFutureExt, try_join};

use crate::{
    bot_handler::{BotHandlerError, BotHandlerResult, CommandState, Context},
    storage::RepoEntity,
};

pub async fn handle(ctx: Context<'_>, label_name: &str) -> BotHandlerResult<()> {
    let chat_id = ctx.message.chat.id;
    let query = ctx
        .query
        .ok_or_else(|| BotHandlerError::InvalidInput("Callback query is missing".to_string()))?;

    // Extract repository name with owner from the dialogue state
    let dialogue_state = ctx.dialogue.get().await.map_err(BotHandlerError::DialogueError)?;

    let (repo_id, from_page) = match dialogue_state {
        Some(CommandState::ViewingRepoLabels { repo_id, from_page }) => (repo_id, from_page),
        _ =>
            return Err(BotHandlerError::InvalidInput(
                "Invalid state: expected ViewingRepoLabels".to_string(),
            )),
    };

    let repo =
        RepoEntity::from_str(&repo_id).map_err(|e| BotHandlerError::InvalidInput(e.to_string()))?;

    // Try to toggle the label for the repository and handle potential limit errors.
    let is_selected =
        ctx.handler.repository_service.toggle_label(chat_id, &repo, label_name).await?;

    // Concurrently fetch updated labels and answer the callback query.
    let (labels, _) = try_join!(
        ctx.handler
            .repository_service
            .get_repo_github_labels(chat_id, &repo, from_page)
            .map_err(BotHandlerError::from),
        ctx.handler
            .messaging_service
            .answer_toggle_label_callback_query(&query.id, label_name, is_selected)
            .map_err(BotHandlerError::from)
    )?;

    // Edit labels message to show the updated labels.
    ctx.handler
        .messaging_service
        .edit_labels_msg(chat_id, ctx.message.id, &labels, &repo_id, from_page)
        .await?;

    Ok(())
}
