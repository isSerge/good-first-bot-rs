use crate::bot_handler::{BotHandlerError, BotHandlerResult, CommandState, Context};

pub async fn handle(ctx: Context<'_>, page: usize) -> BotHandlerResult<()> {
    let user_repos =
        ctx.handler.repository_service.get_user_repos(ctx.message.chat.id, page).await?;

    if user_repos.items.is_empty() {
        ctx.handler.messaging_service.send_list_empty_msg(ctx.message.chat.id).await?;
        return Ok(());
    }

    ctx.handler
        .messaging_service
        .edit_list_msg(ctx.message.chat.id, ctx.message.id, user_repos)
        .await?;

    ctx.dialogue.update(CommandState::None).await.map_err(BotHandlerError::DialogueError)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use mockall::predicate::eq;

    use super::*;
    use crate::{
        bot_handler::{
            CallbackAction,
            test_helpers::{CHAT_ID, TestHarness},
        },
        messaging::MockMessagingService,
        pagination::Paginated,
        repository::MockRepositoryService,
        storage::RepoEntity,
    };

    #[tokio::test]
    async fn test_handle_callback_back_to_repo_list() {
        // Arrange
        let mut mock_messaging = MockMessagingService::new();
        let mut mock_repository = MockRepositoryService::new();
        let page = 2;

        let repos = vec![RepoEntity::from_str("owner/repo1").unwrap()];
        let paginated_repos = Paginated::new(repos.clone(), page);

        mock_repository
            .expect_get_user_repos()
            .with(eq(CHAT_ID), eq(page))
            .times(1)
            .returning(move |_, _| Ok(paginated_repos.clone()));

        mock_messaging.expect_answer_callback_query().times(1).returning(|_, _| Ok(()));

        mock_messaging
            .expect_edit_list_msg()
            .withf(move |&cid, _, p_repos| cid == CHAT_ID && p_repos.items == repos)
            .times(1)
            .returning(|_, _, _| Ok(()));

        let harness = TestHarness::new(mock_messaging, mock_repository).await;
        harness
            .dialogue
            .update(CommandState::ViewingRepoLabels {
                repo_id: "owner/repo".to_string(),
                from_page: 1,
            })
            .await
            .unwrap();
        let action = CallbackAction::BackToRepoList(page);

        // Act
        let result = harness.handle_callback(&action).await;

        // Assert
        assert!(result.is_ok());
        let state = harness.dialogue.get().await.unwrap();
        assert!(
            matches!(state, Some(CommandState::None)),
            "Dialogue state should be reset to None"
        );
    }
}
