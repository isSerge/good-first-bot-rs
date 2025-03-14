use crate::bot_handler::{BotHandler, Command, CommandState};
use anyhow::{Error, Ok};
use std::sync::Arc;
use teloxide::{
    dispatching::{
        DefaultKey, DpHandlerDescription,
        dialogue::{Dialogue, InMemStorage},
    },
    dptree::{deps, filter_map},
    prelude::*,
    types::Update,
};

/// Type alias to simplify handler type signatures.
type BotResultHandler = Handler<'static, DependencyMap, Result<(), Error>, DpHandlerDescription>;

pub struct BotDispatcher {
    handler: Arc<BotHandler>,
    dialogue_storage: Arc<InMemStorage<CommandState>>,
}

impl BotDispatcher {
    pub fn new(
        handler: Arc<BotHandler>,
        dialogue_storage: Arc<InMemStorage<CommandState>>,
    ) -> Self {
        Self {
            handler,
            dialogue_storage,
        }
    }

    pub fn build(&self, bot: Bot) -> Dispatcher<Bot, Error, DefaultKey> {
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
                    handler.handle_commands(msg, cmd, dialogue).await?;
                    Ok(())
                },
            )
    }

    /// Builds the branch for handling callback queries.
    fn build_callback_queries_branch(&self) -> BotResultHandler {
        Update::filter_callback_query()
            .chain(filter_map(extract_dialogue))
            .endpoint(
                |query: CallbackQuery,
                 dialogue: Dialogue<CommandState, InMemStorage<CommandState>>,
                 handler: Arc<BotHandler>| async move {
                    if let Some(msg) = query.message.as_ref().and_then(|m| m.regular_message()) {
                        if let Some(data) = query.data.as_deref() {
                            if let Some(command) = parse_callback_command(data) {
                                handler
                                    .handle_commands(msg.clone(), command, dialogue)
                                    .await?;
                            }
                        }
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
                    handler.handle_reply(msg, dialogue).await
                },
            )
    }
}

/// Helper that extracts a dialogue from an update using the provided dialogue storage.
fn extract_dialogue(
    update: Update,
    storage: Arc<InMemStorage<CommandState>>,
) -> Option<Dialogue<CommandState, InMemStorage<CommandState>>> {
    update.chat().map(|chat| Dialogue::new(storage, chat.id))
}

/// Helper that converts callback data into a corresponding command.
fn parse_callback_command(data: &str) -> Option<Command> {
    match data {
        "help" => Some(Command::Help),
        "list" => Some(Command::List),
        "add" => Some(Command::Add(String::new())),
        "remove" => Some(Command::Remove(String::new())),
        _ => None,
    }
}
