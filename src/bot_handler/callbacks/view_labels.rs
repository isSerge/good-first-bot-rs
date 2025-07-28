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

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use mockall::predicate::eq;

    use super::*;
    use crate::{
        bot_handler::{
            CallbackAction,
            test_helpers::{CHAT_ID, TestHarness},
        },
        messaging::MockMessagingService,
        pagination::Paginated,
        repository::{LabelNormalized, MockRepositoryService, RepositoryServiceError},
        storage::StorageError,
    };

    #[tokio::test]
    async fn test_dialogue_persists_viewing_repo_labels_state() {
        // Arrange
        let mut mock_messaging = MockMessagingService::new();
        let mut mock_repository = MockRepositoryService::new();
        let repo_id = "owner/repo";
        let from_page = 1;
        let labels_page = 1;
        let label_to_toggle = "bug";
        let repo_entity = RepoEntity::from_str(repo_id).unwrap();

        // --- Expectations for the test flow ---

        // `handle_callback_query` always answers the query immediately.
        mock_messaging.expect_answer_callback_query().times(2).returning(|_, _| Ok(()));

        // `get_repo_github_labels` is called three times in total.
        // 1. Once in `action_view_labels`.
        // 2. Twice in `action_toggle_label` (the bug we are testing around).
        let initial_labels = Paginated::new(vec![], 1);
        let updated_labels = Paginated::new(
            vec![LabelNormalized {
                name: "bug".to_string(),
                color: "d73a4a".to_string(),
                count: 1,
                is_selected: true,
            }],
            1,
        );
        let call_count = RefCell::new(0);
        mock_repository
            .expect_get_repo_github_labels()
            .with(eq(CHAT_ID), eq(repo_entity.clone()), eq(labels_page))
            .times(2)
            .returning(move |_, _, _| {
                *call_count.borrow_mut() += 1;
                match *call_count.borrow() {
                    1 => Ok(initial_labels.clone()),
                    _ => Ok(updated_labels.clone()),
                }
            });

        mock_messaging
            .expect_edit_labels_msg()
            .withf(move |&cid, _, _, rid, fp| cid == CHAT_ID && rid == repo_id && *fp == from_page)
            .times(1)
            .returning(|_, _, _, _, _| Ok(()));

        // Other calls are only expected once.
        mock_messaging
            .expect_answer_labels_callback_query()
            .withf(move |&cid, _, _, rid, fp| cid == CHAT_ID && rid == repo_id && *fp == from_page)
            .times(1)
            .returning(|_, _, _, _, _| Ok(()));

        mock_repository
            .expect_toggle_label()
            .with(eq(CHAT_ID), eq(repo_entity.clone()), eq(label_to_toggle))
            .times(1)
            .returning(|_, _, _| Ok(true));

        mock_messaging
            .expect_answer_toggle_label_callback_query()
            .withf(move |_, name, is_selected| name == label_to_toggle && *is_selected)
            .times(1)
            .returning(|_, _, _| Ok(()));

        let harness = TestHarness::new(mock_messaging, mock_repository).await;

        // Act & Assert: View labels
        let dialogue1 = harness.new_dialogue();
        let view_action = CallbackAction::ViewRepoLabels(repo_id, labels_page, from_page);
        harness.handle_callback_with_dialogue(&view_action, dialogue1.clone()).await.unwrap();
        let state1 = dialogue1.get().await.unwrap();
        assert!(
            matches!(&state1, Some(CommandState::ViewingRepoLabels { repo_id: r, from_page: f }) if r == repo_id && *f == from_page),
            "State should be ViewingRepoLabels"
        );

        // Act & Assert: Toggle label
        let dialogue2 = harness.new_dialogue();
        let toggle_action = CallbackAction::ToggleLabel(label_to_toggle, labels_page, from_page);
        harness.handle_callback_with_dialogue(&toggle_action, dialogue2.clone()).await.unwrap();
        let final_state = dialogue2.get().await.unwrap();
        assert!(
            matches!(&final_state, Some(CommandState::ViewingRepoLabels { repo_id: r, from_page: f }) if r == repo_id && *f == from_page),
            "State should remain ViewingRepoLabels"
        );
    }

    #[tokio::test]
    async fn test_handle_callback_view_repo_labels() {
        // Arrange
        let mut mock_messaging = MockMessagingService::new();
        let mut mock_repository = MockRepositoryService::new();
        let repo_id = "owner/repo";
        let page = 1;
        let from_page = 1;
        let repo_entity = RepoEntity::from_str(repo_id).unwrap();

        let paginated_labels = Paginated::new(vec![], page);
        mock_repository
            .expect_get_repo_github_labels()
            .with(eq(CHAT_ID), eq(repo_entity.clone()), eq(page))
            .times(1)
            .returning(move |_, _, _| Ok(paginated_labels.clone()));

        mock_messaging.expect_answer_callback_query().times(1).returning(|_, _| Ok(()));

        mock_messaging
            .expect_answer_labels_callback_query()
            .withf(move |&cid, _, labels, r_id, fp| {
                cid == CHAT_ID && labels.items.is_empty() && r_id == repo_id && *fp == from_page
            })
            .times(1)
            .returning(|_, _, _, _, _| Ok(()));

        let harness = TestHarness::new(mock_messaging, mock_repository).await;
        let action = CallbackAction::ViewRepoLabels(repo_id, page, from_page);

        // Act
        let result = harness.handle_callback(&action).await;

        // Assert
        assert!(result.is_ok());
        let state = harness.dialogue.get().await.unwrap();
        assert!(
            matches!(&state, Some(CommandState::ViewingRepoLabels { repo_id: r, from_page: f }) if r == repo_id && *f == from_page),
            "State should be ViewingRepoLabels"
        );
    }

    #[tokio::test]
    async fn test_handle_callback_view_repo_labels_error() {
        // Arrange
        let mut mock_messaging = MockMessagingService::new();
        let mut mock_repository = MockRepositoryService::new();
        let repo_id = "owner/repo";
        let page = 1;
        let from_page = 1;
        let repo_entity = RepoEntity::from_str(repo_id).unwrap();

        mock_repository
            .expect_get_repo_github_labels()
            .with(eq(CHAT_ID), eq(repo_entity.clone()), eq(page))
            .times(1)
            .returning(|_, _, _| {
                Err(RepositoryServiceError::StorageError(StorageError::DbError(
                    "DB is down".to_string(),
                )))
            });

        mock_messaging.expect_answer_callback_query().times(1).returning(|_, _| Ok(()));

        let harness = TestHarness::new(mock_messaging, mock_repository).await;
        let action = CallbackAction::ViewRepoLabels(repo_id, page, from_page);

        // Act
        let result = harness.handle_callback(&action).await;

        // Assert
        assert!(matches!(result, Err(BotHandlerError::InternalError(_))));
    }
}
