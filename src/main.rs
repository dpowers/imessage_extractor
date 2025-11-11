mod clean_message;
mod contacts;
mod html_output;
mod message_store;
mod resolved_handle;
mod tapback_emoji;

use anyhow::{Result, anyhow};
use chrono::NaiveDate;
use clean_message::CleanMessage;
use contacts::ContactMap;
use gumdrop::Options;
use html_output::HtmlOutput;
use imessage_database::{
    error::table::TableError,
    tables::{
        chat::Chat,
        handle::Handle,
        messages::Message,
        table::{Cacheable, Table, get_connection},
    },
    util::dirs::default_db_path,
};
use message_store::MessageStore;
use resolved_handle::ResolvedHandle;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Options)]
struct Args {
    #[options(help = "Limit export to messages on or after this date")]
    start_date: Option<NaiveDate>,
    #[options(help = "Limit export to messages before this date")]
    end_date: Option<NaiveDate>,
    #[options(
        help = "Chat to export. Defaults to all if no --chat flag given.  May be given multiple times"
    )]
    chat: Vec<String>,
    #[options(help = "Override the default database path")]
    database_path: Option<PathBuf>,
    #[options(help = "Output directory for HTML and attachments (default: output)")]
    output_directory: Option<PathBuf>,
    #[options(help = "print help message")]
    help: bool,
}

impl Args {
    pub fn database_path(&self) -> PathBuf {
        match &self.database_path {
            None => default_db_path(),
            Some(path) => path.clone(),
        }
    }

    pub fn output_directory(&self) -> PathBuf {
        match &self.output_directory {
            None => PathBuf::from("output"),
            Some(path) => path.clone(),
        }
    }
}

fn resolve_chat_name(
    message: &Message,
    chat_data_cache: &HashMap<i32, Chat>,
    contact_map: &ContactMap,
) -> Option<String> {
    match message.chat_id {
        None => None,
        Some(chat_id) => {
            let chat = chat_data_cache
                .get(&chat_id)
                .expect("Unable to find chat data for a chat id");

            if let Some(display_name) = chat.display_name.as_ref()
                && !display_name.is_empty()
            {
                Some(display_name.clone())
            } else {
                Some(
                    contact_map
                        .get(&chat.chat_identifier)
                        .unwrap_or(&chat.chat_identifier)
                        .clone(),
                )
            }
        }
    }
}

fn collect_messages(args: &Args) -> Result<MessageStore> {
    let db = get_connection(&args.database_path()).map_err(|e| anyhow!(format!("{}", e)))?;

    let chat_data_cache = Chat::cache(&db).map_err(|e| anyhow!(format!("{}", e)))?;
    let handle_cache = Handle::cache(&db).map_err(|e| anyhow!(format!("{}", e)))?;
    let contact_map = ContactMap::fetch()?;

    let mut message_store = MessageStore::new();

    // Iterate over a stream of messages
    Message::stream(&db, |message_result| {
        match message_result {
            Ok(message) => {
                use imessage_database::message_types::variants::Variant::*;
                match message.variant() {
                    Normal => {
                        let chat_name = resolve_chat_name(&message, &chat_data_cache, &contact_map);

                        let clean_message = CleanMessage::from_message(
                            &db,
                            &handle_cache,
                            &contact_map,
                            chat_name,
                            message,
                        )
                        .expect("unable to clean message");

                        if clean_message.matches(&args.start_date, &args.end_date, &args.chat) {
                            message_store.insert(clean_message)
                        }
                    }
                    Edited => (),
                    Tapback(_body_id, action, tapback) => {
                        if let Some((_, associated_id)) = message.clean_associated_guid() {
                            let tapback_handle = ResolvedHandle::from_message_sender(
                                &message,
                                &handle_cache,
                                &contact_map,
                            );
                            message_store.tapback(
                                associated_id.to_string(),
                                action,
                                tapback_handle,
                                tapback,
                            );
                        }
                    }
                    App(_) | SharePlay | Vote | PollUpdate | Unknown(_) => (),
                }
            }
            Err(e) => return Err(e),
        };

        Ok::<(), TableError>(())
    })
    .map_err(|e| anyhow!(format!("{}", e)))?;

    Ok(message_store)
}

fn main() -> Result<()> {
    let args = Args::parse_args_default_or_exit();

    let database_path = args.database_path();
    let output_directory = args.output_directory();

    // Check if output directory already exists
    if output_directory.exists() {
        return Err(anyhow!(
            "Output directory '{}' already exists. Please remove it or specify a different output directory with --output-directory",
            output_directory.display()
        ));
    }

    let message_store = collect_messages(&args)?;

    // Collect messages for all chats
    let chat_messages: Vec<_> = message_store.drain_to_sorted_vector();

    // Generate HTML output (which will also save attachments)
    if !chat_messages.is_empty() {
        let html_generator = HtmlOutput::new(chat_messages, database_path);
        html_generator.generate(output_directory.to_str().unwrap())?;
    }

    Ok(())
}
