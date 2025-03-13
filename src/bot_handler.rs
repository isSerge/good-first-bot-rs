use crate::github;
use crate::storage::{Repository, Storage};
use teloxide::utils::command::BotCommands;
use teloxide::{prelude::*, types::Message};

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Available commands:")]
pub enum Command {
    #[command(description = "Show this help text.")]
    Help,
    #[command(description = "Add a repository (e.g., owner/repo).")]
    Add(String),
    #[command(description = "Remove a repository.")]
    Remove(String),
    #[command(description = "List tracked repositories.")]
    List,
}

/// Encapsulates the bot, storage and GitHub client.
#[derive(Clone)]
pub struct BotHandler {
    github_client: github::GithubClient,
    storage: Storage,
    bot: Bot,
}

impl BotHandler {
    /// Creates a new `BotHandler` instance.
    pub fn new(github_client: github::GithubClient, storage: Storage, bot: Bot) -> Self {
        Self {
            github_client,
            storage,
            bot,
        }
    }

    /// Sends a text message to the provided chat.
    async fn send_response(&self, chat_id: ChatId, text: impl ToString) -> ResponseResult<()> {
        self.bot
            .send_message(chat_id, text.to_string())
            .await
            .map(|_| ())
    }

    async fn handle_add_command(&self, msg: &Message, repo: String) -> ResponseResult<()> {
        if repo.trim().is_empty() {
            return self
                .send_response(
                    msg.chat.id,
                    "Repository name cannot be empty. Please use format: owner/repo",
                )
                .await;
        }

        if let Some((owner, repo_name)) = parse_repo_name(&repo) {
            let repo_url = format!("https://github.com/{}/{}", owner, repo_name);
            let repo = Repository::new(format!("{}/{}", owner, repo_name), repo_url.clone());
            match self.github_client.repo_exists(owner, repo_name).await {
                Ok(true) => {
                    let mut storage_lock = self.storage.lock().await;
                    let repos = storage_lock.entry(msg.chat.id).or_default();
                    if repos.contains(&repo) {
                        self.send_response(
                            msg.chat.id,
                            format!(
                                "Repository {} is already in your list",
                                repo.name_with_owner
                            ),
                        )
                        .await?;
                    } else {
                        repos.push(repo);
                        self.send_response(
                            msg.chat.id,
                            format!("Added repo: {} ({})", repo_name, repo_url),
                        )
                        .await?;
                    }
                }
                Ok(false) => {
                    self.send_response(msg.chat.id, "Repository does not exist on GitHub.")
                        .await?;
                }
                Err(e) => {
                    self.send_response(msg.chat.id, format!("Error checking repository: {}", e))
                        .await?;
                }
            }
        } else {
            self.send_response(msg.chat.id, "Invalid repository format. Use owner/repo.")
                .await?;
        }
        Ok(())
    }

    /// Handles the List command.
    async fn handle_list_command(&self, msg: &Message) -> ResponseResult<()> {
        let storage_lock = self.storage.lock().await;
        let repos_msg = storage_lock
            .get(&msg.chat.id)
            .filter(|repos| !repos.is_empty())
            .map(|repos| {
                repos
                    .iter()
                    .map(|r| format!("{} ({})", r.name_with_owner, r.url))
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_else(|| "No repositories tracked.".to_string());

        self.send_response(
            msg.chat.id,
            format!("Your tracked repositories:\n{}", repos_msg),
        )
        .await
    }

    /// Handles the Remove command.
    async fn handle_remove_command(&self, msg: &Message, repo: String) -> ResponseResult<()> {
        let mut storage_lock = self.storage.lock().await;
        if let Some(repos) = storage_lock.get_mut(&msg.chat.id) {
            let initial_len = repos.len();
            repos.retain(|r| r.name_with_owner != repo);
            if repos.len() != initial_len {
                self.send_response(msg.chat.id, format!("Removed repo: {}", repo))
                    .await?;
            } else {
                self.send_response(msg.chat.id, format!("You are not tracking repo: {}", repo))
                    .await?;
            }
        } else {
            self.send_response(msg.chat.id, format!("You are not tracking repo: {}", repo))
                .await?;
        }
        Ok(())
    }

    /// Dispatches the incoming command to the appropriate handler.
    pub async fn handle_commands(&self, msg: Message, cmd: Command) -> ResponseResult<()> {
        match cmd {
            Command::Help => {
                self.send_response(msg.chat.id, Command::descriptions())
                    .await?;
            }
            Command::Add(repo) => {
                self.handle_add_command(&msg, repo).await?;
            }
            Command::Remove(repo) => {
                self.handle_remove_command(&msg, repo).await?;
            }
            Command::List => {
                self.handle_list_command(&msg).await?;
            }
        }
        Ok(())
    }
}

/// Parses a repository string in "owner/repo" format.
fn parse_repo_name(repo_name_with_owner: &str) -> Option<(&str, &str)> {
    repo_name_with_owner.split_once('/')
}
