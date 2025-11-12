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
    println!("Analyzing how messages would be grouped\n");

    let db = get_connection(&db_path).map_err(|e| anyhow!(format!("{}", e)))?;

    let chat_data_cache = Chat::cache(&db).map_err(|e| anyhow!(format!("{}", e)))?;
    let handle_cache = Handle::cache(&db).map_err(|e| anyhow!(format!("{}", e)))?;

    let ralph_handles = vec![740, 713, 789, 801];

    // Simulate the grouping logic
    let mut chat_id_info: HashMap<i32, ChatInfo> = HashMap::new();

    Message::stream(&db, |message_result| {
        if let Ok(message) = message_result {
            if let Some(chat_id) = message.chat_id {
                let chat = chat_data_cache.get(&chat_id);
                let has_chat_name = chat.and_then(|c| c.display_name.as_ref()).is_some();

                // Only track direct messages (no chat name)
                if !has_chat_name {
                    let info = chat_id_info.entry(chat_id).or_insert_with(|| ChatInfo {
                        chat_id,
                        from_me: 0,
                        from_others: 0,
                        participants: Vec::new(),
                        involves_ralph: false,
                    });

                    if message.is_from_me {
                        info.from_me += 1;
                    } else {
                        info.from_others += 1;

                        // Track participant
                        if let Some(handle_id) = message.handle_id {
                            let name = handle_cache
                                .get(&handle_id)
                                .cloned()
                                .unwrap_or_else(|| format!("Handle {}", handle_id));

                            if !info.participants.contains(&name) {
                                info.participants.push(name.clone());
                            }

                            if ralph_handles.contains(&handle_id) {
                                info.involves_ralph = true;
                            }
                        }
                    }
                }
            }
        }
        Ok::<(), imessage_database::error::table::TableError>(())
    })
    .map_err(|e| anyhow!(format!("{}", e)))?;

    // Find Ralph-related chats
    let mut ralph_chats: Vec<_> = chat_id_info
        .iter()
        .filter(|(_, info)| info.involves_ralph)
        .collect();
    ralph_chats.sort_by_key(|(chat_id, _)| *chat_id);

    println!("=== Direct message chats involving Ralph ===\n");

    for (chat_id, info) in &ralph_chats {
        let chat = chat_data_cache.get(chat_id);
        let identifier = chat
            .map(|c| c.chat_identifier.as_str())
            .unwrap_or("[unknown]");

        println!("Chat ID {}: {}", chat_id, identifier);
        println!("  From me: {}", info.from_me);
        println!("  From others: {}", info.from_others);
        println!("  Participants: {:?}", info.participants);
        println!("  Participant set (sorted): {:?}", {
            let mut p = info.participants.clone();
            p.sort();
            p
        });
        println!();
    }

    // Now simulate the grouping logic
    println!("=== Simulating participant-based grouping ===\n");

    let mut participant_groups: HashMap<Vec<String>, Vec<i32>> = HashMap::new();

    for (chat_id, info) in &chat_id_info {
        if info.involves_ralph {
            let mut participants = info.participants.clone();
            participants.sort();
            participants.dedup();

            participant_groups
                .entry(participants)
                .or_default()
                .push(*chat_id);
        }
    }

    for (participants, chat_ids) in &participant_groups {
        let key = if participants.len() == 1 {
            format!("Direct: {}", participants[0])
        } else {
            format!("Direct: {}", participants.join(", "))
        };

        println!("Group: {}", key);
        println!("  Participant set: {:?}", participants);
        println!("  Chat IDs: {:?}", chat_ids);

        let total_from_me: usize = chat_ids
            .iter()
            .filter_map(|id| chat_id_info.get(id))
            .map(|info| info.from_me)
            .sum();
        let total_from_others: usize = chat_ids
            .iter()
            .filter_map(|id| chat_id_info.get(id))
            .map(|info| info.from_others)
            .sum();

        println!("  Total from me: {}", total_from_me);
        println!("  Total from others: {}", total_from_others);
        println!("  Total messages: {}", total_from_me + total_from_others);
        println!();
    }

    Ok(())
}

#[derive(Debug)]
struct ChatInfo {
    chat_id: i32,
    from_me: usize,
    from_others: usize,
    participants: Vec<String>,
    involves_ralph: bool,
}
