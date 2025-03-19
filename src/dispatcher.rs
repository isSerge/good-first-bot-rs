use std::sync::Arc;

use anyhow::Result;
use teloxide::{
    dispatching::{
        DefaultKey, DpHandlerDescription,
        dialogue::{Dialogue, InMemStorage},
    },
    dptree::{deps, filter_map},
    prelude::*,
    types::Update,
    utils::command::BotCommands,
};

use crate::bot_handler::{BotHandler, Command, CommandState};

/// Type alias to simplify handler type signatures.
type BotResultHandler = Handler<'static, DependencyMap, Result<()>, DpHandlerDescription>;

/// Encapsulates the dispatcher logic for the bot.
pub struct BotDispatcher {
    handler: Arc<BotHandler>,
    dialogue_storage: Arc<InMemStorage<CommandState>>,
}

impl BotDispatcher {
    /// Creates a new `BotDispatcher`.
    pub fn new(
        handler: Arc<BotHandler>,
        dialogue_storage: Arc<InMemStorage<CommandState>>,
    ) -> Self {
        Self { handler, dialogue_storage }
    }

    /// Builds the dispatcher using the provided `bot` instance.
    #[must_use = "This function returns a Dispatcher that should not be ignored"]
    pub fn build(&self, bot: Bot) -> Dispatcher<Bot, anyhow::Error, DefaultKey> {
        Dispatcher::builder(
            bot,
            dptree::entry()
                .branch(self.build_commands_branch())
                .branch(self.build_callback_queries_branch())
                .branch(self.build_force_reply_branch()),
        )
        .dependencies(deps![self.dialogue_storage.clone(), self.handler.clone()])
        .enable_ctrlc_handler()
        .build()
    }

    /// Builds the branch for handling text commands.
    fn build_commands_branch(&self) -> BotResultHandler {
        Update::filter_message()
            .filter_command::<Command>()
            .chain(filter_map(extract_dialogue))
            .endpoint(
                |msg: Message,
                 cmd: Command,
                 dialogue: Dialogue<CommandState, InMemStorage<CommandState>>,
                 handler: Arc<BotHandler>| async move {
                    handler.handle_commands(&msg, cmd, dialogue).await?;
                    Ok(())
                },
            )
    }

    /// Builds the branch for handling callback queries using combinators to
    /// reduce nesting.
    fn build_callback_queries_branch(&self) -> BotResultHandler {
        Update::filter_callback_query().chain(filter_map(extract_dialogue)).endpoint(
            |query: CallbackQuery,
             dialogue: Dialogue<CommandState, InMemStorage<CommandState>>,
             handler: Arc<BotHandler>| async move {
                // If the callback query is a remove query, handle it as a callback query.
                if let Some(data) = query.data.as_deref() {
                    if data.starts_with("remove:") {
                        handler.handle_remove_callback_query(query).await?;
                        return Ok(());
                    }
                }

                // If the callback query is not a remove query, handle it as a command.
                let maybe_tuple =
                    query.message.as_ref().and_then(|m| m.regular_message().cloned()).and_then(
                        |msg| {
                            query.data.as_deref().and_then(|data| {
                                let cmd_str = format!("/{}", data);
                                Command::parse(&cmd_str, "botname").ok().map(|cmd| (msg, cmd))
                            })
                        },
                    );

                if let Some((msg, command)) = maybe_tuple {
                    handler.handle_commands(&msg, command, dialogue).await?;
                }
                Ok(())
            },
        )
    }

    /// Builds the branch for handling messages that are force-reply responses.
    fn build_force_reply_branch(&self) -> BotResultHandler {
        Update::filter_message()
            .filter(|msg: Message| msg.reply_to_message().is_some())
            .chain(filter_map(extract_dialogue))
            .endpoint(
                |msg: Message,
                 dialogue: Dialogue<CommandState, InMemStorage<CommandState>>,
                 handler: Arc<BotHandler>| async move {
                    handler.handle_reply(&msg, &dialogue).await
                },
            )
    }
}

/// Extracts a dialogue from an update using the provided dialogue storage.
fn extract_dialogue(
    update: Update,
    storage: Arc<InMemStorage<CommandState>>,
) -> Option<Dialogue<CommandState, InMemStorage<CommandState>>> {
    update.chat().map(|chat| Dialogue::new(storage, chat.id))
}
