use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

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

async fn handle_commands(bot: Bot, msg: Message, cmd: Command, storage: Storage) -> ResponseResult<()> {
    match cmd {
        Command::Help => {
            bot.send_message(msg.chat.id, Command::descriptions().to_string())
                .await?;
        }
        Command::Add(repo) => {
            {
                let mut storage = storage.lock().unwrap();
                storage.entry(msg.chat.id)
                    .or_insert_with(Vec::new)
                    .push(repo.clone());
            }
            bot.send_message(msg.chat.id, format!("Added repo: {}", repo))
                .await?;
        }
        Command::Remove(repo) => {
            let removed = {
                let mut storage = storage.lock().unwrap();
                if let Some(repos) = storage.get_mut(&msg.chat.id) {
                    repos.retain(|r| r != &repo);
                    true
                } else {
                    false
                }
            };
            
            if removed {
                bot.send_message(msg.chat.id, format!("Removed repo: {}", repo)).await?;
            } else {
                bot.send_message(msg.chat.id, "You don't have any repositories tracked.").await?;
            }
        }
        Command::List => {
            let repos = {
                let storage = storage.lock().unwrap();
                storage.get(&msg.chat.id)
                    .map(|repos| repos.join("\n"))
                    .unwrap_or_else(|| "No repositories tracked.".to_string())
            };
            bot.send_message(msg.chat.id, format!("Your tracked repositories:\n{}", repos))
                .await?;
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    // store repo list for each user
    let storage: Storage = Arc::new(Mutex::new(HashMap::new()));
    let bot = Bot::from_env();

    let handler = dptree::entry()
        .branch(Update::filter_message()
            .filter_command::<Command>()
            .endpoint(|bot: Bot, msg: Message, cmd: Command, storage: Storage| async move {
                handle_commands(bot, msg, cmd, storage).await
            }));

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![storage])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}
