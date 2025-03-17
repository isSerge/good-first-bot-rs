use crate::bot_handler::BotHandler;
use crate::bot_handler::services::messaging::MockMessagingService;
use crate::bot_handler::services::repository::MockRepositoryService;
use mockall::predicate::*;
use std::sync::Arc;
use teloxide::types::ChatId;

#[tokio::test]
async fn test_process_add_success() {
    let mut mock_messaging = MockMessagingService::new();
    let mut mock_repository = MockRepositoryService::new();

    // Mock the repository exists and is not already tracked
    mock_repository
        .expect_repo_exists()
        .with(eq("owner"), eq("repo"))
        .returning(|_, _| Ok(true));
    mock_repository
        .expect_contains_repo()
        .with(eq(ChatId(123)), always())
        .returning(|_, _| Ok(false));
    mock_repository
        .expect_add_repo()
        .with(eq(ChatId(123)), always())
        .returning(|_, _| Ok(()));

    // Mock the messaging service to expect a success message
    mock_messaging
        .expect_send_repo_added_msg()
        .with(eq(ChatId(123)), eq(String::from("owner/repo")))
        .returning(|_, _| Ok(()));

    let bot_handler = BotHandler::new(Arc::new(mock_messaging), Arc::new(mock_repository));
    let mock_msg_text = "https://github.com/owner/repo";

    let result = bot_handler.process_add(mock_msg_text, ChatId(123)).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_process_add_already_tracked() {
    let mut mock_messaging = MockMessagingService::new();
    let mut mock_repository = MockRepositoryService::new();

    // Mock the repository exists and is already tracked
    mock_repository
        .expect_repo_exists()
        .with(eq("owner"), eq("repo"))
        .returning(|_, _| Ok(true));
    mock_repository
        .expect_contains_repo()
        .with(eq(ChatId(123)), always())
        .returning(|_, _| Ok(true));

    // Mock the messaging service to expect an "already tracked" message
    mock_messaging
        .expect_send_already_tracked_msg()
        .with(eq(ChatId(123)), eq(String::from("owner/repo")))
        .returning(|_, _| Ok(()));

    let bot_handler = BotHandler::new(Arc::new(mock_messaging), Arc::new(mock_repository));
    let mock_msg_text = "https://github.com/owner/repo";

    let result = bot_handler.process_add(mock_msg_text, ChatId(123)).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_process_add_repo_does_not_exist() {
    let mut mock_messaging = MockMessagingService::new();
    let mut mock_repository = MockRepositoryService::new();

    // Mock the repository does not exist
    mock_repository
        .expect_repo_exists()
        .with(eq("owner"), eq("repo"))
        .returning(|_, _| Ok(false));

    // Mock the messaging service to expect a "no repo exists" message
    mock_messaging
        .expect_send_no_repo_exists_msg()
        .with(eq(ChatId(123)), eq(String::from("owner/repo")))
        .returning(|_, _| Ok(()));

    let bot_handler = BotHandler::new(Arc::new(mock_messaging), Arc::new(mock_repository));
    let mock_msg_text = "https://github.com/owner/repo";

    let result = bot_handler.process_add(mock_msg_text, ChatId(123)).await;
    // The error is handled by sending an error message, so process_add returns Ok(()).
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_process_add_parse_error() {
    let mut mock_messaging = MockMessagingService::new();
    let mock_repository = MockRepositoryService::new();

    // Use an invalid repository URL that causes parsing to fail.
    // When parsing fails, process_add should call send_error_msg.
    mock_messaging
        .expect_send_error_msg()
        .with(eq(ChatId(123)), always())
        .returning(|_, _| Ok(()));

    // In this case, remove_repo should never be called, so no expectation is set on mock_repository.
    let bot_handler = BotHandler::new(Arc::new(mock_messaging), Arc::new(mock_repository));
    let mock_msg_text = "invalid_repo_url";

    let result = bot_handler.process_add(mock_msg_text, ChatId(123)).await;
    // The error is handled by sending an error message, so process_add returns Ok(()).
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_process_add_error() {
    let mut mock_messaging = MockMessagingService::new();
    let mut mock_repository = MockRepositoryService::new();

    // Mock the repository exists check to return an error
    mock_repository
        .expect_repo_exists()
        .with(eq("owner"), eq("repo"))
        .returning(|_, _| Err(anyhow::anyhow!("GitHub API error")));

    // Mock the messaging service to expect an error message
    mock_messaging
        .expect_send_error_msg()
        .with(eq(ChatId(123)), always())
        .returning(|_, _| Ok(()));

    let bot_handler = BotHandler::new(Arc::new(mock_messaging), Arc::new(mock_repository));
    let mock_msg_text = "https://github.com/owner/repo";

    let result = bot_handler.process_add(mock_msg_text, ChatId(123)).await;
    // The error is handled by sending an error message, so process_add returns Ok(()).
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_process_remove_success() {
    let mut mock_messaging = MockMessagingService::new();
    let mut mock_repository = MockRepositoryService::new();

    // For a valid repo URL, we expect the repository to be removed.
    mock_repository
        .expect_remove_repo()
        .with(eq(ChatId(123)), eq("owner/repo"))
        .returning(|_, _| Ok(true));

    // When removal succeeds, expect messaging to send a removal confirmation.
    mock_messaging
        .expect_send_repo_removed_msg()
        .with(eq(ChatId(123)), eq(String::from("owner/repo")))
        .returning(|_, _| Ok(()));

    let bot_handler = BotHandler::new(Arc::new(mock_messaging), Arc::new(mock_repository));
    let mock_msg_text = "https://github.com/owner/repo";

    let result = bot_handler.process_remove(mock_msg_text, ChatId(123)).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_process_remove_not_tracked() {
    let mut mock_messaging = MockMessagingService::new();
    let mut mock_repository = MockRepositoryService::new();

    // Simulate that the repository is not tracked.
    mock_repository
        .expect_remove_repo()
        .with(eq(ChatId(123)), eq("owner/repo"))
        .returning(|_, _| Ok(false));

    // Expect messaging to send a "not tracked" message.
    mock_messaging
        .expect_send_repo_not_tracked_msg()
        .with(eq(ChatId(123)), eq(String::from("owner/repo")))
        .returning(|_, _| Ok(()));

    let bot_handler = BotHandler::new(Arc::new(mock_messaging), Arc::new(mock_repository));
    let mock_msg_text = "https://github.com/owner/repo";

    let result = bot_handler.process_remove(mock_msg_text, ChatId(123)).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_process_remove_parse_error() {
    let mut mock_messaging = MockMessagingService::new();
    let mock_repository = MockRepositoryService::new();

    // Use an invalid repository URL that causes parsing to fail.
    // When parsing fails, process_remove should call send_error_msg.
    mock_messaging
        .expect_send_error_msg()
        .with(eq(ChatId(123)), always())
        .returning(|_, _| Ok(()));

    // In this case, remove_repo should never be called, so no expectation is set on mock_repository.
    let bot_handler = BotHandler::new(Arc::new(mock_messaging), Arc::new(mock_repository));
    let mock_msg_text = "invalid_repo_url";

    let result = bot_handler.process_remove(mock_msg_text, ChatId(123)).await;
    // The error is handled by sending an error message, so process_remove returns Ok(()).
    assert!(result.is_ok());
}
