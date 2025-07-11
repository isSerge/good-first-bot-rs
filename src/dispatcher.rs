use std::sync::Arc;

use teloxide::{
    dispatching::{
        DefaultKey, DpHandlerDescription,
        dialogue::{Dialogue, SqliteStorage, serializer::Json},
    },
    dptree::{deps, filter_map},
    prelude::*,
    types::Update,
};

use crate::bot_handler::{BotHandler, BotHandlerError, BotHandlerResult, Command, CommandState};

type DispatchHandler = Handler<'static, DependencyMap, BotHandlerResult<()>, DpHandlerDescription>;
type DialogueStorage = SqliteStorage<Json>;

/// Encapsulates the dispatcher logic for the bot.
pub struct BotDispatcher {
    handler: Arc<BotHandler>,
    dialogue_storage: Arc<DialogueStorage>,
}

impl BotDispatcher {
    /// Creates a new `BotDispatcher`.
    pub fn new(handler: Arc<BotHandler>, dialogue_storage: Arc<DialogueStorage>) -> Self {
        Self { handler, dialogue_storage }
    }

    /// Builds the dispatcher using the provided `bot` instance.
    #[must_use = "This function returns a Dispatcher that should not be ignored"]
    pub fn build(&self, bot: Bot) -> Dispatcher<Bot, BotHandlerError, DefaultKey> {
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
    fn build_commands_branch(&self) -> DispatchHandler {
        Update::filter_message()
            .filter_command::<Command>()
            .chain(filter_map(extract_dialogue))
            .endpoint(
                |msg: Message,
                 cmd: Command,
                 dialogue: Dialogue<CommandState, DialogueStorage>,
                 handler: Arc<BotHandler>| async move {
                    handler.handle_commands(&msg, cmd, dialogue).await?;
                    Ok(())
                },
            )
    }

    /// Builds the branch for handling callback queries using combinators to
    /// reduce nesting.
    fn build_callback_queries_branch(&self) -> DispatchHandler {
        Update::filter_callback_query().chain(filter_map(extract_dialogue)).endpoint(
            |query: CallbackQuery,
             dialogue: Dialogue<CommandState, DialogueStorage>,
             handler: Arc<BotHandler>| async move {
                handler.handle_callback_query(&query, dialogue).await
            },
        )
    }

    /// Builds the branch for handling messages that are force-reply responses.
    fn build_force_reply_branch(&self) -> DispatchHandler {
        Update::filter_message()
            .filter(|msg: Message| msg.reply_to_message().is_some())
            .chain(filter_map(extract_dialogue))
            .endpoint(
                |msg: Message,
                 dialogue: Dialogue<CommandState, DialogueStorage>,
                 handler: Arc<BotHandler>| async move {
                    handler.handle_reply(&msg, &dialogue).await
                },
            )
    }
}

/// Extracts a dialogue from an update using the provided dialogue storage.
fn extract_dialogue(
    update: Update,
    storage: Arc<DialogueStorage>,
) -> Option<Dialogue<CommandState, DialogueStorage>> {
    update.chat().map(|chat| Dialogue::new(storage.clone(), chat.id))
}
