mod utils;

use crate::github;
use crate::storage::{Repository, Storage};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use teloxide::dispatching::dialogue::{Dialogue, InMemStorage};
use teloxide::utils::command::BotCommands;
use teloxide::{
    prelude::*,
    types::{ForceReply, Message},
};

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

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub enum CommandState {
    #[default]
    None,
    WaitingForRepo {
        command: String,
    },
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

    /// Handles the Add command when the user provides a repository name.
    async fn handle_add_command(&self, msg: &Message, repo: String) -> ResponseResult<()> {
        if let Some((owner, repo_name)) = utils::parse_repo_name(&repo) {
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
        let user_repos = storage_lock.get(&msg.chat.id);

        if user_repos.is_none() || user_repos.unwrap().is_empty() {
            return self
                .send_response(msg.chat.id, "No repositories tracked.")
                .await;
        }

        let repos_msg = utils::format_tracked_repos(user_repos.unwrap());

        self.send_response(
            msg.chat.id,
            format!("Your tracked repositories:\n{}", repos_msg),
        )
        .await
    }

    /// Handles the Remove command when the user provides a repository name.
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
    pub async fn handle_commands(
        &self,
        msg: Message,
        cmd: Command,
        dialogue: Dialogue<CommandState, InMemStorage<CommandState>>,
    ) -> Result<()> {
        match cmd {
            Command::Help => {
                self.send_response(msg.chat.id, Command::descriptions())
                    .await?;
                dialogue.exit().await?;
            }
            Command::Add(repo) => {
                if repo.trim().is_empty() {
                    self.prompt_for_repo(msg.chat.id).await?;
                    // Save state indicating we're waiting for repo input for 'add'.
                    dialogue
                        .update(CommandState::WaitingForRepo {
                            command: "add".into(),
                        })
                        .await?;
                } else {
                    self.handle_add_command(&msg, repo).await?;
                    dialogue.exit().await?;
                }
            }
            Command::Remove(repo) => {
                if repo.trim().is_empty() {
                    self.prompt_for_repo(msg.chat.id).await?;
                    dialogue
                        .update(CommandState::WaitingForRepo {
                            command: "remove".into(),
                        })
                        .await?;
                } else {
                    self.handle_remove_command(&msg, repo).await?;
                    dialogue.exit().await?;
                }
            }
            Command::List => {
                self.handle_list_command(&msg).await?;
                dialogue.exit().await?;
            }
        }
        Ok(())
    }

    /// Handle a reply message when we're waiting for repository input.
    pub async fn handle_reply(
        &self,
        msg: Message,
        dialogue: Dialogue<CommandState, InMemStorage<CommandState>>,
    ) -> Result<()> {
        // Check if we're waiting for repository input.
        match dialogue.get().await? {
            Some(CommandState::WaitingForRepo { command }) if msg.text().is_some() => {
                let text = msg.text().unwrap().to_string();
                match command.as_str() {
                    "add" => self.handle_add_command(&msg, text).await?,
                    "remove" => self.handle_remove_command(&msg, text).await?,
                    _ => self.send_response(msg.chat.id, "Unknown command").await?,
                }
                dialogue.exit().await?;
            }
            Some(_) => {
                self.send_response(msg.chat.id, "No text found in your reply.")
                    .await?;
                dialogue.exit().await?;
            }
            None => {
                // Do nothing
            }
        }
        Ok(())
    }

    /// Prompts the user for repository input if there was no repository provided initially.
    async fn prompt_for_repo(&self, chat_id: ChatId) -> ResponseResult<()> {
        let prompt = "Please reply with the repository in the format owner/repo.";
        self.bot
            .send_message(chat_id, prompt)
            .reply_markup(ForceReply::new())
            .await
            .map(|_| ())
    }
}
