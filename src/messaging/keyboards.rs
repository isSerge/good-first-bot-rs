use lazy_static::lazy_static;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

use super::utils;
use crate::{
    bot_handler::CallbackAction, pagination::Paginated, repository::LabelNormalized,
    storage::RepoEntity,
};

pub fn build_repo_list_keyboard(paginated_repos: &Paginated<RepoEntity>) -> InlineKeyboardMarkup {
    let mut buttons: Vec<Vec<InlineKeyboardButton>> = paginated_repos
        .get_page_items()
        .iter()
        .map(|repo| {
            // define callback action
            let action =
                utils::serialize_action(&CallbackAction::ViewRepoDetails(&repo.name_with_owner));

            // Repository name with link
            vec![InlineKeyboardButton::callback(repo.name_with_owner.clone(), action)]
        })
        .collect();

    // Add navigation buttons if there are more pages
    let mut nav_buttons = Vec::new();

    if paginated_repos.has_prev() {
        let prev_action =
            utils::serialize_action(&CallbackAction::ListReposPage(paginated_repos.page - 1));
        nav_buttons.push(InlineKeyboardButton::callback("◀️ Previous".to_string(), prev_action));
    }
    if paginated_repos.has_next() {
        let next_action =
            utils::serialize_action(&CallbackAction::ListReposPage(paginated_repos.page + 1));
        nav_buttons.push(InlineKeyboardButton::callback("Next ▶️".to_string(), next_action));
    }

    if !nav_buttons.is_empty() {
        buttons.push(nav_buttons);
    }

    InlineKeyboardMarkup::new(buttons)
}

pub fn build_repo_item_keyboard(repo: &RepoEntity) -> InlineKeyboardMarkup {
    let id = &repo.name_with_owner;
    // actions
    let back_to_list = utils::serialize_action(&CallbackAction::BackToRepoList);
    let repo_labels = utils::serialize_action(&CallbackAction::ViewRepoLabels(id, 1));
    let remove_repo = utils::serialize_action(&CallbackAction::RemoveRepoPrompt(id));

    // buttons
    let buttons = vec![
        // Back to list button
        vec![InlineKeyboardButton::callback("🔙 Repository list".to_string(), back_to_list)],
        // Manage repo labels button
        vec![InlineKeyboardButton::callback("⚙️ Labels".to_string(), repo_labels)],
        // Remove repo action
        vec![InlineKeyboardButton::callback("❌ Remove".to_string(), remove_repo)],
    ];

    InlineKeyboardMarkup::new(buttons)
}

pub fn build_repo_labels_keyboard(
    paginated_labels: &Paginated<LabelNormalized>,
    id: &str, // repo name with owner
) -> InlineKeyboardMarkup {
    let label_buttons = paginated_labels
        .get_page_items()
        .iter()
        .map(|label| {
            // define callback action
            let toggle_action = utils::serialize_action(&CallbackAction::ToggleLabel(
                &label.name,
                paginated_labels.page,
            ));

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
    let go_back_repo = utils::serialize_action(&CallbackAction::BackToRepoDetails(id));
    let go_back_list = utils::serialize_action(&CallbackAction::BackToRepoList);
    let mut buttons = vec![vec![
        InlineKeyboardButton::callback("🔙 Back to repository".to_string(), go_back_repo),
        InlineKeyboardButton::callback("🔙 Back to list".to_string(), go_back_list),
    ]];

    // Add the label buttons to the main buttons
    buttons.extend(label_buttons);

    // Add navigation buttons if there are more pages
    let mut nav_buttons = Vec::new();

    if paginated_labels.has_prev() {
        let prev_action =
            utils::serialize_action(&CallbackAction::ViewRepoLabels(id, paginated_labels.page - 1));
        nav_buttons.push(InlineKeyboardButton::callback("◀️ Previous".to_string(), prev_action));
    }

    if paginated_labels.has_next() {
        let next_action =
            utils::serialize_action(&CallbackAction::ViewRepoLabels(id, paginated_labels.page + 1));
        nav_buttons.push(InlineKeyboardButton::callback("Next ▶️".to_string(), next_action));
    }

    if !nav_buttons.is_empty() {
        buttons.push(nav_buttons);
    }

    InlineKeyboardMarkup::new(buttons)
}

lazy_static! {
    pub static ref COMMAND_KEYBOARD: InlineKeyboardMarkup = InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback(
            "ℹ️ Help",
            utils::serialize_action(&CallbackAction::CmdHelp)
        ),],
        vec![InlineKeyboardButton::callback(
            "📋 Overview",
            utils::serialize_action(&CallbackAction::CmdOverview)
        ),],
        vec![InlineKeyboardButton::callback(
            "⚙️ Manage repositories",
            utils::serialize_action(&CallbackAction::CmdList)
        ),],
        vec![InlineKeyboardButton::callback(
            "➕ Add repository",
            utils::serialize_action(&CallbackAction::CmdAdd)
        ),],
    ]);
}
