use super::clean_message::CleanMessage;
use super::resolved_handle::ResolvedHandle;
use imessage_database::message_types::variants::{Tapback, TapbackAction};
use std::collections::HashMap;

pub struct MessageStore(HashMap<String, CleanMessage>);

impl MessageStore {
    pub fn new() -> Self {
        MessageStore(HashMap::new())
    }

    pub fn insert(&mut self, message: CleanMessage) {
        self.0.insert(message.guid.clone(), message);
    }

    pub fn tapback(
        &mut self,
        message_id: String,
        tapback_action: TapbackAction,
        tapback_handle: ResolvedHandle,
        tapback: Tapback,
    ) {
        match self.0.get_mut(&message_id) {
            None => (),
            Some(message) => message.tapback(tapback_action, tapback_handle, tapback),
        }
    }

    // pub fn edit_message(&mut self, message_id: String) {
    //     match self.0.get_mut(&message_id) {
    //         None => (),
    //         Some(message) => message.
    //     }
    // }

    pub fn drain_to_sorted_vector(mut self) -> Vec<CleanMessage> {
        let mut vec = self.0.drain().map(|(_, m)| m).collect::<Vec<_>>();
        vec.sort_by(|a, b| a.date.cmp(&b.date));
        vec
    }
}
