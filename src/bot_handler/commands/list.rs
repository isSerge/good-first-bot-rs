use crate::bot_handler::commands::CommandContext;
use crate::storage::Repository;
use anyhow::Result;
use std::collections::HashSet;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};
use url::Url;

fn build_repo_list_keyboard(repos: &HashSet<Repository>) -> InlineKeyboardMarkup {
    let buttons: Vec<Vec<InlineKeyboardButton>> = repos
        .iter()
        .map(|repo| {
            vec![
                // Left button: repository name (with a no-op or details callback)
                InlineKeyboardButton::url(
                    repo.name_with_owner.clone(),
                    Url::parse(&repo.url()).expect("Failed to parse repository URL"),
                ),
                // Right button: remove action
                InlineKeyboardButton::callback(
                    "❌".to_string(),
                    format!("remove:{}", repo.name_with_owner),
                ),
            ]
        })
        .collect();

    InlineKeyboardMarkup::new(buttons)
}

pub async fn handle(ctx: CommandContext<'_>) -> Result<()> {
    let user_repos = ctx
        .handler
        .repository_service
        .get_user_repos(ctx.message.chat.id)
        .await?;

    if user_repos.is_empty() {
        return ctx
            .handler
            .messaging_service
            .send_response_with_keyboard(
                ctx.message.chat.id,
                "No repositories tracked.".to_string(),
                None,
            )
            .await;
    }

    let keyboard = build_repo_list_keyboard(&user_repos);

    ctx.handler
        .messaging_service
        .send_response_with_keyboard(
            ctx.message.chat.id,
            "Your tracked repositories:".to_string(),
            Some(keyboard),
        )
        .await
}
