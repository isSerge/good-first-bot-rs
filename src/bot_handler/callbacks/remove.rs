use crate::bot_handler::{BotHandlerError, BotHandlerResult, Context};

pub async fn handle(ctx: Context<'_>, repo_id: &str, from_page: usize) -> BotHandlerResult<()> {
    let chat_id = ctx.message.chat.id;
    let query = ctx
        .query
        .ok_or_else(|| BotHandlerError::InvalidInput("Callback query is missing".to_string()))?;

    // Attempt to remove the repository.
    let removed = ctx.handler.repository_service.remove_repo(chat_id, repo_id).await?;

    // Answer the callback query to clear the spinner.
    ctx.handler.messaging_service.answer_remove_callback_query(&query.id, removed).await?;

    // If removal was successful, update the inline keyboard on the original
    // message.
    if removed {
        // Get the updated repository list.
        let user_repos = ctx.handler.repository_service.get_user_repos(chat_id, from_page).await?;

        if user_repos.items.is_empty() {
            ctx.handler.messaging_service.send_list_empty_msg(chat_id).await?;
        }

        ctx.handler.messaging_service.edit_list_msg(chat_id, ctx.message.id, user_repos).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use mockall::predicate::eq;

    use super::*;
    use crate::{
        bot_handler::{
            CallbackAction,
            test_helpers::{CHAT_ID, TestHarness},
        },
        messaging::MockMessagingService,
        repository::{MockRepositoryService, RepositoryServiceError},
        storage::StorageError,
    };

    #[tokio::test]
    async fn test_handle_callback_remove_repo_error() {
        // Arrange
        let mut mock_messaging = MockMessagingService::new();
        let mut mock_repository = MockRepositoryService::new();
        let repo_id = "owner/repo";

        mock_repository.expect_remove_repo().with(eq(CHAT_ID), eq(repo_id)).times(1).returning(
            |_, _| {
                Err(RepositoryServiceError::StorageError(StorageError::DbError(
                    "DB is down".to_string(),
                )))
            },
        );

        mock_messaging.expect_answer_callback_query().times(1).returning(|_, _| Ok(()));

        let harness = TestHarness::new(mock_messaging, mock_repository).await;
        let action = CallbackAction::RemoveRepoPrompt(repo_id);

        // Act
        let result = harness.handle_callback(&action).await;

        // Assert
        assert!(matches!(result, Err(BotHandlerError::InternalError(_))));
    }
}
