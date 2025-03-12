use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;

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

async fn handle_commands(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
    // In a real app, you'd update a persistent data store here.
    match cmd {
        Command::Help => {
            bot.send_message(msg.chat.id, Command::descriptions().to_string())
                .await?;
        }
        Command::Add(repo) => {
            // Add repo to user's list
            bot.send_message(msg.chat.id, format!("Added repo: {}", repo))
                .await?;
        }
        Command::Remove(repo) => {
            // Remove repo from user's list
            bot.send_message(msg.chat.id, format!("Removed repo: {}", repo))
                .await?;
        }
        Command::List => {
            // Retrieve and show the list of tracked repos
            bot.send_message(msg.chat.id, "Your tracked repositories: ...")
                .await?;
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    let bot = Bot::from_env();

    Command::repl(bot, handle_commands).await;
}
