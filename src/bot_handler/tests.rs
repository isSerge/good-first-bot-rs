use std::{collections::HashSet, sync::Arc};

use mockall::predicate::*;
use teloxide::types::ChatId;

use crate::{
    bot_handler::BotHandler,
    messaging::MockMessagingService,
    repository::{MockRepositoryService, RepositoryServiceError},
    storage::{RepoEntity, StorageError},
};

const CHAT_ID: ChatId = ChatId(123);

// Helper function to create a HashSet from a slice of strings
fn str_hashset(items: &[&str]) -> HashSet<String> {
    items.iter().map(|s| s.to_string()).collect()
}

// Helper function to create a HashSet of (String, String) tuples
fn str_tuple_hashset(items: &[(&str, &str)]) -> HashSet<(String, String)> {
    items.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect()
}

#[tokio::test]
async fn test_process_add_success() {
    // Arrange
    let mut mock_messaging = MockMessagingService::new();
    let mut mock_repository = MockRepositoryService::new();

    let repo_owner = "owner";
    let repo_name = "repo";
    let repo_name_with_owner = "owner/repo";
    let repo_url = "https://github.com/owner/repo";

    // Mock repository interactions
    mock_repository
        .expect_repo_exists()
        .with(eq(repo_owner), eq(repo_name))
        .times(1) // Expect it to be called once for this repo
        .returning(|_, _| Ok(true));
    mock_repository
        .expect_contains_repo()
        .withf(move |&id, entity| id == CHAT_ID && entity.name_with_owner == repo_name_with_owner)
        .times(1)
        .returning(|_, _| Ok(false));
    mock_repository
        .expect_add_repo()
        .withf(move |&id, entity| id == CHAT_ID && entity.name_with_owner == repo_name_with_owner)
        .times(1)
        .returning(|_, _| Ok(()));

    // Expect send_add_summary_msg to be called
    let expected_successfully_added = str_hashset(&[repo_name_with_owner]);
    mock_messaging
        .expect_send_add_summary_msg()
        .withf(move |&chat_id_param, s_added, a_tracked, n_found, inv_urls, p_errors| {
            chat_id_param == CHAT_ID &&
                *s_added == expected_successfully_added && // Compare HashSets directly
                a_tracked.is_empty() &&
                n_found.is_empty() &&
                inv_urls.is_empty() &&
                p_errors.is_empty()
        })
        .times(1)
        .returning(|_, _, _, _, _, _| Ok(()));

    let bot_handler = BotHandler::new(Arc::new(mock_messaging), Arc::new(mock_repository));

    // Act
    let result = bot_handler.process_add(repo_url, CHAT_ID).await;

    // Assert
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_process_add_already_tracked() {
    // Arrange
    let mut mock_messaging = MockMessagingService::new();
    let mut mock_repository = MockRepositoryService::new();

    let repo_name_with_owner = "owner/repo";
    let repo_url = "https://github.com/owner/repo";

    mock_repository.expect_repo_exists().returning(|_, _| Ok(true));
    mock_repository.expect_contains_repo().returning(|_, _| Ok(true));
    // add_repo should not be called

    let expected_already_tracked = str_hashset(&[repo_name_with_owner]);
    mock_messaging
        .expect_send_add_summary_msg()
        .withf(move |&chat_id_param, s_added, a_tracked, n_found, inv_urls, p_errors| {
            chat_id_param == CHAT_ID
                && s_added.is_empty()
                && *a_tracked == expected_already_tracked
                && n_found.is_empty()
                && inv_urls.is_empty()
                && p_errors.is_empty()
        })
        .returning(|_, _, _, _, _, _| Ok(()));

    let bot_handler = BotHandler::new(Arc::new(mock_messaging), Arc::new(mock_repository));

    // Act
    let result = bot_handler.process_add(repo_url, CHAT_ID).await;

    // Assert
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_process_add_repo_does_not_exist() {
    // Arrange
    let mut mock_messaging = MockMessagingService::new();
    let mut mock_repository = MockRepositoryService::new();

    let repo_name_with_owner = "owner/nonexistent";
    let repo_url = "https://github.com/owner/nonexistent";

    mock_repository.expect_repo_exists().returning(|_, _| Ok(false));
    // contains_repo and add_repo should not be called

    let expected_not_found = str_hashset(&[repo_name_with_owner]);
    mock_messaging
        .expect_send_add_summary_msg()
        .withf(move |&chat_id_param, s_added, a_tracked, n_found, inv_urls, p_errors| {
            chat_id_param == CHAT_ID
                && s_added.is_empty()
                && a_tracked.is_empty()
                && *n_found == expected_not_found
                && inv_urls.is_empty()
                && p_errors.is_empty()
        })
        .returning(|_, _, _, _, _, _| Ok(()));

    let bot_handler = BotHandler::new(Arc::new(mock_messaging), Arc::new(mock_repository));

    // Act
    let result = bot_handler.process_add(repo_url, CHAT_ID).await;

    // Assert
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_process_add_parse_error() {
    // Arrange
    let mut mock_messaging = MockMessagingService::new();
    let mock_repository = MockRepositoryService::new(); // No repo interactions expected

    let invalid_url = "this_is_not_a_url";

    let expected_invalid_urls = str_hashset(&[invalid_url]);
    mock_messaging
        .expect_send_add_summary_msg()
        .withf(move |&chat_id_param, s_added, a_tracked, n_found, inv_urls, p_errors| {
            chat_id_param == CHAT_ID
                && s_added.is_empty()
                && a_tracked.is_empty()
                && n_found.is_empty()
                && *inv_urls == expected_invalid_urls
                && p_errors.is_empty()
        })
        .returning(|_, _, _, _, _, _| Ok(()));

    let bot_handler = BotHandler::new(Arc::new(mock_messaging), Arc::new(mock_repository));

    // Act
    let result = bot_handler.process_add(invalid_url, CHAT_ID).await;

    // Assert
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_process_add_error() {
    // Arrange
    let mut mock_messaging = MockMessagingService::new();
    let mut mock_repository = MockRepositoryService::new();

    let repo_name_with_owner = "owner/gh-error";
    let repo_url = "https://github.com/owner/gh-error";
    let error_msg = "GitHub API error";

    mock_repository
        .expect_repo_exists()
        .returning(move |_, _| Err(RepositoryServiceError::RepoExistCheckFailed));

    let expected_errors = str_tuple_hashset(&[(repo_name_with_owner, error_msg)]);
    mock_messaging
        .expect_send_add_summary_msg()
        .withf(move |&chat_id_param, s_added, a_tracked, n_found, inv_urls, p_errors| {
            chat_id_param == CHAT_ID
                && s_added.is_empty()
                && a_tracked.is_empty()
                && n_found.is_empty()
                && inv_urls.is_empty()
                && *p_errors == expected_errors
        })
        .returning(|_, _, _, _, _, _| Ok(()));

    let bot_handler = BotHandler::new(Arc::new(mock_messaging), Arc::new(mock_repository));

    // Act
    let result = bot_handler.process_add(repo_url, CHAT_ID).await;

    // Assert
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_process_add_multiple_mixed_outcomes() {
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
    let gh_error_msg = "GH API Failed";
    let url_add_error = "https://github.com/owner/add-error";
    let name_add_error = "owner/add-error";
    let add_error_msg = "DB Add Failed";

    // Mocking for 'new' repo
    mock_repository.expect_repo_exists().with(eq("owner"), eq("new")).returning(|_, _| Ok(true));
    mock_repository
        .expect_contains_repo()
        .withf(move |_, e: &RepoEntity| e.name_with_owner == name_new)
        .returning(|_, _| Ok(false));
    mock_repository
        .expect_add_repo()
        .withf(move |_, e: &RepoEntity| e.name_with_owner == name_new)
        .returning(|_, _| Ok(()));

    // Mocking for 'tracked' repo
    mock_repository
        .expect_repo_exists()
        .with(eq("owner"), eq("tracked"))
        .returning(|_, _| Ok(true));
    mock_repository
        .expect_contains_repo()
        .withf(move |_, e: &RepoEntity| e.name_with_owner == name_tracked)
        .returning(|_, _| Ok(true));

    // Mocking for 'notfound' repo
    mock_repository
        .expect_repo_exists()
        .with(eq("owner"), eq("notfound"))
        .returning(|_, _| Ok(false));

    // Mocking for 'gh-error' repo
    mock_repository
        .expect_repo_exists()
        .with(eq("owner"), eq("gh-error"))
        .returning(move |_, _| Err(RepositoryServiceError::RepoExistCheckFailed));

    // Mocking for 'add-error' repo
    mock_repository
        .expect_repo_exists()
        .with(eq("owner"), eq("add-error"))
        .returning(|_, _| Ok(true));
    mock_repository
        .expect_contains_repo()
        .withf(move |_, e: &RepoEntity| e.name_with_owner == name_add_error)
        .returning(|_, _| Ok(false));
    mock_repository
        .expect_add_repo()
        .withf(move |_, e: &RepoEntity| e.name_with_owner == name_add_error)
        .returning(move |_, _| {
            Err(RepositoryServiceError::StorageError(StorageError::DbError(
                add_error_msg.to_string(),
            )))
        });

    // Expected HashSets for summary
    let expected_s_added = str_hashset(&[name_new]);
    let expected_a_tracked = str_hashset(&[name_tracked]);
    let expected_n_found = str_hashset(&[name_notfound]);
    let expected_inv_urls = str_hashset(&[url_invalid]);
    let expected_p_errors =
        str_tuple_hashset(&[(name_gh_error, gh_error_msg), (name_add_error, add_error_msg)]);

    mock_messaging
        .expect_send_add_summary_msg()
        .withf(move |&ch_id, s, a, n, inv, p_err| {
            ch_id == CHAT_ID
                && *s == expected_s_added
                && *a == expected_a_tracked
                && *n == expected_n_found
                && *inv == expected_inv_urls
                && *p_err == expected_p_errors
        })
        .times(1)
        .returning(|_, _, _, _, _, _| Ok(()));

    let bot_handler = BotHandler::new(Arc::new(mock_messaging), Arc::new(mock_repository));
    let mock_msg_text = format!(
        "{} {} {} {} {} {}",
        url_new, url_tracked, url_notfound, url_invalid, url_gh_error, url_add_error
    );

    // Act
    let result = bot_handler.process_add(&mock_msg_text, CHAT_ID).await;

    // Assert
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_process_remove_success() {
    // Arrange
    let mut mock_messaging = MockMessagingService::new();
    let mut mock_repository = MockRepositoryService::new();

    // For a valid repo URL, we expect the repository to be removed.
    mock_repository
        .expect_remove_repo()
        .with(eq(CHAT_ID), eq("owner/repo"))
        .returning(|_, _| Ok(true));

    // When removal succeeds, expect messaging to send a removal confirmation.
    mock_messaging
        .expect_send_repo_removed_msg()
        .with(eq(CHAT_ID), eq(String::from("owner/repo")))
        .returning(|_, _| Ok(()));

    let bot_handler = BotHandler::new(Arc::new(mock_messaging), Arc::new(mock_repository));
    let mock_msg_text = "https://github.com/owner/repo";

    // Act
    let result = bot_handler.process_remove(mock_msg_text, CHAT_ID).await;

    // Assert
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_process_remove_not_tracked() {
    // Arrange
    let mut mock_messaging = MockMessagingService::new();
    let mut mock_repository = MockRepositoryService::new();

    // Simulate that the repository is not tracked.
    mock_repository
        .expect_remove_repo()
        .with(eq(CHAT_ID), eq("owner/repo"))
        .returning(|_, _| Ok(false));

    // Expect messaging to send a "not tracked" message.
    mock_messaging
        .expect_send_repo_not_tracked_msg()
        .with(eq(CHAT_ID), eq(String::from("owner/repo")))
        .returning(|_, _| Ok(()));

    let bot_handler = BotHandler::new(Arc::new(mock_messaging), Arc::new(mock_repository));
    let mock_msg_text = "https://github.com/owner/repo";

    // Act
    let result = bot_handler.process_remove(mock_msg_text, CHAT_ID).await;

    // Assert
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_process_remove_parse_error() {
    // Arrange
    let mut mock_messaging = MockMessagingService::new();
    let mock_repository = MockRepositoryService::new();

    // Use an invalid repository URL that causes parsing to fail.
    // When parsing fails, process_remove should call send_error_msg.
    mock_messaging.expect_send_error_msg().with(eq(CHAT_ID), always()).returning(|_, _| Ok(()));

    // In this case, remove_repo should never be called, so no expectation is set on
    // mock_repository.
    let bot_handler = BotHandler::new(Arc::new(mock_messaging), Arc::new(mock_repository));
    let mock_msg_text = "invalid_repo_url";

    // Act
    let result = bot_handler.process_remove(mock_msg_text, ChatId(123)).await;

    // Assert
    // The error is handled by sending an error message, so process_remove returns
    // Ok(()).
    assert!(result.is_ok());
}
