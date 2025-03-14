use crate::bot_handler::commands::CommandContext;
use anyhow::Result;

pub async fn handle(ctx: CommandContext<'_>) -> Result<()> {
    let welcome_text =
        "Welcome! Use the buttons below to track repositories with good first issues.";
    ctx.handler
        .send_response(ctx.message.chat.id, welcome_text)
        .await?;
    Ok(())
}
