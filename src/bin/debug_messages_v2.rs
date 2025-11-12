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
    // ===== CUSTOMIZE SEARCH HERE =====
    // Change these search terms to look for different conversations
    let search_terms = vec!["ralph", "douglass"];
    // =================================

    let db_path = default_db_path();
    println!("Opening database: {}", db_path.display());
    println!("Searching for conversations matching: {:?}\n", search_terms);

    let db = get_connection(&db_path).map_err(|e| anyhow!(format!("{}", e)))?;

    // Cache chats and handles
    let chat_data_cache = Chat::cache(&db).map_err(|e| anyhow!(format!("{}", e)))?;
    let handle_cache = Handle::cache(&db).map_err(|e| anyhow!(format!("{}", e)))?;

    println!("=== Step 1: Finding matching handles ===\n");

    // Find all handles matching search terms
    let mut matching_handles: Vec<i32> = Vec::new();
    for (handle_id, handle_str) in &handle_cache {
        let matches = search_terms
            .iter()
            .any(|term| handle_str.to_lowercase().contains(term));
        if matches {
            matching_handles.push(*handle_id);
            println!("Found handle {}: {}", handle_id, handle_str);
        }
    }

    println!("\n=== Step 2: Finding chats by name ===\n");

    // Find chats by chat name/identifier
    let mut named_chats: Vec<(i32, String)> = Vec::new();
    for (chat_id, chat) in &chat_data_cache {
        let chat_identifier = &chat.chat_identifier;
        let display_name = chat.display_name.as_deref().unwrap_or("");

        let matches = search_terms.iter().any(|term| {
            chat_identifier.to_lowercase().contains(term)
                || display_name.to_lowercase().contains(term)
        });

        if matches {
            let name = if display_name.is_empty() {
                format!("Direct: {}", chat_identifier)
            } else {
                format!("{} ({})", display_name, chat_identifier)
            };
            named_chats.push((*chat_id, name));
            println!("Found chat {}: {}", chat_id, named_chats.last().unwrap().1);
        }
    }

    println!("\n=== Step 3: Scanning all messages ===\n");

    // Collect messages by chat_id, tracking which ones involve our handles
    let mut chat_message_counts: HashMap<i32, (usize, usize, usize)> = HashMap::new();
    let mut messages_by_chat: HashMap<i32, Vec<MessageDebugInfo>> = HashMap::new();
    let mut chats_by_handle: HashMap<i32, Vec<i32>> = HashMap::new(); // handle_id -> [chat_ids]

    let mut total_messages_scanned = 0;

    Message::stream(&db, |message_result| {
        if let Ok(message) = message_result {
            total_messages_scanned += 1;

            let chat_id = message.chat_id.unwrap_or(-1);

            // Check if this message involves a matching handle
            let involves_matching_handle = if message.is_from_me {
                false // Messages from me don't have a handle_id for the match
            } else if let Some(handle_id) = message.handle_id {
                matching_handles.contains(&handle_id)
            } else {
                false
            };

            // Check if this message is in a named chat we found
            let in_named_chat = named_chats.iter().any(|(id, _)| *id == chat_id);

            // Count this message if it matches our criteria
            if involves_matching_handle || in_named_chat {
                // Track which chats involve which handles
                if let Some(handle_id) = message.handle_id {
                    if matching_handles.contains(&handle_id) {
                        chats_by_handle.entry(handle_id).or_default().push(chat_id);
                    }
                }

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
                        chat_id,
                    });
                }
            }
        }
        Ok::<(), imessage_database::error::table::TableError>(())
    })
    .map_err(|e| anyhow!(format!("{}", e)))?;

    println!("Scanned {} total messages", total_messages_scanned);
    println!(
        "Found {} chats with matching messages\n",
        chat_message_counts.len()
    );

    println!("=== Step 4: Chat-to-Handle mapping ===\n");

    for (handle_id, chat_ids) in &chats_by_handle {
        let handle_str = handle_cache.get(handle_id).unwrap();
        let mut unique_chats: Vec<i32> = chat_ids.clone();
        unique_chats.sort();
        unique_chats.dedup();

        println!(
            "Handle {} ({}) appears in {} chats:",
            handle_id,
            handle_str,
            unique_chats.len()
        );
        for chat_id in unique_chats {
            let chat_name = chat_data_cache
                .get(&chat_id)
                .and_then(|c| c.display_name.as_ref())
                .cloned()
                .unwrap_or_else(|| {
                    chat_data_cache
                        .get(&chat_id)
                        .map(|c| format!("[{}]", c.chat_identifier))
                        .unwrap_or_else(|| "[unknown]".to_string())
                });
            println!("  Chat {}: {}", chat_id, chat_name);
        }
        println!();
    }

    println!("=== Message Statistics by Chat ===\n");

    let mut chat_stats: Vec<_> = chat_message_counts.iter().collect();
    chat_stats.sort_by_key(|(chat_id, _)| **chat_id);

    for (chat_id, (total, from_me, from_others)) in chat_stats {
        let chat_name = chat_data_cache
            .get(chat_id)
            .map(|c| {
                let display = c.display_name.as_deref().unwrap_or("");
                if display.is_empty() {
                    format!("Direct: {}", c.chat_identifier)
                } else {
                    format!("{} ({})", display, c.chat_identifier)
                }
            })
            .unwrap_or_else(|| "[unknown chat]".to_string());

        println!("Chat ID {}: {}", chat_id, chat_name);
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

    println!("=== Sample Messages (first 10 from each chat) ===\n");

    let mut chat_samples: Vec<_> = messages_by_chat.iter().collect();
    chat_samples.sort_by_key(|(chat_id, _)| **chat_id);

    for (chat_id, messages) in chat_samples {
        let chat_name = chat_data_cache
            .get(chat_id)
            .map(|c| {
                let display = c.display_name.as_deref().unwrap_or("");
                if display.is_empty() {
                    format!("Direct: {}", c.chat_identifier)
                } else {
                    format!("{} ({})", display, c.chat_identifier)
                }
            })
            .unwrap_or_else(|| "[unknown chat]".to_string());

        println!("Chat ID {}: {}", chat_id, chat_name);
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

    Ok(())
}

#[derive(Debug)]
struct MessageDebugInfo {
    guid: String,
    text: String,
    is_from_me: bool,
    handle_id: Option<i32>,
    date: i64,
    chat_id: i32,
}
