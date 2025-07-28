use std::str::FromStr;

use crate::{
    bot_handler::{BotHandlerError, BotHandlerResult, CommandState, Context},
    storage::RepoEntity,
};

pub async fn handle(
    ctx: Context<'_>,
    repo_id: &str,
    from_page: usize,
    _query_id: &str,
) -> BotHandlerResult<()> {
    let chat_id = ctx.message.chat.id;

    // Extract repository name with owner
    let repo =
        RepoEntity::from_str(repo_id).map_err(|e| BotHandlerError::InvalidInput(e.to_string()))?;

    // Get all repo labels
    let repo_labels = ctx
        .handler
        .repository_service
        .get_repo_github_labels(chat_id, &repo, 1)
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
        pagination::Paginated,
        repository::MockRepositoryService,
    };

    #[tokio::test]
    async fn test_handle_callback_view_repo_details() {
        // Arrange
        let mut mock_messaging = MockMessagingService::new();
        let mut mock_repository = MockRepositoryService::new();
        let repo_id = "owner/repo";
        let from_page = 1;
        let repo_entity = RepoEntity::from_str(repo_id).unwrap();

        mock_repository
            .expect_get_repo_github_labels()
            .with(eq(CHAT_ID), eq(repo_entity.clone()), eq(1))
            .times(1)
            .returning(|_, _, _| Ok(Paginated::new(vec![], 1)));

        mock_messaging.expect_answer_callback_query().times(1).returning(|_, _| Ok(()));

        mock_messaging
            .expect_answer_details_callback_query()
            .withf(move |&cid, _, repo, labels, page| {
                cid == CHAT_ID
                    && repo.name_with_owner == repo_id
                    && labels.is_empty()
                    && *page == from_page
            })
            .times(1)
            .returning(|_, _, _, _, _| Ok(()));

        let harness = TestHarness::new(mock_messaging, mock_repository).await;
        let action = CallbackAction::ViewRepoDetails(repo_id, from_page);

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

    #[tokio::test]
    async fn test_handle_callback_back_to_repo_details() {
        // Arrange
        let mut mock_messaging = MockMessagingService::new();
        let mut mock_repository = MockRepositoryService::new();
        let repo_id = "owner/repo";
        let from_page = 1;

        mock_repository
            .expect_get_repo_github_labels()
            .with(eq(CHAT_ID), eq(RepoEntity::from_str(repo_id).unwrap()), eq(1))
            .times(1)
            .returning(|_, _, _| Ok(Paginated::new(vec![], 1)));

        mock_messaging.expect_answer_callback_query().times(1).returning(|_, _| Ok(()));

        mock_messaging
            .expect_answer_details_callback_query()
            .withf(move |&cid, _, repo, labels, page| {
                cid == CHAT_ID
                    && repo.name_with_owner == repo_id
                    && labels.is_empty()
                    && *page == from_page
            })
            .times(1)
            .returning(|_, _, _, _, _| Ok(()));

        let harness = TestHarness::new(mock_messaging, mock_repository).await;
        let action = CallbackAction::BackToRepoDetails(repo_id, from_page);

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
