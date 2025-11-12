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
use std::collections::{HashMap, HashSet};

fn main() -> Result<()> {
    // ===== CUSTOMIZE SEARCH HERE =====
    // We'll look for messages from me that might be related to Ralph conversations
    let search_terms = vec!["ralph", "douglass"];
    // =================================

    let db_path = default_db_path();
    println!("Opening database: {}", db_path.display());
    println!(
        "Searching for YOUR messages that might relate to: {:?}\n",
        search_terms
    );

    let db = get_connection(&db_path).map_err(|e| anyhow!(format!("{}", e)))?;

    // Cache chats and handles
    let chat_data_cache = Chat::cache(&db).map_err(|e| anyhow!(format!("{}", e)))?;
    let handle_cache = Handle::cache(&db).map_err(|e| anyhow!(format!("{}", e)))?;

    println!("=== Step 1: Finding handles matching search terms ===\n");

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

    println!("\n=== Step 2: Finding chats that involve these handles ===\n");

    // Track which chat IDs have messages from matching handles
    let mut chats_with_matching_handles: HashSet<i32> = HashSet::new();

    Message::stream(&db, |message_result| {
        if let Ok(message) = message_result {
            if let Some(handle_id) = message.handle_id {
                if matching_handles.contains(&handle_id) {
                    if let Some(chat_id) = message.chat_id {
                        chats_with_matching_handles.insert(chat_id);
                    }
                }
            }
        }
        Ok::<(), imessage_database::error::table::TableError>(())
    })
    .map_err(|e| anyhow!(format!("{}", e)))?;

    println!(
        "Found {} chats that contain messages from matching handles",
        chats_with_matching_handles.len()
    );

    for chat_id in &chats_with_matching_handles {
        let chat_name = chat_data_cache
            .get(chat_id)
            .map(|c| {
                let display = c.display_name.as_deref().unwrap_or("");
                let identifier = &c.chat_identifier;
                if display.is_empty() {
                    format!("Chat {}: {}", chat_id, identifier)
                } else {
                    format!("Chat {}: {} ({})", chat_id, display, identifier)
                }
            })
            .unwrap_or_else(|| format!("Chat {}: [unknown]", chat_id));
        println!("  {}", chat_name);
    }

    println!("\n=== Step 3: Analyzing YOUR messages in these chats ===\n");

    // Now collect statistics about messages from YOU in these chats
    let mut my_messages_by_chat: HashMap<i32, Vec<MyMessageInfo>> = HashMap::new();
    let mut chat_stats: HashMap<i32, (usize, usize)> = HashMap::new(); // (from_me, from_others)

    Message::stream(&db, |message_result| {
        if let Ok(message) = message_result {
            if let Some(chat_id) = message.chat_id {
                if chats_with_matching_handles.contains(&chat_id) {
                    let (from_me, from_others) = chat_stats.entry(chat_id).or_insert((0, 0));

                    if message.is_from_me {
                        *from_me += 1;

                        // Store sample of your messages
                        let my_messages = my_messages_by_chat.entry(chat_id).or_default();
                        if my_messages.len() < 5 {
                            my_messages.push(MyMessageInfo {
                                text: message
                                    .text
                                    .as_deref()
                                    .unwrap_or("[no text]")
                                    .chars()
                                    .take(60)
                                    .collect(),
                                date: message.date,
                                handle_id: message.handle_id,
                                destination_caller_id: None, // Not exposed by imessage-database
                            });
                        }
                    } else {
                        *from_others += 1;
                    }
                }
            }
        }
        Ok::<(), imessage_database::error::table::TableError>(())
    })
    .map_err(|e| anyhow!(format!("{}", e)))?;

    println!("=== Message Statistics ===\n");

    let mut stats_vec: Vec<_> = chat_stats.iter().collect();
    stats_vec.sort_by_key(|(chat_id, _)| **chat_id);

    for (chat_id, (from_me, from_others)) in &stats_vec {
        let total = from_me + from_others;
        let chat_name = chat_data_cache
            .get(chat_id)
            .map(|c| {
                let display = c.display_name.as_deref().unwrap_or("");
                let identifier = &c.chat_identifier;
                if display.is_empty() {
                    identifier.to_string()
                } else {
                    format!("{} ({})", display, identifier)
                }
            })
            .unwrap_or_else(|| "[unknown]".to_string());

        println!("Chat {}: {}", chat_id, chat_name);
        println!("  Total: {}", total);
        println!(
            "  From you: {} ({:.1}%)",
            from_me,
            (*from_me as f64 / total as f64) * 100.0
        );
        println!(
            "  From others: {} ({:.1}%)",
            from_others,
            (*from_others as f64 / total as f64) * 100.0
        );

        if *from_me == 0 {
            println!("  ⚠️  WARNING: You have NO messages in this chat!");
            println!("  Your messages may be in a different chat_id");
        }
        println!();
    }

    println!("=== Sample of YOUR messages (first 5 from each chat) ===\n");

    let mut samples_vec: Vec<_> = my_messages_by_chat.iter().collect();
    samples_vec.sort_by_key(|(chat_id, _)| **chat_id);

    for (chat_id, messages) in &samples_vec {
        if messages.is_empty() {
            continue;
        }

        let chat_name = chat_data_cache
            .get(chat_id)
            .map(|c| {
                let display = c.display_name.as_deref().unwrap_or("");
                let identifier = &c.chat_identifier;
                if display.is_empty() {
                    identifier.to_string()
                } else {
                    format!("{} ({})", display, identifier)
                }
            })
            .unwrap_or_else(|| "[unknown]".to_string());

        println!("Chat {}: {}", chat_id, chat_name);
        for msg in *messages {
            println!(
                "  [ME] date={} handle_id={:?} text=\"{}\"",
                msg.date, msg.handle_id, msg.text
            );
        }
        println!();
    }

    println!("\n=== Summary ===\n");

    let chats_with_my_messages = my_messages_by_chat
        .iter()
        .filter(|(_, msgs)| !msgs.is_empty())
        .count();

    let chats_without_my_messages = chats_with_matching_handles.len() - chats_with_my_messages;

    println!(
        "Chats with Ralph handles: {}",
        chats_with_matching_handles.len()
    );
    println!("  - Chats with your messages: {}", chats_with_my_messages);
    println!(
        "  - Chats WITHOUT your messages: {}",
        chats_without_my_messages
    );

    if chats_without_my_messages > 0 {
        println!(
            "\n⚠️  CONCLUSION: Your messages are MISSING from {} chats!",
            chats_without_my_messages
        );
        println!("This means either:");
        println!("  1. Your messages are stored in completely different chat_id(s)");
        println!("  2. Your messages are lost/corrupted in the database");
        println!("  3. These chats only contain messages from Ralph to you");
    }

    Ok(())
}

#[derive(Debug)]
struct MyMessageInfo {
    text: String,
    date: i64,
    handle_id: Option<i32>,
    destination_caller_id: Option<String>,
}
