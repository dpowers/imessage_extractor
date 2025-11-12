use anyhow::{Result, anyhow};
use imessage_database::{
    tables::{
        chat::Chat,
        handle::Handle,
        messages::Message,
        table::{Cacheable, Table, get_connection},
    },
    util::dirs::default_db_path,
};
use std::collections::HashMap;

fn main() -> Result<()> {
    let db_path = default_db_path();
    println!("Opening database: {}", db_path.display());

    let db = get_connection(&db_path).map_err(|e| anyhow!(format!("{}", e)))?;

    // Cache chats and handles
    let chat_data_cache = Chat::cache(&db).map_err(|e| anyhow!(format!("{}", e)))?;
    let handle_cache = Handle::cache(&db).map_err(|e| anyhow!(format!("{}", e)))?;

    println!("\n=== Looking for Ralph Douglass conversations ===\n");

    // Find all chats involving Ralph
    let mut ralph_chats: Vec<(i32, String)> = Vec::new();
    for (chat_id, chat) in &chat_data_cache {
        let chat_identifier = &chat.chat_identifier;
        let display_name = chat.display_name.as_deref().unwrap_or("");

        if chat_identifier.to_lowercase().contains("ralph")
            || chat_identifier.to_lowercase().contains("douglass")
            || display_name.to_lowercase().contains("ralph")
            || display_name.to_lowercase().contains("douglass")
        {
            ralph_chats.push((*chat_id, format!("{} ({})", display_name, chat_identifier)));
        }
    }

    // Also check handle cache for Ralph's contact info
    let mut ralph_handles: Vec<i32> = Vec::new();
    for (handle_id, handle_str) in &handle_cache {
        if handle_str.to_lowercase().contains("ralph")
            || handle_str.to_lowercase().contains("douglass")
        {
            ralph_handles.push(*handle_id);
            println!("Found Ralph handle: {} -> {}", handle_id, handle_str);
        }
    }

    println!("\nFound {} chats with Ralph:", ralph_chats.len());
    for (chat_id, name) in &ralph_chats {
        println!("  Chat ID {}: {}", chat_id, name);
    }

    // Now collect messages from these chats
    let mut chat_message_counts: HashMap<i32, (usize, usize, usize)> = HashMap::new();
    let mut messages_by_chat: HashMap<i32, Vec<MessageDebugInfo>> = HashMap::new();

    Message::stream(&db, |message_result| {
        if let Ok(message) = message_result {
            // Check if this message is in a Ralph chat
            if let Some(chat_id) = message.chat_id {
                if ralph_chats.iter().any(|(id, _)| *id == chat_id) {
                    let (total, from_me, from_others) =
                        chat_message_counts.entry(chat_id).or_insert((0, 0, 0));
                    *total += 1;

                    if message.is_from_me {
                        *from_me += 1;
                    } else {
                        *from_others += 1;
                    }

                    // Store first 10 messages for detailed inspection
                    let messages = messages_by_chat.entry(chat_id).or_insert_with(Vec::new);
                    if messages.len() < 10 {
                        messages.push(MessageDebugInfo {
                            guid: message.guid.clone(),
                            text: message
                                .text
                                .as_deref()
                                .unwrap_or("[no text]")
                                .chars()
                                .take(50)
                                .collect(),
                            is_from_me: message.is_from_me,
                            handle_id: message.handle_id,
                            date: message.date,
                        });
                    }
                }
            }
        }
        Ok::<(), imessage_database::error::table::TableError>(())
    })
    .map_err(|e| anyhow!(format!("{}", e)))?;

    println!("\n=== Message Statistics by Chat ===\n");

    for (chat_id, name) in &ralph_chats {
        if let Some((total, from_me, from_others)) = chat_message_counts.get(chat_id) {
            println!("Chat ID {}: {}", chat_id, name);
            println!("  Total messages: {}", total);
            println!(
                "  From me: {} ({:.1}%)",
                from_me,
                (*from_me as f64 / *total as f64) * 100.0
            );
            println!(
                "  From others: {} ({:.1}%)",
                from_others,
                (*from_others as f64 / *total as f64) * 100.0
            );

            if *from_me == 0 && *total > 10 {
                println!(
                    "  ⚠️  WARNING: No messages marked as from_me in this chat with {} messages!",
                    total
                );
                println!("  This indicates a database issue with is_from_me field");
            }

            println!();
        }
    }

    println!("\n=== Sample Messages (first 10 from each chat) ===\n");

    for (chat_id, name) in &ralph_chats {
        if let Some(messages) = messages_by_chat.get(chat_id) {
            println!("Chat ID {}: {}", chat_id, name);
            for msg in messages {
                let sender = if msg.is_from_me {
                    "ME".to_string()
                } else if let Some(handle_id) = msg.handle_id {
                    handle_cache
                        .get(&handle_id)
                        .cloned()
                        .unwrap_or_else(|| format!("Handle {}", handle_id))
                } else {
                    "UNKNOWN".to_string()
                };

                println!(
                    "  [{}] is_from_me={} handle_id={:?} date={} text=\"{}...\"",
                    sender, msg.is_from_me, msg.handle_id, msg.date, msg.text
                );
            }
            println!();
        }
    }

    Ok(())
}

#[derive(Debug)]
struct MessageDebugInfo {
    guid: String,
    text: String,
    is_from_me: bool,
    handle_id: Option<i32>,
    date: i64,
}
