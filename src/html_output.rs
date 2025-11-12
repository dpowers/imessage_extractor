use crate::clean_message::CleanMessage;
use anyhow::Result;
use imessage_database::util::platform::Platform;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

pub struct HtmlOutput {
    messages: Vec<CleanMessage>,
    database_path: PathBuf,
}

impl HtmlOutput {
    pub fn new(messages: Vec<CleanMessage>, database_path: PathBuf) -> Self {
        Self {
            messages,
            database_path,
        }
    }

    pub fn generate(&self, output_dir: &str) -> Result<()> {
        // Group messages by chat
        let grouped_messages = self.group_messages_by_chat();

        // Save all attachments first
        self.save_attachments(output_dir)?;

        // Generate individual chat HTML files in subdirectories
        for (chat_key, chat_messages) in &grouped_messages {
            let is_group = !chat_key.starts_with("Direct: ");
            let subdir = if is_group { "groups" } else { "direct" };
            self.generate_chat_html(output_dir, subdir, chat_key, chat_messages)?;
        }

        // Generate index page
        self.generate_index_html(output_dir, &grouped_messages)?;

        Ok(())
    }

    fn group_messages_by_chat(&self) -> HashMap<String, Vec<&CleanMessage>> {
        let mut grouped: HashMap<String, Vec<&CleanMessage>> = HashMap::new();
        let mut chat_id_to_name: HashMap<i32, String> = HashMap::new();

        for message in &self.messages {
            let chat_key = match &message.chat_name {
                Some(name) => name.clone(),
                None => {
                    // For direct messages without a chat name, use chat_id to group
                    if let Some(chat_id) = message.chat_id {
                        // Get or create a name for this chat_id
                        chat_id_to_name
                            .entry(chat_id)
                            .or_insert_with(|| {
                                // Find the first non-"Me" participant in this chat
                                self.messages
                                    .iter()
                                    .filter(|m| m.chat_id == Some(chat_id))
                                    .map(|m| m.from.to_string())
                                    .find(|name| name != "Me")
                                    .map(|name| format!("Direct: {}", name))
                                    .unwrap_or_else(|| format!("Direct: Unknown ({})", chat_id))
                            })
                            .clone()
                    } else {
                        // Fallback if no chat_id is available
                        format!("Direct: {}", message.from)
                    }
                }
            };

            grouped.entry(chat_key).or_default().push(message);
        }

        grouped
    }

    fn generate_index_html(
        &self,
        output_dir: &str,
        grouped_messages: &HashMap<String, Vec<&CleanMessage>>,
    ) -> Result<()> {
        let mut chat_entries: Vec<_> = grouped_messages
            .iter()
            .map(|(chat_key, messages)| {
                let message_count = messages.len();
                let latest_date = messages
                    .iter()
                    .map(|m| m.date)
                    .max()
                    .expect("No messages in chat");
                let is_group = !chat_key.starts_with("Direct: ");

                // Collect unique participants (excluding "Me")
                let mut participants: Vec<String> = messages
                    .iter()
                    .map(|m| m.from.to_string())
                    .filter(|name| name != "Me")
                    .collect();
                participants.sort();
                participants.dedup();

                (chat_key, message_count, latest_date, is_group, participants)
            })
            .collect();

        // Sort alphabetically by chat name for easier finding
        chat_entries.sort_by(|a, b| a.0.cmp(b.0));

        // Separate into groups and direct messages
        let mut group_chats: Vec<_> = chat_entries.iter().filter(|e| e.3).collect();
        let mut direct_chats: Vec<_> = chat_entries.iter().filter(|e| !e.3).collect();

        // Sort each category by name
        group_chats.sort_by(|a, b| a.0.cmp(b.0));
        direct_chats.sort_by(|a, b| a.0.cmp(b.0));

        let mut html = String::new();
        html.push_str(&format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>iMessage Chats</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif;
            max-width: 900px;
            margin: 0 auto;
            padding: 20px;
            background-color: #f5f5f5;
        }}

        h1 {{
            text-align: center;
            color: #333;
            border-bottom: 2px solid #007aff;
            padding-bottom: 10px;
            margin-bottom: 20px;
        }}

        .search-box {{
            margin-bottom: 20px;
            padding: 12px;
            background: white;
            border-radius: 12px;
            box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
        }}

        #searchInput {{
            width: 100%;
            padding: 10px;
            font-size: 1em;
            border: 2px solid #e5e5ea;
            border-radius: 8px;
            box-sizing: border-box;
        }}

        #searchInput:focus {{
            outline: none;
            border-color: #007aff;
        }}

        .stats {{
            text-align: center;
            margin-bottom: 20px;
            color: #666;
            font-size: 0.9em;
        }}

        .category-header {{
            background-color: #f9f9f9;
            padding: 12px 20px;
            font-weight: 600;
            color: #333;
            border-bottom: 2px solid #e5e5ea;
            font-size: 0.95em;
            text-transform: uppercase;
            letter-spacing: 0.5px;
        }}

        .chat-list {{
            background: white;
            border-radius: 12px;
            box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
            overflow: hidden;
            margin-bottom: 20px;
        }}

        .chat-item {{
            display: block;
            padding: 16px 20px;
            border-bottom: 1px solid #e5e5ea;
            text-decoration: none;
            color: inherit;
            transition: background-color 0.2s;
        }}

        .chat-item:last-child {{
            border-bottom: none;
        }}

        .chat-item:hover {{
            background-color: #f9f9f9;
        }}

        .chat-name {{
            font-size: 1.1em;
            font-weight: 600;
            color: #000;
            margin-bottom: 4px;
        }}

        .chat-info {{
            font-size: 0.9em;
            color: #666;
            display: flex;
            justify-content: space-between;
        }}

        .chat-members {{
            font-size: 0.85em;
            color: #888;
            margin-top: 4px;
            font-style: italic;
        }}

        .message-count {{
            color: #007aff;
        }}

        .hidden {{
            display: none;
        }}
    </style>
</head>
<body>
    <h1>iMessage Chats</h1>

    <div class="search-box">
        <input type="text" id="searchInput" placeholder="Search chats by name..." onkeyup="filterChats()">
    </div>

    <div class="stats">
        <span id="totalChats">{}</span> total chats
        (<span id="groupCount">{}</span> groups, <span id="directCount">{}</span> direct messages)
    </div>
"#, chat_entries.len(), group_chats.len(), direct_chats.len()));

        // Output group chats
        if !group_chats.is_empty() {
            html.push_str(
                r#"    <div class="chat-list">
        <div class="category-header">Group Chats</div>
"#,
            );

            for (chat_key, message_count, latest_date, _, participants) in group_chats {
                let filename = format!("groups/{}.html", self.sanitize_filename(chat_key));
                let members_str = participants.join(", ");
                let search_text = format!("{} {}", chat_key, members_str).to_lowercase();

                html.push_str(&format!(
                    r#"        <a href="{}" class="chat-item" data-search="{}">
            <div class="chat-name">{}</div>
"#,
                    filename,
                    self.html_escape(&search_text),
                    self.html_escape(chat_key)
                ));

                if !participants.is_empty() {
                    html.push_str(&format!(
                        r#"            <div class="chat-members">{}</div>
"#,
                        self.html_escape(&members_str)
                    ));
                }

                html.push_str(&format!(
                    r#"            <div class="chat-info">
                <span class="message-count">{} messages</span>
                <span class="latest-date">{}</span>
            </div>
        </a>
"#,
                    message_count,
                    latest_date.format("%b %d, %Y")
                ));
            }

            html.push_str(
                r#"    </div>
"#,
            );
        }

        // Output direct messages
        if !direct_chats.is_empty() {
            html.push_str(
                r#"    <div class="chat-list">
        <div class="category-header">Direct Messages</div>
"#,
            );

            for (chat_key, message_count, latest_date, _, participants) in direct_chats {
                let filename = format!("direct/{}.html", self.sanitize_filename(chat_key));
                // Remove "Direct: " prefix for display
                let display_name = chat_key.strip_prefix("Direct: ").unwrap_or(chat_key);
                let members_str = participants.join(", ");
                let search_text = format!("{} {}", display_name, members_str).to_lowercase();

                html.push_str(&format!(
                    r#"        <a href="{}" class="chat-item" data-search="{}">
            <div class="chat-name">{}</div>
"#,
                    filename,
                    self.html_escape(&search_text),
                    self.html_escape(display_name)
                ));

                if !participants.is_empty() {
                    html.push_str(&format!(
                        r#"            <div class="chat-members">{}</div>
"#,
                        self.html_escape(&members_str)
                    ));
                }

                html.push_str(&format!(
                    r#"            <div class="chat-info">
                <span class="message-count">{} messages</span>
                <span class="latest-date">{}</span>
            </div>
        </a>
"#,
                    message_count,
                    latest_date.format("%b %d, %Y")
                ));
            }

            html.push_str(
                r#"    </div>
"#,
            );
        }

        // Add JavaScript for search functionality
        html.push_str(
            r#"
    <script>
        function filterChats() {
            const searchInput = document.getElementById('searchInput');
            const filter = searchInput.value.toLowerCase();
            const chatItems = document.querySelectorAll('.chat-item');

            let visibleCount = 0;
            chatItems.forEach(function(item) {
                const searchText = item.getAttribute('data-search');
                if (searchText.includes(filter)) {
                    item.classList.remove('hidden');
                    visibleCount++;
                } else {
                    item.classList.add('hidden');
                }
            });

            // Hide empty categories
            const chatLists = document.querySelectorAll('.chat-list');
            chatLists.forEach(function(list) {
                const visibleItems = list.querySelectorAll('.chat-item:not(.hidden)');
                if (visibleItems.length === 0) {
                    list.classList.add('hidden');
                } else {
                    list.classList.remove('hidden');
                }
            });
        }
    </script>
</body>
</html>
"#,
        );

        let index_path = format!("{}/index.html", output_dir);
        fs::write(&index_path, html)?;

        Ok(())
    }

    fn generate_chat_html(
        &self,
        output_dir: &str,
        subdir: &str,
        chat_key: &str,
        messages: &[&CleanMessage],
    ) -> Result<()> {
        // Create subdirectory
        let chat_dir = format!("{}/{}", output_dir, subdir);
        fs::create_dir_all(&chat_dir)?;

        let html = self.build_chat_html(chat_key, messages);
        let output_path = format!("{}/{}.html", chat_dir, self.sanitize_filename(chat_key));
        fs::write(&output_path, html)?;
        Ok(())
    }

    fn save_attachments(&self, output_dir: &str) -> Result<()> {
        use anyhow::anyhow;

        for message in &self.messages {
            if !message.attachments.is_empty() {
                let attachment_subpath = self.get_attachment_path(&message.guid);
                let message_dir = format!("{}/attachments/{}", output_dir, attachment_subpath);
                fs::create_dir_all(&message_dir)?;

                for attachment in &message.attachments {
                    if let Some(filename) = attachment.filename()
                        && let Some(bytes) = attachment
                            .as_bytes(&Platform::macOS, &self.database_path, None)
                            .map_err(|e| anyhow!(e))?
                    {
                        let output_path = format!("{}/{}", message_dir, filename);
                        fs::write(&output_path, bytes)?;
                    }
                }
            }
        }

        Ok(())
    }

    fn get_attachment_path(&self, guid: &str) -> String {
        // Extract first 4 characters from GUID for two-level directory structure
        // Example: "FE718EBE-BB92-4650-A656-D59ACB15619C" -> "FE/71/FE718EBE-BB92-4650-A656-D59ACB15619C"
        let level1 = &guid[0..2];
        let level2 = &guid[2..4];
        format!("{}/{}/{}", level1, level2, guid)
    }

    fn sanitize_filename(&self, name: &str) -> String {
        name.chars()
            .map(|c| match c {
                '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
                _ => c,
            })
            .collect()
    }

    fn build_chat_html(&self, chat_name: &str, messages: &[&CleanMessage]) -> String {
        let mut html = String::new();

        // Extract unique participants (excluding "Me")
        let is_group_chat = !chat_name.starts_with("Direct: ");
        let mut participants: Vec<String> = messages
            .iter()
            .map(|m| m.from.to_string())
            .filter(|name| name != "Me")
            .collect();
        participants.sort();
        participants.dedup();

        // HTML header with CSS
        html.push_str(&format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{}</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif;
            max-width: 800px;
            margin: 0 auto;
            padding: 20px;
            background-color: #f5f5f5;
        }}

        .back-link {{
            display: inline-block;
            margin-bottom: 20px;
            padding: 8px 16px;
            background-color: #007aff;
            color: white;
            text-decoration: none;
            border-radius: 8px;
            transition: background-color 0.2s;
        }}

        .back-link:hover {{
            background-color: #0051d5;
        }}

        h1 {{
            text-align: center;
            color: #333;
            border-bottom: 2px solid #007aff;
            padding-bottom: 10px;
        }}

        .participants {{
            background: white;
            border-radius: 12px;
            padding: 16px 20px;
            margin-bottom: 20px;
            box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
        }}

        .participants-header {{
            font-weight: 600;
            color: #333;
            margin-bottom: 10px;
            font-size: 0.95em;
        }}

        .participants-list {{
            display: flex;
            flex-wrap: wrap;
            gap: 8px;
        }}

        .participant {{
            background-color: #e5e5ea;
            color: #333;
            padding: 6px 12px;
            border-radius: 16px;
            font-size: 0.9em;
        }}

        .message {{
            margin: 15px 0;
            padding: 12px 16px;
            border-radius: 18px;
            max-width: 70%;
            word-wrap: break-word;
            position: relative;
        }}

        .message.from-me {{
            background-color: #007aff;
            color: white;
            margin-left: auto;
            margin-right: 0;
        }}

        .message.from-others {{
            background-color: #e5e5ea;
            color: black;
            margin-left: 0;
            margin-right: auto;
        }}

        .message-header {{
            font-size: 0.85em;
            margin-bottom: 6px;
            opacity: 0.8;
            font-weight: 600;
        }}

        .message.from-me .message-header {{
            color: rgba(255, 255, 255, 0.9);
        }}

        .message.from-others .message-header {{
            color: rgba(0, 0, 0, 0.6);
        }}

        .message-text {{
            white-space: pre-wrap;
            line-height: 1.4;
        }}

        .message-footer {{
            font-size: 0.75em;
            margin-top: 6px;
            opacity: 0.7;
        }}

        .attachments {{
            margin-top: 10px;
        }}

        .attachment-image {{
            max-width: 100%;
            border-radius: 12px;
            margin-top: 8px;
            display: block;
        }}

        .attachment-link {{
            display: inline-block;
            padding: 8px 12px;
            background-color: rgba(0, 0, 0, 0.1);
            border-radius: 8px;
            text-decoration: none;
            color: inherit;
            margin-top: 8px;
            font-size: 0.9em;
        }}

        .message.from-me .attachment-link {{
            background-color: rgba(255, 255, 255, 0.2);
            color: white;
        }}

        .attachment-link:hover {{
            opacity: 0.8;
        }}

        .attachment-icon {{
            margin-right: 6px;
        }}

        .tapbacks {{
            margin-top: 6px;
            font-size: 0.9em;
            display: flex;
            flex-wrap: wrap;
            gap: 8px;
        }}

        .tapback {{
            display: inline-flex;
            align-items: center;
            background-color: rgba(0, 0, 0, 0.05);
            padding: 4px 8px;
            border-radius: 12px;
            font-size: 0.85em;
        }}

        .message.from-me .tapback {{
            background-color: rgba(255, 255, 255, 0.2);
        }}

        .tapback-emoji {{
            font-size: 1.2em;
            margin-right: 4px;
        }}

        .tapback-name {{
            opacity: 0.8;
        }}

        .date-separator {{
            text-align: center;
            color: #666;
            font-size: 0.85em;
            margin: 20px 0;
            font-weight: 500;
        }}
    </style>
</head>
<body>
    <a href="../index.html" class="back-link">← Back to Chats</a>
    <h1>{}</h1>
"#,
            chat_name, chat_name
        ));

        // Add participants section for group chats
        if is_group_chat && !participants.is_empty() {
            html.push_str(
                r#"    <div class="participants">
        <div class="participants-header">Participants:</div>
        <div class="participants-list">
"#,
            );
            for participant in &participants {
                html.push_str(&format!(
                    r#"            <span class="participant">{}</span>
"#,
                    self.html_escape(participant)
                ));
            }
            html.push_str(
                r#"        </div>
    </div>
"#,
            );
        }

        // Group messages by date
        let mut last_date = String::new();

        for message in messages {
            let message_date = message.date.format("%B %d, %Y").to_string();

            // Add date separator if date changed
            if message_date != last_date {
                html.push_str(&format!(
                    r#"    <div class="date-separator">{}</div>
"#,
                    message_date
                ));
                last_date = message_date;
            }

            // Determine message class
            let message_class = if message.from.to_string() == "Me" {
                "from-me"
            } else {
                "from-others"
            };

            html.push_str(&format!(
                r#"    <div class="message {}">
"#,
                message_class
            ));

            // Message header (sender name for others)
            if message_class == "from-others" {
                html.push_str(&format!(
                    r#"        <div class="message-header">{}</div>
"#,
                    self.html_escape(&message.from.to_string())
                ));
            }

            // Message text
            if !message.text.is_empty() {
                html.push_str(&format!(
                    r#"        <div class="message-text">{}</div>
"#,
                    self.html_escape(&message.text)
                ));
            }

            // Attachments
            if !message.attachments.is_empty() {
                html.push_str(
                    r#"        <div class="attachments">
"#,
                );

                for attachment in &message.attachments {
                    if let Some(filename) = attachment.filename() {
                        let attachment_subpath = self.get_attachment_path(&message.guid);
                        let attachment_path =
                            format!("../attachments/{}/{}", attachment_subpath, filename);

                        // Use MIME type to determine how to display the attachment
                        use imessage_database::tables::attachment::MediaType;
                        match attachment.mime_type() {
                            MediaType::Image(_) => {
                                html.push_str(&format!(
                                    r#"            <img src="{}" alt="{}" class="attachment-image">
"#,
                                    attachment_path,
                                    self.html_escape(filename)
                                ));
                            }
                            MediaType::Video(_) => {
                                html.push_str(&format!(
                                    r#"            <video src="{}" controls class="attachment-image">
                Your browser does not support the video tag.
            </video>
"#,
                                    attachment_path
                                ));
                            }
                            MediaType::Audio(_) => {
                                html.push_str(&format!(
                                    r#"            <audio src="{}" controls class="attachment-link">
                Your browser does not support the audio tag.
            </audio>
"#,
                                    attachment_path
                                ));
                            }
                            _ => {
                                // For other files (text, application, other), create a download link
                                let icon = self.get_file_icon(filename);
                                html.push_str(&format!(
                                    r#"            <a href="{}" class="attachment-link" download>
                <span class="attachment-icon">{}</span>{}
            </a>
"#,
                                    attachment_path,
                                    icon,
                                    self.html_escape(filename)
                                ));
                            }
                        }
                    }
                }

                html.push_str(
                    r#"        </div>
"#,
                );
            }

            // Tapbacks
            if !message.tapbacks.is_empty() {
                html.push_str(
                    r#"        <div class="tapbacks">
"#,
                );

                for (handle, emoji) in &message.tapbacks {
                    html.push_str(&format!(
                        r#"            <div class="tapback">
                <span class="tapback-emoji">{}</span>
                <span class="tapback-name">{}</span>
            </div>
"#,
                        emoji,
                        self.html_escape(&handle.to_string())
                    ));
                }

                html.push_str(
                    r#"        </div>
"#,
                );
            }

            // Message footer (timestamp)
            html.push_str(&format!(
                r#"        <div class="message-footer">{}</div>
"#,
                message.date.format("%I:%M %p")
            ));

            html.push_str(
                r#"    </div>
"#,
            );
        }

        // Close HTML
        html.push_str(
            r#"</body>
</html>
"#,
        );

        html
    }

    fn html_escape(&self, text: &str) -> String {
        text.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#39;")
    }

    fn get_file_icon(&self, filename: &str) -> &str {
        let lower = filename.to_lowercase();

        if lower.ends_with(".pdf") {
            "📄"
        } else if lower.ends_with(".mp4") || lower.ends_with(".mov") || lower.ends_with(".avi") {
            "🎥"
        } else if lower.ends_with(".mp3") || lower.ends_with(".m4a") || lower.ends_with(".wav") {
            "🎵"
        } else if lower.ends_with(".zip") || lower.ends_with(".tar") || lower.ends_with(".gz") {
            "📦"
        } else if lower.ends_with(".doc") || lower.ends_with(".docx") {
            "📝"
        } else {
            "📎"
        }
    }
}
