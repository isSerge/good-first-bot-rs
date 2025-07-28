use std::collections::HashSet;

use futures::{StreamExt, stream};

use crate::{
    bot_handler::{BotHandlerError, BotHandlerResult, CommandState, commands::Context},
    storage::RepoEntity,
};

/// A struct to hold the summary of the add operation.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct AddSummary {
    /// Repositories that were successfully added.
    pub successfully_added: HashSet<String>,
    /// Repositories that were already tracked.
    pub already_tracked: HashSet<String>,
    /// Repositories that were not found on GitHub.
    pub not_found: HashSet<String>,
    /// URLs that were invalid.
    pub invalid_urls: HashSet<String>,
    /// Repositories that failed to be added due to an error.
    pub errors: HashSet<(String, String)>,
}

pub async fn handle(ctx: Context<'_>) -> BotHandlerResult<()> {
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

/// Handle the reply message when we're waiting for repository input.
/// It processes the input, checks each URL, and adds the repositories
/// accordingly.
pub async fn handle_reply(ctx: Context<'_>, text: &str) -> BotHandlerResult<()> {
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

    let status_msg = ctx
        .handler
        .messaging_service
        .send_text_message(ctx.message.chat.id, "Processing... ⏳")
        .await?;

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
        .edit_add_summary_msg(ctx.message.chat.id, status_msg.id, &summary)
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use mockall::predicate::eq;

    use super::*;
    use crate::{
        bot_handler::{
            Command,
            test_helpers::{
                CHAT_ID, TestHarness, setup_add_repo_mocks, str_hashset, str_tuple_hashset,
            },
        },
        github::GithubError,
        messaging::MockMessagingService,
        repository::{MockRepositoryService, RepositoryServiceError},
        storage::StorageError,
    };

    #[tokio::test]
    async fn test_add_repos_success() {
        // Arrange
        let mut mock_messaging = MockMessagingService::new();
        let mut mock_repository = MockRepositoryService::new();

        let repo_owner = "owner";
        let repo_name = "repo";
        let repo_name_with_owner = "owner/repo";
        let repo_url = "https://github.com/owner/repo";

        setup_add_repo_mocks(&mut mock_messaging);

        mock_repository
            .expect_repo_exists()
            .with(eq(repo_owner), eq(repo_name))
            .times(1)
            .returning(|_, _| Ok(true));
        mock_repository
            .expect_add_repo()
            .withf(move |&id, entity| {
                id == CHAT_ID && entity.name_with_owner == repo_name_with_owner
            })
            .times(1)
            .returning(|_, _| Ok(true));

        let expected_summary = AddSummary {
            successfully_added: str_hashset(&[repo_name_with_owner]),
            ..Default::default()
        };
        mock_messaging
            .expect_edit_add_summary_msg()
            .withf(move |&chat_id_param, _, summary| {
                chat_id_param == CHAT_ID && summary == &expected_summary
            })
            .times(1)
            .returning(|_, _, _| Ok(()));

        let harness = TestHarness::new(mock_messaging, mock_repository).await;

        // Act
        let result = harness.handle_add_reply(repo_url).await;

        // Assert
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_add_repos_already_tracked() {
        // Arrange
        let mut mock_messaging = MockMessagingService::new();
        let mut mock_repository = MockRepositoryService::new();
        let repo_name_with_owner = "owner/repo";
        let repo_url = "https://github.com/owner/repo";

        setup_add_repo_mocks(&mut mock_messaging);

        mock_repository.expect_repo_exists().returning(|_, _| Ok(true));
        mock_repository
            .expect_add_repo()
            .withf(move |&id, entity| {
                id == CHAT_ID && entity.name_with_owner == repo_name_with_owner
            })
            .times(1)
            .returning(|_, _| Ok(false));

        let expected_summary = AddSummary {
            already_tracked: str_hashset(&[repo_name_with_owner]),
            ..Default::default()
        };
        mock_messaging
            .expect_edit_add_summary_msg()
            .withf(move |&chat_id_param, _, summary| {
                chat_id_param == CHAT_ID && summary == &expected_summary
            })
            .returning(|_, _, _| Ok(()));

        let harness = TestHarness::new(mock_messaging, mock_repository).await;

        // Act
        let result = harness.handle_add_reply(repo_url).await;

        // Assert
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_add_repos_does_not_exist() {
        // Arrange
        let mut mock_messaging = MockMessagingService::new();
        let mut mock_repository = MockRepositoryService::new();
        let repo_name_with_owner = "owner/nonexistent";
        let repo_url = "https://github.com/owner/nonexistent";

        setup_add_repo_mocks(&mut mock_messaging);

        mock_repository.expect_repo_exists().returning(|_, _| Ok(false));

        let expected_summary =
            AddSummary { not_found: str_hashset(&[repo_name_with_owner]), ..Default::default() };
        mock_messaging
            .expect_edit_add_summary_msg()
            .withf(move |&chat_id_param, _, summary| {
                chat_id_param == CHAT_ID && summary == &expected_summary
            })
            .returning(|_, _, _| Ok(()));

        let harness = TestHarness::new(mock_messaging, mock_repository).await;

        // Act
        let result = harness.handle_add_reply(repo_url).await;

        // Assert
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_add_repos_parse_error() {
        // Arrange
        let mut mock_messaging = MockMessagingService::new();
        let mock_repository = MockRepositoryService::new();
        let invalid_url = "this_is_not_a_url";

        setup_add_repo_mocks(&mut mock_messaging);

        let expected_summary =
            AddSummary { invalid_urls: str_hashset(&[invalid_url]), ..Default::default() };
        mock_messaging
            .expect_edit_add_summary_msg()
            .withf(move |&chat_id_param, _, summary| {
                chat_id_param == CHAT_ID && summary == &expected_summary
            })
            .returning(|_, _, _| Ok(()));

        let harness = TestHarness::new(mock_messaging, mock_repository).await;

        // Act
        let result = harness.handle_add_reply(invalid_url).await;

        // Assert
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_add_repos_error() {
        // Arrange
        let mut mock_messaging = MockMessagingService::new();
        let mut mock_repository = MockRepositoryService::new();
        let repo_name_with_owner = "owner/gh-error";
        let repo_url = "https://github.com/owner/gh-error";
        let error_msg = "Github client error";

        setup_add_repo_mocks(&mut mock_messaging);

        mock_repository.expect_repo_exists().returning(move |_, _| {
            Err(RepositoryServiceError::GithubClientError(GithubError::Unauthorized))
        });

        let expected_summary = AddSummary {
            errors: str_tuple_hashset(&[(repo_name_with_owner, error_msg)]),
            ..Default::default()
        };
        mock_messaging
            .expect_edit_add_summary_msg()
            .withf(move |&chat_id_param, _, summary| {
                chat_id_param == CHAT_ID && summary == &expected_summary
            })
            .returning(|_, _, _| Ok(()));

        let harness = TestHarness::new(mock_messaging, mock_repository).await;

        // Act
        let result = harness.handle_add_reply(repo_url).await;

        // Assert
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_add_repos_limit_reached() {
        // Arrange
        let mut mock_messaging = MockMessagingService::new();
        let mut mock_repository = MockRepositoryService::new();
        let repo_name_with_owner = "owner/repo";
        let repo_url = "https://github.com/owner/repo";
        let limit_error_msg = "You have reached the maximum limit of 10
repositories.";
        let full_error_str =
            RepositoryServiceError::LimitExceeded(limit_error_msg.to_string()).to_string();

        setup_add_repo_mocks(&mut mock_messaging);

        mock_repository.expect_repo_exists().returning(|_, _| Ok(true));
        mock_repository.expect_add_repo().returning(move |_, _| {
            Err(RepositoryServiceError::LimitExceeded(limit_error_msg.to_string()))
        });

        let expected_summary = AddSummary {
            errors: str_tuple_hashset(&[(repo_name_with_owner, &full_error_str)]),
            ..Default::default()
        };

        mock_messaging
            .expect_edit_add_summary_msg()
            .withf(move |&chat_id_param, _, summary| {
                chat_id_param == CHAT_ID && summary == &expected_summary
            })
            .returning(|_, _, _| Ok(()));

        let harness = TestHarness::new(mock_messaging, mock_repository).await;

        // Act
        let result = harness.handle_add_reply(repo_url).await;

        // Assert
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_add_repos_multiple_mixed_outcomes() {
        // Arrange
        let mut mock_messaging = MockMessagingService::new();
        let mut mock_repository = MockRepositoryService::new();
        let url_new = "https://github.com/owner/new";
        let name_new = "owner/new";
        let url_tracked = "https://github.com/owner/tracked";
        let name_tracked = "owner/tracked";
        let url_notfound = "https://github.com/owner/notfound";
        let name_notfound = "owner/notfound";
        let url_invalid = "invalid-url";
        let url_gh_error = "https://github.com/owner/gh-error";
        let name_gh_error = "owner/gh-error";
        let gh_error_msg = "Github client error";
        let url_add_error = "https://github.com/owner/add-error";
        let name_add_error = "owner/add-error";
        let db_failure_reason = "DB Add Failed";
        let add_error_msg = format!("Storage error: Database error: {db_failure_reason}");

        setup_add_repo_mocks(&mut mock_messaging);

        mock_repository
            .expect_repo_exists()
            .with(eq("owner"), eq("new"))
            .returning(|_, _| Ok(true));
        mock_repository
            .expect_add_repo()
            .withf(move |_, e: &RepoEntity| e.name_with_owner == name_new)
            .returning(|_, _| Ok(true));
        mock_repository
            .expect_repo_exists()
            .with(eq("owner"), eq("tracked"))
            .returning(|_, _| Ok(true));
        mock_repository
            .expect_add_repo()
            .withf(move |_, e: &RepoEntity| e.name_with_owner == name_tracked)
            .returning(|_, _| Ok(false));
        mock_repository
            .expect_repo_exists()
            .with(eq("owner"), eq("notfound"))
            .returning(|_, _| Ok(false));
        mock_repository.expect_repo_exists().with(eq("owner"), eq("gh-error")).returning(
            move |_, _| Err(RepositoryServiceError::GithubClientError(GithubError::Unauthorized)),
        );
        mock_repository
            .expect_repo_exists()
            .with(eq("owner"), eq("add-error"))
            .returning(|_, _| Ok(true));
        mock_repository
            .expect_add_repo()
            .withf(move |_, e: &RepoEntity| e.name_with_owner == name_add_error)
            .returning(move |_, _| {
                Err(RepositoryServiceError::StorageError(StorageError::DbError(
                    db_failure_reason.to_string(),
                )))
            });

        let expected_summary = AddSummary {
            successfully_added: str_hashset(&[name_new]),
            already_tracked: str_hashset(&[name_tracked]),
            not_found: str_hashset(&[name_notfound]),
            invalid_urls: str_hashset(&[url_invalid]),
            errors: str_tuple_hashset(&[
                (name_gh_error, gh_error_msg),
                (name_add_error, &add_error_msg),
            ]),
        };

        mock_messaging
            .expect_edit_add_summary_msg()
            .withf(move |&ch_id, _, summary| ch_id == CHAT_ID && summary == &expected_summary)
            .times(1)
            .returning(|_, _, _| Ok(()));

        let harness = TestHarness::new(mock_messaging, mock_repository).await;
        let mock_msg_text = format!(
            "{url_new} {url_tracked} {url_notfound} {url_invalid} {url_gh_error}
{url_add_error}"
        );

        // Act
        let result = harness.handle_add_reply(&mock_msg_text).await;

        // Assert
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_dialogue_persists_awaiting_add_repo_state() {
        // Arrage
        let mut mock_messaging = MockMessagingService::new();
        let mut mock_repository = MockRepositoryService::new();
        let repo_url = "https://github.com/owner/repo";
        let repo_name_with_owner = "owner/repo";

        // Set ALL expectations for the entire test flow
        // Expectation for Interaction 1 (/add command)
        mock_messaging
            .expect_prompt_for_repo_input()
            .with(eq(CHAT_ID))
            .times(1)
            .returning(|_| Ok(()));

        // Expectations for Interaction 2 (the reply to the prompt)
        setup_add_repo_mocks(&mut mock_messaging);

        // Expect the repository to exist
        mock_repository
            .expect_repo_exists()
            .with(eq("owner"), eq("repo"))
            .times(1)
            .returning(|_, _| Ok(true));

        // Expect the repository to be added
        mock_repository
            .expect_add_repo()
            .withf(move |&id, e| id == CHAT_ID && e.name_with_owner == repo_name_with_owner)
            .times(1)
            .returning(|_, _| Ok(true));

        let expected_summary = AddSummary {
            successfully_added: str_hashset(&[repo_name_with_owner]),
            ..Default::default()
        };
        mock_messaging
            .expect_edit_add_summary_msg()
            .withf(move |&cid, _, summary| cid == CHAT_ID && summary == &expected_summary)
            .times(1)
            .returning(|_, _, _| Ok(()));

        let harness = TestHarness::new(mock_messaging, mock_repository).await;

        // Act & Assert: Initial command
        let dialogue1 = harness.new_dialogue();
        harness.handle_command_with_dialogue(Command::Add, dialogue1.clone()).await.unwrap();
        assert!(
            matches!(dialogue1.get().await.unwrap(), Some(CommandState::AwaitingAddRepo)),
            "State should be AwaitingAddRepo"
        );

        // Act & Assert: Reply with repo URL
        let dialogue2 = harness.new_dialogue();
        assert!(
            matches!(dialogue2.get().await.unwrap(), Some(CommandState::AwaitingAddRepo)),
            "State should persist"
        );
        harness.handle_reply_with_dialogue(repo_url, &dialogue2).await.unwrap();
        assert!(dialogue2.get().await.unwrap().is_none(), "State should be cleared");
    }

    #[tokio::test]
    async fn test_handle_reply_invalid_state() {
        // Arrange
        let mut mock_messaging = MockMessagingService::new();
        let mock_repository = MockRepositoryService::new();

        // Set an initial state for the dialogue to ensure it exists in storage.
        mock_messaging
            .expect_send_error_msg()
            .withf(move |&cid, e| {
                cid == CHAT_ID
                    && matches!(e, BotHandlerError::InvalidInput(s) if s == "Invalid state")
            })
            .times(1)
            .returning(|_, _| Ok(()));

        let harness = TestHarness::new(mock_messaging, mock_repository).await;
        harness.dialogue.update(CommandState::None).await.unwrap();

        // Act
        let result =
            harness.handle_reply_with_dialogue("some random text", &harness.dialogue).await;

        // Assert
        assert!(result.is_ok());
        let state = harness.dialogue.get().await.unwrap();
        assert!(state.is_none(), "Dialogue state should be cleared");
    }

    #[tokio::test]
    async fn test_handle_reply_awaiting_add_repo_success() {
        // Arrange
        let mut mock_messaging = MockMessagingService::new();
        let mut mock_repository = MockRepositoryService::new();
        let repo_url = "https://github.com/owner/repo";
        let repo_name_with_owner = "owner/repo";

        setup_add_repo_mocks(&mut mock_messaging);

        // Mock the repository interactions for a successful add
        mock_repository
            .expect_repo_exists()
            .with(eq("owner"), eq("repo"))
            .times(1)
            .returning(|_, _| Ok(true));
        mock_repository
            .expect_add_repo()
            .withf(move |&id, e| id == CHAT_ID && e.name_with_owner == repo_name_with_owner)
            .times(1)
            .returning(|_, _| Ok(true));

        let expected_summary = AddSummary {
            successfully_added: str_hashset(&[repo_name_with_owner]),
            ..Default::default()
        };
        mock_messaging
            .expect_edit_add_summary_msg()
            .withf(move |&cid, _, summary| cid == CHAT_ID && summary == &expected_summary)
            .times(1)
            .returning(|_, _, _| Ok(()));

        let harness = TestHarness::new(mock_messaging, mock_repository).await;
        // Set the state to AwaitingAddRepo
        harness.dialogue.update(CommandState::AwaitingAddRepo).await.unwrap();

        // Act
        let result = harness.handle_reply_with_dialogue(repo_url, &harness.dialogue).await;

        // Assert
        assert!(result.is_ok());
        let state = harness.dialogue.get().await.unwrap();
        assert!(state.is_none(), "Dialogue state should be cleared after successful reply");
    }
}
