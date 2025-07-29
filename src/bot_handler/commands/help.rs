use crate::bot_handler::{BotHandlerResult, commands::Context};

pub async fn handle(ctx: Context<'_>) -> BotHandlerResult<()> {
    ctx.handler.messaging_service.send_help_msg(ctx.message.chat.id).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        bot_handler::{
            test_helpers::{TestHarness, CHAT_ID}, CallbackAction
        },
        messaging::{MessagingError, MockMessagingService},
        repository::MockRepositoryService,
    };

    #[tokio::test]
    async fn test_handle_help_command() {
        // Arrange
        let mut mock_messaging = MockMessagingService::new();

        mock_messaging.expect_answer_callback_query()
            .times(1)
            .returning(|_, _| Ok(()));

        mock_messaging
            .expect_send_help_msg()
            .withf(|&chat_id| chat_id == CHAT_ID)
            .times(1)
            .returning(|_| Ok(()));

        let mock_repository = MockRepositoryService::new();
        let harness = TestHarness::new(mock_messaging, mock_repository).await;

        // Act
        let result = harness.handle_callback(&CallbackAction::CmdHelp).await;

        // Assert
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_help_command_messaging_error() {
        // Arrange
        let mut mock_messaging = MockMessagingService::new();

        mock_messaging.expect_answer_callback_query()
            .times(1)
            .returning(|_, _| Ok(()));

        mock_messaging.expect_send_help_msg()
            .withf(|&chat_id| chat_id == CHAT_ID)
            .times(1)
            .returning(|_| Err(MessagingError::TeloxideRequest(
                teloxide::RequestError::RetryAfter(teloxide::types::Seconds::from_seconds(5))
            )));

        let mock_repository = MockRepositoryService::new();
        let harness = TestHarness::new(mock_messaging, mock_repository).await;

        // Act
        let result = harness.handle_callback(&CallbackAction::CmdHelp).await;

        // Assert
        assert!(result.is_err());
    }
}
