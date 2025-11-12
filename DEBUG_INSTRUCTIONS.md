# Debug Utility Instructions

## Purpose

This debug utility helps diagnose issues where conversations show all messages from one person, with none appearing as sent by you. It inspects the raw iMessage database to determine if the problem is in the database itself or in the export code.

## Running the Debug Utility

```bash
cargo run --bin debug_messages
```

## Customizing the Search

To search for different conversations, edit `src/bin/debug_messages.rs` line 13:

```rust
let search_terms = vec!["ralph", "douglass"];
```

Change to any names, phone numbers, or emails you want to search for:

```rust
let search_terms = vec!["john", "smith"];  // Find conversations with John Smith
let search_terms = vec!["+1234567890"];    // Find conversations with phone number
let search_terms = vec!["example@email.com"]; // Find conversations with email
```

The search is case-insensitive and matches partial strings.

## Understanding the Output

### 1. Found Handles
```
Found Ralph handle: 42 -> +15555555123
Found Ralph handle: 43 -> ralph@example.com
```
Shows all contact identifiers (phone/email) associated with the search terms.

### 2. Chat List
```
Found 2 chats with Ralph:
  Chat ID 100: Ralph Douglass (+15555555123)
  Chat ID 200:  (ralph@example.com)
```
Lists all conversation threads found. Empty display name indicates a direct/SMS conversation.

### 3. Message Statistics
```
Chat ID 100: Ralph Douglass (+15555555123)
  Total messages: 773
  From me: 0 (0.0%)
  From others: 773 (100.0%)
  ⚠️  WARNING: No messages marked as from_me in this chat with 773 messages!
  This indicates a database issue with is_from_me field
```

**Key indicators:**
- **"From me: 0"** with many total messages = **DATABASE BUG** (the `is_from_me` field is wrong)
- **Balanced split** (e.g., 45% you, 55% them) = Normal, working correctly

### 4. Sample Messages
```
Chat ID 100: Ralph Douglass (+15555555123)
  [Ralph Douglass] is_from_me=false handle_id=Some(42) date=123456789 text="I'm going to eat before I come..."
  [Ralph Douglass] is_from_me=false handle_id=Some(42) date=123456790 text="Yeah..."
  [Ralph Douglass] is_from_me=false handle_id=Some(42) date=123456791 text="Sweet. I will call..."
```

Shows the first 10 messages with raw database values:
- **Sender**: Resolved contact name or "ME" if `is_from_me=true`
- **is_from_me**: The critical field - should be `true` for messages you sent
- **handle_id**: The contact identifier in the database
- **date**: Unix timestamp
- **text**: Preview of message content

## Confirming the Database Issue

If you see:
1. ✅ A conversation with 0% "From me" messages
2. ✅ All sample messages show `is_from_me=false`
3. ✅ But you know you participated in the conversation

Then **this confirms the database has incorrect `is_from_me` values**, and the problem cannot be fixed by the exporter since the source data is wrong.

## Next Steps

If the debug utility confirms a database issue:

1. **Check for a working conversation**: Often there's a separate group chat or named conversation with the same person that has correct data
2. **Consider database repair**: Apple occasionally has bugs that corrupt message metadata during sync
3. **Accept the limitation**: Document which conversations have the issue
4. **Report to Apple**: If this affects many conversations, file feedback at https://feedbackassistant.apple.com

## Modifying for Other Contacts

To check other conversations, simply change the `search_terms` in the source file and run again. You can search for:
- First or last names
- Phone numbers
- Email addresses
- Group chat names
- Any text that appears in chat identifiers

The tool will find all matching conversations and show their statistics.
