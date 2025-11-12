use super::contacts::ContactMap;
use imessage_database::tables::messages::Message;
use std::collections::HashMap;

#[derive(Hash, Eq, PartialEq)]
pub struct ResolvedHandle {
    id: i32,
    display: String,
}

impl ResolvedHandle {
    fn resolve_handle_to_name(
        handle_id: &i32,
        handle_cache: &HashMap<i32, String>,
        contact_map: &ContactMap,
    ) -> String {
        let unknown = "Unknown";

        match handle_cache.get(handle_id) {
            None => unknown,
            Some(contact_string) => match contact_map.get(contact_string) {
                None => contact_string,
                Some(better_contact_string) => better_contact_string,
            },
        }
        .to_owned()
    }

    pub fn from_message_sender(
        message: &Message,
        handle_cache: &HashMap<i32, String>,
        contact_map: &ContactMap,
    ) -> ResolvedHandle {
        let (id, display) = if message.is_from_me {
            (0, "Me".to_owned())
        } else if let Some(handle_id) = message.handle_id {
            (
                handle_id,
                ResolvedHandle::resolve_handle_to_name(&handle_id, handle_cache, contact_map),
            )
        } else {
            // When is_from_me is false but handle_id is None, this might be a bug
            // in the database where messages from me aren't properly marked.
            // In this case, we'll mark it as from an unknown sender rather than
            // incorrectly assuming it's from me.
            (-1, "Unknown".to_owned())
        };

        ResolvedHandle { id, display }
    }
}

impl std::fmt::Display for ResolvedHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display)
    }
}
