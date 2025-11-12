# iMessage Extractor

A Rust utility to export iMessage conversations to searchable HTML format with embedded media.

## Features

- **HTML Export**: Generates clean, styled HTML pages for each conversation
- **Media Support**: 
  - Embedded images display inline
  - Video files (.mov, .mp4, etc.) play with HTML5 video player
  - Audio files play with HTML5 audio player
  - Other attachments available as downloads
- **Smart Contact Resolution**: Uses macOS Contacts to display real names instead of phone numbers/emails
- **Searchable Index**: Central index page with search functionality to find conversations by name or participant
- **Tapback Support**: Displays reactions (hearts, likes, etc.) on messages
- **Date Filtering**: Export messages within specific date ranges
- **Chat Filtering**: Export specific conversations or all at once

## Usage

```bash
# Export all messages
cargo run -- --output-directory output

# Export messages from a specific chat
cargo run -- --chat "Family Group" --output-directory output

# Export messages within a date range
cargo run -- --start-date 2024-01-01 --end-date 2024-12-31 --output-directory output

# Combine filters
cargo run -- --chat "Work Team" --start-date 2024-01-01 --output-directory output
```

## Options

- `--output-directory <PATH>`: Output directory (default: `output`)
- `--start-date <YYYY-MM-DD>`: Only export messages on or after this date
- `--end-date <YYYY-MM-DD>`: Only export messages before this date
- `--chat <NAME>`: Export specific chat(s) - can be used multiple times
- `--database-path <PATH>`: Override default iMessage database location

## Requirements

- macOS (tested on recent versions)
- Rust toolchain
- **Full Disk Access** for your terminal emulator in System Settings > Privacy & Security > Full Disk Access

## Known Limitations

### Message Direction in Some Conversations

In rare cases, you may encounter conversations where all messages appear to be from the other person, with none showing as sent by you. This is a **database issue** in the iMessage `chat.db` file itself, where the `is_from_me` field is incorrectly set to `0` (false) for all messages in certain conversation threads.

**Why this happens:**
- **SMS vs iMessage**: SMS messages sometimes don't have sender metadata set correctly
- **Account switching**: Messages sent from a different phone number or Apple ID
- **Database sync issues**: iCloud sync problems can corrupt message metadata
- **Conversation type quirks**: Some conversation types store metadata differently

**What you'll see:**
- A conversation showing only messages from the other person
- Often accompanied by a separate group chat with the same person showing both sides correctly

**Unfortunately, this cannot be fixed by the extractor** because:
1. The `is_from_me` field is the authoritative source for message direction
2. No other reliable metadata exists to determine the true sender
3. This is a problem in the source database, not the export process

**Workaround:**
If you have a named group chat with the same person (even if it's just the two of you), that conversation likely has correct sender information. The group chat and direct message threads are stored separately in the database.

## Project Structure

```
imessage_extractor/
├── src/
│   ├── main.rs              # CLI and orchestration
│   ├── clean_message.rs     # Message data structure
│   ├── html_output.rs       # HTML generation
│   ├── message_store.rs     # Message collection and grouping
│   ├── resolved_handle.rs   # Contact resolution
│   ├── tapback_emoji.rs     # Tapback reactions
│   └── contacts.rs          # macOS Contacts integration
├── contacts_helper.swift    # Swift script for Contacts access
└── README.md
```

## Dependencies

- `imessage-database`: Interface to iMessage database
- `rusqlite`: SQLite access
- `chrono`: Date/time handling
- `anyhow`: Error handling
- `gumdrop`: CLI argument parsing

## Building

```bash
cargo build --release
```

The binary will be at `target/release/imessage_extractor`.

## Debugging

If you encounter conversations where all messages appear to be from one person, you can use the debug utility to inspect the raw database values:

```bash
cargo run --bin debug_messages
```

This will search for conversations (you can modify the search terms in the source), display message statistics showing the `is_from_me` field distribution, and output sample messages with their actual database values. This helps confirm whether the issue is in the database or the export code.

## Output Structure

```
output/
├── index.html           # Searchable list of all conversations
├── groups/              # Group chat HTML files
│   └── [chat_name].html
├── direct/              # Direct message HTML files
│   └── Direct_ [name].html
└── attachments/         # Media files organized by message GUID
    └── [GUID]/
        └── [filename]
```

## License

This is a personal utility. Use at your own risk.
