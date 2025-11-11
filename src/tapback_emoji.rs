use imessage_database::message_types::variants::Tapback;

pub struct TapbackEmoji(String);

impl TapbackEmoji {
    pub fn from_message_tapback(tapback: Tapback) -> Self {
        use Tapback::*;
        let emoji = match tapback {
            Loved => "🩷",
            Liked => "👍",
            Disliked => "👎",
            Laughed => "😂",
            Emphasized => "‼️",
            Questioned => "❓",
            Emoji(emoji) => emoji.unwrap_or_default(),
            Sticker => "🎨",
        };
        Self(emoji.to_string())
    }
}

impl std::fmt::Display for TapbackEmoji {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
