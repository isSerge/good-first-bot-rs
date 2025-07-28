use std::{collections::HashSet, sync::Arc};

use chrono::Utc;
use mockall::predicate::*;
use teloxide::{
    dispatching::dialogue::{Dialogue, serializer},
    types::{
        Chat, ChatId, ChatKind, ChatPrivate, MaybeInaccessibleMessage, MediaKind, MediaText,
        Message, MessageCommon, MessageId, MessageKind, User,
    },
};

use super::*;
use crate::{
    bot_handler::{BotHandler, Command, CommandState},
    messaging::MockMessagingService,
    repository::MockRepositoryService,
};

pub const CHAT_ID: ChatId = ChatId(123);

// Test harness to encapsulate common test setup and actions.
pub struct TestHarness {
    bot_handler: BotHandler,
    pub dialogue: Dialogue<CommandState, DialogueStorage>,
    storage: Arc<DialogueStorage>,
}

impl TestHarness {
    // Creates a new TestHarness with mock services.
    pub async fn new(
        mock_messaging: MockMessagingService,
        mock_repository: MockRepositoryService,
    ) -> Self {
        let max_concurrency = 10;
        let bot_handler =
            BotHandler::new(Arc::new(mock_messaging), Arc::new(mock_repository), max_concurrency);
        let storage = DialogueStorage::open("sqlite::memory:", serializer::Json).await.unwrap();
        let dialogue = Dialogue::<CommandState, DialogueStorage>::new(storage.clone(), CHAT_ID);

        Self { bot_handler, dialogue, storage }
    }

    // Creates a new dialogue for the same chat ID to test state persistence.
    pub fn new_dialogue(&self) -> Dialogue<CommandState, DialogueStorage> {
        Dialogue::new(self.storage.clone(), CHAT_ID)
    }

    // Simulates handling a command message.
    pub async fn handle_command_with_dialogue(
        &self,
        command: Command,
        dialogue: Dialogue<CommandState, DialogueStorage>,
    ) -> Result<(), BotHandlerError> {
        let msg = mock_message(CHAT_ID, &format!("/{}", command.to_string()));
        self.bot_handler.handle_commands(&msg, command, dialogue).await
    }

    // Simulates handling a reply message.
    pub async fn handle_reply_with_dialogue(
        &self,
        text: &str,
        dialogue: &Dialogue<CommandState, DialogueStorage>,
    ) -> Result<(), BotHandlerError> {
        let mut msg = mock_message(CHAT_ID, text);
        if let MessageKind::Common(common) = &mut msg.kind {
            common.reply_to_message = Some(Box::new(mock_message(CHAT_ID, "prompt")));
        }
        self.bot_handler.handle_reply(&msg, dialogue).await
    }

    // Simulates handling a callback query.
    // This is the core method for handling callbacks, allowing explicit dialogue
    // control. Use this method directly when testing state persistence across
    // multiple interactions, where you need to create and pass a new dialogue
    // instance for the second interaction.
    pub async fn handle_callback_with_dialogue<'a>(
        &self,
        action: &CallbackAction<'a>,
        dialogue: Dialogue<CommandState, DialogueStorage>,
    ) -> Result<(), BotHandlerError> {
        let (_, query) = mock_callback_query(CHAT_ID, action);
        self.bot_handler.handle_callback_query(&query, dialogue).await
    }

    // Simulates handling a callback query with the main dialogue.
    // This is a convenience wrapper around `handle_callback_with_dialogue`.
    // Use this for most callback tests where you are only testing a single
    // interaction and don't need to manually manage the dialogue instance.
    pub async fn handle_callback<'a>(
        &self,
        action: &CallbackAction<'a>,
    ) -> Result<(), BotHandlerError> {
        self.handle_callback_with_dialogue(action, self.dialogue.clone()).await
    }

    // Simulates handling a reply for the 'add' command specifically.
    pub async fn handle_add_reply(&self, text: &str) -> Result<(), BotHandlerError> {
        let message = mock_message(CHAT_ID, text);
        let ctx = Context {
            handler: &self.bot_handler,
            message: &message,
            dialogue: &self.dialogue,
            query: None,
        };
        commands::add::handle_reply(ctx, message.text().unwrap()).await
    }
}

// Helper function to create a HashSet from a slice of strings
pub fn str_hashset(items: &[&str]) -> HashSet<String> {
    items.iter().map(|s| s.to_string()).collect()
}

// Helper function to create a HashSet of (String, String) tuples
pub fn str_tuple_hashset(items: &[(&str, &str)]) -> HashSet<(String, String)> {
    items.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect()
}

// Helper to create a mock teloxide message to reduce boilerplate in tests
pub fn mock_message(chat_id: ChatId, text: &str) -> Message {
    Message {
        id: MessageId(1),
        date: Utc::now(),
        chat: Chat {
            id: chat_id,
            kind: ChatKind::Private(ChatPrivate {
                username: Some("test".to_string()),
                first_name: Some("Test".to_string()),
                last_name: None,
            }),
        },
        kind: MessageKind::Common(MessageCommon {
            media_kind: MediaKind::Text(MediaText {
                text: text.to_string(),
                entities: vec![],
                link_preview_options: None,
            }),
            reply_to_message: None,
            reply_markup: None,
            edit_date: None,
            author_signature: None,
            has_protected_content: false,
            is_automatic_forward: false,
            effect_id: None,
            forward_origin: None,
            external_reply: None,
            quote: None,
            reply_to_story: None,
            sender_boost_count: None,
            is_from_offline: false,
            business_connection_id: None,
        }),
        from: None,
        is_topic_message: false,
        sender_business_bot: None,
        sender_chat: None,
        thread_id: None,
        via_bot: None,
    }
}

// Helper to create a mock callback query
pub fn mock_callback_query<'a>(
    chat_id: ChatId,
    action: &CallbackAction<'a>,
) -> (Message, CallbackQuery) {
    let msg = mock_message(chat_id, "This is a message with a keyboard.");
    let query = CallbackQuery {
        id: "test_callback_id".to_string(),
        from: User {
            id: UserId(1),
            is_bot: false,
            first_name: "Test".to_string(),
            last_name: None,
            username: Some("testuser".to_string()),
            language_code: None,
            is_premium: false,
            added_to_attachment_menu: false,
        },
        message: Some(MaybeInaccessibleMessage::Regular(Box::new(msg.clone()))),
        inline_message_id: None,
        chat_instance: "test_instance".to_string(),
        data: Some(serde_json::to_string(action).unwrap()),
        game_short_name: None,
    };
    (msg, query)
}

pub fn setup_add_repo_mocks(mock_messaging: &mut MockMessagingService) {
    let status_msg = mock_message(CHAT_ID, "Processing... ⏳");
    mock_messaging
        .expect_send_text_message()
        .with(eq(CHAT_ID), eq("Processing... ⏳"))
        .times(1)
        .returning(move |_, _| Ok(status_msg.clone()));
}
