use lazy_static::lazy_static;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

use super::utils;
use crate::{bot_handler::CallbackAction, repository::LabelNormalized, storage::RepoEntity};

pub fn build_repo_list_keyboard(repos: &[RepoEntity]) -> InlineKeyboardMarkup {
    let buttons: Vec<Vec<InlineKeyboardButton>> = repos
        .iter()
        .map(|repo| {
            // define callback action
            let action =
                utils::serialize_action(&CallbackAction::ViewRepoDetails(&repo.name_with_owner));

            // Repository name with link
            vec![InlineKeyboardButton::callback(repo.name_with_owner.clone(), action)]
        })
        .collect();

    InlineKeyboardMarkup::new(buttons)
}

pub fn build_repo_item_keyboard(repo: &RepoEntity) -> InlineKeyboardMarkup {
    let id = &repo.name_with_owner;
    // actions
    let back_to_list = utils::serialize_action(&CallbackAction::BackToRepoList);
    let repo_labels = utils::serialize_action(&CallbackAction::ViewRepoLabels(id));
    let remove_repo = utils::serialize_action(&CallbackAction::RemoveRepoPrompt(id));

    // buttons
    let buttons = vec![
        // Back to list button
        vec![InlineKeyboardButton::callback("🔙 List".to_string(), back_to_list)],
        // Manage repo labels button
        vec![InlineKeyboardButton::callback("⚙️ Labels".to_string(), repo_labels)],
        // Remove repo action
        vec![InlineKeyboardButton::callback("❌ Remove".to_string(), remove_repo)],
    ];

    InlineKeyboardMarkup::new(buttons)
}

pub fn build_repo_labels_keyboard(
    labels: &[LabelNormalized],
    id: &str, // repo name with owner
) -> InlineKeyboardMarkup {
    let label_buttons = labels
        .iter()
        .map(|label| {
            // define callback action
            let toggle_action = utils::serialize_action(&CallbackAction::TL(id, &label.name));

            vec![InlineKeyboardButton::callback(
                format!(
                    "{} {} {}({})",
                    if label.is_selected { "✅ " } else { "" },
                    utils::github_color_to_emoji(&label.color),
                    label.name,
                    label.count,
                ),
                toggle_action,
            )]
        })
        .collect::<Vec<_>>();

    // Prepend the back button to the list of buttons
    let go_back = utils::serialize_action(&CallbackAction::BackToRepoDetails(id));
    let mut buttons = vec![vec![InlineKeyboardButton::callback("🔙 Back".to_string(), go_back)]];
    buttons.extend(label_buttons);

    InlineKeyboardMarkup::new(buttons)
}

lazy_static! {
    pub static ref COMMAND_KEYBOARD: InlineKeyboardMarkup = InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback(
            "ℹ️ Help",
            utils::serialize_action(&CallbackAction::Help)
        ),],
        vec![InlineKeyboardButton::callback(
            "📜 Tracked repositories",
            utils::serialize_action(&CallbackAction::List)
        ),],
        vec![InlineKeyboardButton::callback(
            "➕ Add repository",
            utils::serialize_action(&CallbackAction::Add)
        ),],
    ]);
}
