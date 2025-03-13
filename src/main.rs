#![warn(missing_docs)]
//! A Telegram bot for tracking beginner-friendly GitHub issues.
//!
//! This bot allows users to track repositories and receive notifications for new issues labeled as "good first issue".
//! It provides a simple interface to add, remove, and list tracked repositories.

use std::collections::HashMap;
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

async fn handle_commands(
    bot: Bot,
    msg: Message,
    cmd: Command,
    storage: Storage,
) -> ResponseResult<()> {
    match cmd {
        Command::Help => {
            bot.send_message(msg.chat.id, Command::descriptions().to_string())
                .await?;
        }
        Command::Add(repo) => {
            if repo.trim().is_empty() {
                bot.send_message(
                    msg.chat.id,
                    "Repository name cannot be empty. Please use format: owner/repo",
                )
                .await?;
                return Ok(());
            }
            let mut storage_lock = storage.lock().await;
            let repos = storage_lock.entry(msg.chat.id).or_insert_with(Vec::new);
            if repos.contains(&repo) {
                bot.send_message(
                    msg.chat.id,
                    format!("Repository {} is already in your list", repo),
                )
                .await?;
            } else {
                repos.push(repo.clone());
                bot.send_message(msg.chat.id, format!("Added repo: {}", repo))
                    .await?;
            }
        }
        Command::Remove(repo) => {
            let mut storage_lock = storage.lock().await;
            if let Some(repos) = storage_lock.get_mut(&msg.chat.id) {
                let initial_len = repos.len();
                repos.retain(|r| r != &repo);
                if repos.len() != initial_len {
                    bot.send_message(msg.chat.id, format!("Removed repo: {}", repo))
                        .await?;
                } else {
                    bot.send_message(msg.chat.id, format!("You are not tracking repo: {}", repo))
                        .await?;
                }
            } else {
                bot.send_message(msg.chat.id, format!("You are not tracking repo: {}", repo))
                    .await?;
            }
        }
        Command::List => {
            let storage_lock = storage.lock().await;
            let repos_msg = storage_lock
                .get(&msg.chat.id)
                .map(|repos| repos.join("\n"))
                .unwrap_or_else(|| "No repositories tracked.".to_string());
            bot.send_message(
                msg.chat.id,
                format!("Your tracked repositories:\n{}", repos_msg),
            )
            .await?;
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    env_logger::init();

    let storage: Storage = Arc::new(Mutex::new(HashMap::new()));
    let bot = Bot::from_env();

    let handler = dptree::entry().branch(
        Update::filter_message()
            .filter_command::<Command>()
            .endpoint(
                |bot: Bot, msg: Message, cmd: Command, storage: Storage| async move {
                    handle_commands(bot, msg, cmd, storage).await
                },
            ),
    );

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![storage])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}
