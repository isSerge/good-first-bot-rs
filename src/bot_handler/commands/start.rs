use crate::bot_handler::commands::CommandContext;
use anyhow::Result;

pub async fn handle(ctx: CommandContext<'_>) -> Result<()> {
    let welcome_text =
        "Welcome! Use the buttons below to track repositories with good first issues.";
    ctx.handler
        .messaging_service
        .send_response_with_keyboard(ctx.message.chat.id, welcome_text.to_string(), None)
        .await?;
    Ok(())
}
