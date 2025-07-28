use std::{cell::RefCell, collections::HashSet, str::FromStr, sync::Arc};

use chrono::Utc;
use mockall::predicate::*;
use teloxide::{
    dispatching::dialogue::{Dialogue, SqliteStorage, serializer},
    types::{
        Chat, ChatId, ChatKind, ChatPrivate, MaybeInaccessibleMessage, MediaKind, MediaText,
        Message, MessageCommon, MessageId, MessageKind, User,
    },
};

use super::*;
use crate::{
    bot_handler::{BotHandler, Command, CommandState, commands::add::AddSummary},
    github::GithubError,
    messaging::MockMessagingService,
    pagination::Paginated,
    repository::{LabelNormalized, MockRepositoryService, RepositoryServiceError},
    storage::{RepoEntity, StorageError},
};

const CHAT_ID: ChatId = ChatId(123);
type DialogueStorage = SqliteStorage<serializer::Json>;

// Test harness to encapsulate common test setup and actions.
struct TestHarness {
    bot_handler: BotHandler,
    dialogue: Dialogue<CommandState, DialogueStorage>,
    storage: Arc<DialogueStorage>,
}

impl TestHarness {
    // Creates a new TestHarness with mock services.
    async fn new(
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
    fn new_dialogue(&self) -> Dialogue<CommandState, DialogueStorage> {
        Dialogue::new(self.storage.clone(), CHAT_ID)
    }

    // Simulates handling a command message.
    async fn handle_command_with_dialogue(
        &self,
        command: Command,
        dialogue: Dialogue<CommandState, DialogueStorage>,
    ) -> Result<(), BotHandlerError> {
        let msg = mock_message(CHAT_ID, &format!("/{}", command.to_string()));
        self.bot_handler.handle_commands(&msg, command, dialogue).await
    }

    // Simulates handling a reply message.
    async fn handle_reply_with_dialogue(
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
    async fn handle_callback_with_dialogue<'a>(
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
    async fn handle_callback<'a>(
        &self,
        action: &CallbackAction<'a>,
    ) -> Result<(), BotHandlerError> {
        self.handle_callback_with_dialogue(action, self.dialogue.clone()).await
    }

    // Simulates handling a reply for the 'add' command specifically.
    async fn handle_add_reply(&self, text: &str) -> Result<(), BotHandlerError> {
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
fn str_hashset(items: &[&str]) -> HashSet<String> {
    items.iter().map(|s| s.to_string()).collect()
}

// Helper function to create a HashSet of (String, String) tuples
fn str_tuple_hashset(items: &[(&str, &str)]) -> HashSet<(String, String)> {
    items.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect()
}

// Helper to create a mock teloxide message to reduce boilerplate in tests
fn mock_message(chat_id: ChatId, text: &str) -> Message {
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
fn mock_callback_query<'a>(
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

fn setup_add_repo_mocks(mock_messaging: &mut MockMessagingService) {
    let status_msg = mock_message(CHAT_ID, "Processing... ⏳");
    mock_messaging
        .expect_send_text_message()
        .with(eq(CHAT_ID), eq("Processing... ⏳"))
        .times(1)
        .returning(move |_, _| Ok(status_msg.clone()));
}

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
        .withf(move |&id, entity| id == CHAT_ID && entity.name_with_owner == repo_name_with_owner)
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
        .withf(move |&id, entity| id == CHAT_ID && entity.name_with_owner == repo_name_with_owner)
        .times(1)
        .returning(|_, _| Ok(false));

    let expected_summary =
        AddSummary { already_tracked: str_hashset(&[repo_name_with_owner]), ..Default::default() };
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

    mock_repository.expect_repo_exists().with(eq("owner"), eq("new")).returning(|_, _| Ok(true));
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
    mock_messaging.expect_prompt_for_repo_input().with(eq(CHAT_ID)).times(1).returning(|_| Ok(()));

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
            if *call_count.borrow() == 1 {
                Ok(initial_labels.clone())
            } else {
                Ok(updated_labels.clone())
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
    assert!(matches!(state, Some(CommandState::None)), "Dialogue state should be reset to None");
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
            cid == CHAT_ID && matches!(e, BotHandlerError::InvalidInput(s) if s == "Invalid state")
        })
        .times(1)
        .returning(|_, _| Ok(()));

    let harness = TestHarness::new(mock_messaging, mock_repository).await;
    harness.dialogue.update(CommandState::None).await.unwrap();

    // Act
    let result = harness.handle_reply_with_dialogue("some random text", &harness.dialogue).await;

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
    assert!(matches!(state, Some(CommandState::None)), "Dialogue state should be reset to None");
}

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
        .update(CommandState::ViewingRepoLabels { repo_id: "owner/repo".to_string(), from_page: 1 })
        .await
        .unwrap();
    let action = CallbackAction::BackToRepoList(page);

    // Act
    let result = harness.handle_callback(&action).await;

    // Assert
    assert!(result.is_ok());
    let state = harness.dialogue.get().await.unwrap();
    assert!(matches!(state, Some(CommandState::None)), "Dialogue state should be reset to None");
}
