use super::contacts::ContactMap;
use super::resolved_handle::ResolvedHandle;
use super::tapback_emoji::TapbackEmoji;
use anyhow::{Result, anyhow};
use chrono::{DateTime, Local, NaiveDate};
use imessage_database::message_types::variants::{Tapback, TapbackAction};
use imessage_database::tables::attachment::Attachment;
use imessage_database::tables::messages::Message;
use rusqlite::Connection;
use std::collections::HashMap;

pub struct CleanMessage {
    pub guid: String,
    pub text: String,
    pub from: ResolvedHandle,
    pub chat_name: Option<String>,
    pub date: DateTime<Local>,
    pub tapbacks: HashMap<ResolvedHandle, TapbackEmoji>,
    pub attachments: Vec<Attachment>,
}

impl CleanMessage {
    pub fn from_message(
        db: &Connection,
        handle_cache: &HashMap<i32, String>,
        contact_map: &ContactMap,
        chat_name: Option<String>,
        mut message: Message,
    ) -> Result<Self> {
        let database_tz_offset = imessage_database::util::dates::get_offset();

        // TODO: is this really a result that needs to be checked?
        let _: Result<_, _> = message.generate_text(db);

        let best_date = if message.date_delivered != 0 {
            message
                .date_delivered(&database_tz_offset)
                .expect("unable to calculate date_delivered")
        } else if message.date_read != 0 {
            message
                .date_read(&database_tz_offset)
                .expect("unable to calculate date_read")
        } else {
            message
                .date(&database_tz_offset)
                .expect("unable to calculate date written")
        };

        let from = ResolvedHandle::from_message_sender(&message, handle_cache, contact_map);

        let attachments = if message.has_attachments() {
            Attachment::from_message(db, &message).map_err(|e| anyhow!(format!("{}", e)))?
        } else {
            Vec::new()
        };

        Ok(Self {
            guid: message.guid,
            text: message.text.as_deref().unwrap_or_default().to_owned(),
            from,
            date: best_date,
            chat_name,
            tapbacks: HashMap::new(),
            attachments,
        })
    }

    pub fn tapback(
        &mut self,
        tapback_action: TapbackAction,
        tapback_handle: ResolvedHandle,
        tapback: Tapback,
    ) {
        let tapback_emoji = TapbackEmoji::from_message_tapback(tapback);
        match tapback_action {
            TapbackAction::Added => {
                let _ = self.tapbacks.insert(tapback_handle, tapback_emoji);
            }
            TapbackAction::Removed => {
                let _ = self.tapbacks.remove(&tapback_handle);
            }
        }
    }

    pub fn matches(
        &self,
        on_or_after: &Option<NaiveDate>,
        before: &Option<NaiveDate>,
        chat_names: &[String],
    ) -> bool {
        if let Some(on_or_after) = on_or_after
            && self.date.date_naive() < *on_or_after
        {
            return false;
        }
        if let Some(before) = before
            && self.date.date_naive() >= *before
        {
            return false;
        }
        if chat_names.is_empty() {
            true
        } else {
            match &self.chat_name {
                None => false,
                Some(message_chat_name) => chat_names.contains(message_chat_name),
            }
        }
    }
}

impl std::fmt::Display for CleanMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "From: {}\nDate: {}\n{}", self.from, self.date, self.text)?;
        if !self.attachments.is_empty() {
            writeln!(f, "Attachments:")?;
            for attachment in &self.attachments {
                writeln!(f, "  {:?}", attachment.filename())?
            }
        }
        if !self.tapbacks.is_empty() {
            writeln!(f, "Tapbacks:")?;

            for (handle, tapback_emoji) in &self.tapbacks {
                writeln!(f, "  {}: {}", handle, tapback_emoji)?
            }
        }
        Ok(())
    }
}
