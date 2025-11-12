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
    println!("Looking for chat that creates 'Direct: Ralph Douglass' with 773 messages\n");

    let db = get_connection(&db_path).map_err(|e| anyhow!(format!("{}", e)))?;

    // Cache chats and handles
    let chat_data_cache = Chat::cache(&db).map_err(|e| anyhow!(format!("{}", e)))?;
    let handle_cache = Handle::cache(&db).map_err(|e| anyhow!(format!("{}", e)))?;

    println!("=== Finding chats where Ralph Douglass appears as the only other participant ===\n");

    // Count messages per chat, tracking participants
    let mut chat_info: HashMap<i32, ChatInfo> = HashMap::new();

    Message::stream(&db, |message_result| {
        if let Ok(message) = message_result {
            if let Some(chat_id) = message.chat_id {
                let info = chat_info.entry(chat_id).or_insert_with(|| ChatInfo {
                    chat_id,
                    from_me_count: 0,
                    from_others_count: 0,
                    participants: HashMap::new(),
                });

                if message.is_from_me {
                    info.from_me_count += 1;
                } else {
                    info.from_others_count += 1;
                    if let Some(handle_id) = message.handle_id {
                        *info.participants.entry(handle_id).or_insert(0) += 1;
                    }
                }
            }
        }
        Ok::<(), imessage_database::error::table::TableError>(())
    })
    .map_err(|e| anyhow!(format!("{}", e)))?;

    println!("=== Analyzing {} chats ===\n", chat_info.len());

    // Find chats with exactly 773 from_others and Ralph as the participant
    let ralph_handles = vec![740, 713, 789, 801]; // From previous searches

    let mut candidates: Vec<_> = chat_info
        .iter()
        .filter(|(_, info)| {
            // Must have ~773 messages from others
            info.from_others_count >= 770 && info.from_others_count <= 776
        })
        .collect();

    candidates.sort_by_key(|(_, info)| info.from_others_count);

    println!(
        "Found {} chats with ~773 messages from others:\n",
        candidates.len()
    );

    for (chat_id, info) in &candidates {
        let chat = chat_data_cache.get(chat_id);
        let chat_name = chat
            .and_then(|c| c.display_name.as_ref().cloned())
            .unwrap_or_default();
        let chat_identifier = chat
            .map(|c| c.chat_identifier.as_str())
            .unwrap_or("[unknown]");

        println!("Chat ID {}: {} ({})", chat_id, chat_name, chat_identifier);
        println!("  From you: {}", info.from_me_count);
        println!("  From others: {}", info.from_others_count);
        println!("  Total: {}", info.from_me_count + info.from_others_count);
        println!("  Participants:");

        for (handle_id, count) in &info.participants {
            let handle_str = handle_cache
                .get(handle_id)
                .cloned()
                .unwrap_or_else(|| format!("Handle {}", handle_id));
            let is_ralph = ralph_handles.contains(handle_id);
            let marker = if is_ralph { " ← RALPH" } else { "" };
            println!(
                "    Handle {}: {} ({} messages){}",
                handle_id, handle_str, count, marker
            );
        }
        println!();
    }

    // Also show any chats that have Ralph as the ONLY participant
    println!("=== Chats with Ralph as the ONLY other participant ===\n");

    let mut ralph_only_chats: Vec<_> = chat_info
        .iter()
        .filter(|(_, info)| {
            // Only one participant
            info.participants.len() == 1 &&
            // That participant is Ralph
            info.participants.keys().any(|h| ralph_handles.contains(h))
        })
        .collect();

    ralph_only_chats.sort_by_key(|(_, info)| -(info.from_others_count as i32));

    println!(
        "Found {} chats with Ralph as sole participant:\n",
        ralph_only_chats.len()
    );

    for (chat_id, info) in &ralph_only_chats {
        let chat = chat_data_cache.get(chat_id);
        let chat_name = chat
            .and_then(|c| c.display_name.as_ref().cloned())
            .unwrap_or_default();
        let chat_identifier = chat
            .map(|c| c.chat_identifier.as_str())
            .unwrap_or("[unknown]");

        let handle_id = info.participants.keys().next().unwrap();
        let handle_str = handle_cache
            .get(handle_id)
            .cloned()
            .unwrap_or_else(|| format!("Handle {}", handle_id));

        println!("Chat ID {}: {} ({})", chat_id, chat_name, chat_identifier);
        println!("  From you: {}", info.from_me_count);
        println!(
            "  From others: {} (all from {})",
            info.from_others_count, handle_str
        );
        println!("  Total: {}", info.from_me_count + info.from_others_count);

        if info.from_others_count == 773 {
            println!("  ✓ THIS IS LIKELY THE 'Direct: Ralph Douglass' CONVERSATION!");
        }
        println!();
    }

    Ok(())
}

#[derive(Debug)]
struct ChatInfo {
    chat_id: i32,
    from_me_count: usize,
    from_others_count: usize,
    participants: HashMap<i32, usize>, // handle_id -> message count
}
