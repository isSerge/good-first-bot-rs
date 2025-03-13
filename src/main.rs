#![warn(missing_docs)]
//! A Telegram bot for tracking beginner-friendly GitHub issues.
//!
//! This bot allows users to track repositories and receive notifications for new issues labeled as "good first issue".
//! It provides a simple interface to add, remove, and list tracked repositories.

mod github;

use anyhow::Context;
use log::debug;
use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;
use tokio::sync::Mutex;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Available commands:")]
enum Command {
    #[command(description = "Show this help text.")]
    Help,
    #[command(description = "Add a repository (e.g., owner/repo).")]
    Add(String),
    #[command(description = "Remove a repository.")]
    Remove(String),
    #[command(description = "List tracked repositories.")]
    List,
}

type Storage = Arc<Mutex<HashMap<ChatId, Vec<String>>>>;

async fn send_response(bot: &Bot, chat_id: ChatId, text: impl ToString) -> ResponseResult<()> {
  bot.send_message(chat_id, text.to_string()).await?;
  Ok(())
}

fn parse_repo_name(repo_name_with_owner: &str) -> Option<(&str, &str)> {
  repo_name_with_owner.split_once('/')
}

async fn handle_add_command(
  bot: &Bot,
  msg: &Message,
  repo: String,
  storage: &Storage,
  github_client: &github::GithubClient,
) -> ResponseResult<()> {
  if repo.trim().is_empty() {
      return send_response(bot, msg.chat.id, "Repository name cannot be empty. Please use format: owner/repo").await;
  }

  match parse_repo_name(&repo) {
      Some((owner, repo_name)) => match github_client.repo_exists(owner, repo_name).await {
          Ok(true) => {
              let mut storage_lock = storage.lock().await;
              let repos = storage_lock.entry(msg.chat.id).or_default();
              if repos.contains(&repo) {
                  send_response(bot, msg.chat.id, format!("Repository {} is already in your list", repo)).await
              } else {
                  repos.push(repo.clone());
                  send_response(bot, msg.chat.id, format!("Added repo: {}", repo)).await
              }
          }
          Ok(false) => send_response(bot, msg.chat.id, "Repository does not exist on GitHub.").await,
          Err(e) => send_response(bot, msg.chat.id, format!("Error checking repository: {}", e)).await,
      },
      None => send_response(bot, msg.chat.id, "Invalid repository format. Use owner/repo.").await,
  }
}

async fn handle_commands(
  bot: Bot,
  msg: Message,
  cmd: Command,
  storage: Storage,
  github_client: Arc<github::GithubClient>,
) -> ResponseResult<()> {
  match cmd {
      Command::Help => {
          send_response(&bot, msg.chat.id, Command::descriptions()).await?;
      }
      Command::Add(repo) => {
          handle_add_command(&bot, &msg, repo, &storage, &github_client).await?;
      }
      Command::Remove(repo) => {
          let mut storage_lock = storage.lock().await;
          if let Some(repos) = storage_lock.get_mut(&msg.chat.id) {
              let initial_len = repos.len();
              repos.retain(|r| r != &repo);
              if repos.len() != initial_len {
                  send_response(&bot, msg.chat.id, format!("Removed repo: {}", repo)).await?;
              } else {
                  send_response(&bot, msg.chat.id, format!("You are not tracking repo: {}", repo)).await?;
              }
          } else {
              send_response(&bot, msg.chat.id, format!("You are not tracking repo: {}", repo)).await?;
          }
      }
      Command::List => {
          let storage_lock = storage.lock().await;
          let repos_msg = storage_lock
              .get(&msg.chat.id)
              .map(|repos| repos.join("\n"))
              .unwrap_or_else(|| "No repositories tracked.".to_string());
          send_response(&bot, msg.chat.id, format!("Your tracked repositories:\n{}", repos_msg)).await?;
      }
  }
  Ok(())
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    env_logger::init();

    if let Err(err) = run().await {
        eprintln!("Error: {}", &err);
        std::process::exit(1);
    }
}

async fn run() -> anyhow::Result<()> {
    let storage: Storage = Arc::new(Mutex::new(HashMap::new()));
    let bot = Bot::from_env();
    let github_token =
        env::var("GITHUB_TOKEN").context("GITHUB_TOKEN environment variable is required")?;
    debug!("GitHub token retrieved successfully.");

    let github_client = Arc::new(
        github::GithubClient::new(github_token).context("Failed to create GitHub client")?,
    );
    debug!("GitHub client created successfully.");

    let handler = dptree::entry().branch(
        Update::filter_message()
            .filter_command::<Command>()
            .endpoint(
                |bot: Bot,
                 msg: Message,
                 cmd: Command,
                 storage: Storage,
                 github_client: Arc<github::GithubClient>| async move {
                    handle_commands(bot, msg, cmd, storage, github_client).await
                },
            ),
    );

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![storage, github_client])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
    debug!("Dispatcher built successfully.");

    Ok(())
}
