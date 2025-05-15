use std::str::FromStr;

use chrono::{DateTime, Utc};
use mockall::predicate::*;

use super::*;
use crate::{
    github::{GithubError, MockGithubClient},
    messaging::MockMessagingService,
    storage::{MockRepoStorage, RepoEntity},
};

const OWNER: &str = "owner";
const REPO_NAME: &str = "repo";
const REPO_NAME_WITH_OWNER: &str = "owner/repo";
const CHAT_ID: ChatId = ChatId(123);
const LAST_POLL_TIME: i64 = 1715817600_i64;

fn last_poll_time_system_time() -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(LAST_POLL_TIME as u64)
}

#[test]
fn test_filter_new_issues() {
    let last_poll_time_system_time = last_poll_time_system_time();
    let before_last_poll_time = last_poll_time_system_time - Duration::from_secs(1);
    let after_last_poll_time = last_poll_time_system_time + Duration::from_secs(1);

    let issues = vec![
        issues::IssuesRepositoryIssuesNodes {
            created_at: DateTime::<Utc>::from(after_last_poll_time).to_rfc3339(),
            ..Default::default()
        },
        issues::IssuesRepositoryIssuesNodes {
            created_at: DateTime::<Utc>::from(before_last_poll_time).to_rfc3339(),
            ..Default::default()
        },
    ];

    // Only 1 issue should be included
    let new_issues = GithubPoller::filter_new_issues(issues.clone(), &last_poll_time_system_time);

    assert_eq!(new_issues.len(), 1);
}

#[tokio::test]
async fn test_poll_user_repo_new_issues() {
    // Arrange
    let mut mock_github_client = MockGithubClient::new();
    let mut mock_repo_storage = MockRepoStorage::new();
    let mut mock_messaging_service = MockMessagingService::new();

    let last_poll_time_system_time = last_poll_time_system_time();
    let before_last_poll_time_system_time = last_poll_time_system_time - Duration::from_secs(1);
    let after_last_poll_time_system_time = last_poll_time_system_time + Duration::from_secs(1);

    // Create two issues: one new (after the last poll) and one old (before the last
    // poll)
    let issue_new = issues::IssuesRepositoryIssuesNodes {
        created_at: DateTime::<Utc>::from(after_last_poll_time_system_time).to_rfc3339(),
        ..Default::default()
    };
    let issue_old = issues::IssuesRepositoryIssuesNodes {
        created_at: DateTime::<Utc>::from(before_last_poll_time_system_time).to_rfc3339(),
        ..Default::default()
    };
    let issues = vec![issue_new.clone(), issue_old.clone()];

    mock_github_client
        .expect_repo_issues_by_label()
        .with(
            eq(OWNER),
            eq(REPO_NAME),
            function(|labels: &Vec<String>| *labels == GOOD_FIRST_ISSUE_LABELS.to_vec()),
        )
        .returning(move |_, _, _| Ok(issues.clone()));

    mock_repo_storage
        .expect_get_last_poll_time()
        .withf(|chat_id_param, repo| {
            *chat_id_param == CHAT_ID && repo.name_with_owner == REPO_NAME_WITH_OWNER
        })
        .returning(move |_, _| Ok(Some(LAST_POLL_TIME)));

    mock_messaging_service
        .expect_send_new_issues_msg()
        .withf(move |chat_id_param, repo_name_param, issues_list| {
            *chat_id_param == CHAT_ID
                && repo_name_param == REPO_NAME_WITH_OWNER
                && issues_list.len() == 1
                && issues_list[0].created_at == issue_new.created_at
        })
        .returning(|_, _, _| Ok(()));

    mock_repo_storage
        .expect_set_last_poll_time()
        .withf(|chat_id_param, repo| {
            *chat_id_param == CHAT_ID && repo.name_with_owner == REPO_NAME_WITH_OWNER
        })
        .returning(|_, _| Ok(()));

    let mut poller = GithubPoller::new(
        Arc::new(mock_github_client),
        Arc::new(mock_repo_storage),
        Arc::new(mock_messaging_service),
        10,
    );

    let repo = RepoEntity::from_str(REPO_NAME_WITH_OWNER).unwrap();

    // Act
    let result = poller.poll_user_repo(CHAT_ID, repo).await;

    // Assert
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_poll_user_repo_no_issues() {
    // Arrange
    let mut mock_github_client = MockGithubClient::new();
    let mut mock_repo_storage = MockRepoStorage::new();
    let mock_messaging_service = MockMessagingService::new();

    let last_poll_time_system_time = last_poll_time_system_time();
    let before_last_poll_time_system_time = last_poll_time_system_time - Duration::from_secs(1);

    let issue_old = issues::IssuesRepositoryIssuesNodes {
        created_at: DateTime::<Utc>::from(before_last_poll_time_system_time).to_rfc3339(),
        ..Default::default()
    };
    // Create two issues: both are old (before the last poll)
    let issues = vec![issue_old.clone(), issue_old];

    mock_github_client
        .expect_repo_issues_by_label()
        .with(
            eq(OWNER),
            eq(REPO_NAME),
            function(|labels: &Vec<String>| *labels == GOOD_FIRST_ISSUE_LABELS.to_vec()),
        )
        .returning(move |_, _, _| Ok(issues.clone()));

    mock_repo_storage
        .expect_get_last_poll_time()
        .withf(|chat_id_param, repo| {
            *chat_id_param == CHAT_ID && repo.name_with_owner == REPO_NAME_WITH_OWNER
        })
        .returning(move |_, _| Ok(Some(LAST_POLL_TIME)));

    mock_repo_storage
        .expect_set_last_poll_time()
        .withf(|chat_id_param, repo| {
            *chat_id_param == CHAT_ID && repo.name_with_owner == REPO_NAME_WITH_OWNER
        })
        .returning(|_, _| Ok(()));

    let mut poller = GithubPoller::new(
        Arc::new(mock_github_client),
        Arc::new(mock_repo_storage),
        Arc::new(mock_messaging_service),
        10,
    );

    let repo = RepoEntity::from_str(REPO_NAME_WITH_OWNER).unwrap();

    // Act
    let result = poller.poll_user_repo(CHAT_ID, repo).await;

    // Assert
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_poll_user_repo_github_error() {
    // Arrange
    let mut mock_github_client = MockGithubClient::new();
    let mut mock_repo_storage = MockRepoStorage::new();
    let mock_messaging_service = MockMessagingService::new();

    mock_github_client
        .expect_repo_issues_by_label()
        .returning(move |_, _, _| Err(GithubError::Unauthorized));

    mock_repo_storage
        .expect_get_last_poll_time()
        .withf(|chat_id_param, repo| {
            *chat_id_param == CHAT_ID && repo.name_with_owner == REPO_NAME_WITH_OWNER
        })
        .returning(move |_, _| Ok(Some(LAST_POLL_TIME)));

    let mut poller = GithubPoller::new(
        Arc::new(mock_github_client),
        Arc::new(mock_repo_storage),
        Arc::new(mock_messaging_service),
        10,
    );

    let repo = RepoEntity::from_str(REPO_NAME_WITH_OWNER).unwrap();

    // Act
    let result = poller.poll_user_repo(CHAT_ID, repo).await;

    // Assert
    // for now just log the error and keep going
    assert!(result.is_ok());
}
