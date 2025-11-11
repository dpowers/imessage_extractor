# Contacts Library

A minimal Rust library for reading Mac Contacts (Address Book) data.

## Overview

This library provides a simple interface to access contact information from macOS Contacts app, including:
- Contact names (given name and family name)
- Phone numbers
- Email addresses

## Implementation

The library uses a hybrid approach:
- **Swift script** (`contacts_helper.swift`): Accesses the Contacts framework and outputs JSON
- **Rust module** (`src/contacts.rs`): Embeds the Swift script in the binary and streams it to the `swift` command via stdin

The Swift script is embedded at compile time using `include_str!()` and executed by piping it to `swift -`, so the final binary is self-contained and doesn't require any external script files.

This approach avoids complex FFI bindings while keeping the code minimal and maintainable.

## Usage

```rust
use crate::contacts::{fetch_all_contacts, find_contact_by_phone, find_contact_by_email, Contact};

// Fetch all contacts
let contacts = fetch_all_contacts()?;
```

## API

### Types

```rust
pub struct Contact {
    pub given_name: String,
    pub family_name: String,
    pub phone_numbers: Vec<String>,
    pub email_addresses: Vec<String>,
}
```

### Functions

- `fetch_all_contacts() -> Result<Vec<Contact>>` - Fetches all contacts from the Contacts database
- `find_contact_by_phone(phone_number: &str) -> Result<Option<Contact>>` - Finds a contact by phone number (fuzzy matching on digits)
- `find_contact_by_email(email: &str) -> Result<Option<Contact>>` - Finds a contact by email address (case-insensitive)
- `Contact::full_name(&self) -> String` - Returns the full name (given + family name)

## Requirements

- macOS (uses Contacts framework)
- Swift runtime (included with macOS)
- Contacts access permissions (macOS will prompt the user on first access)

## Testing

The library includes a snapshot test using the `insta` crate:

```bash
# Run the test
cargo test test_fetch_all_contacts

# Review snapshots if they change
cargo insta review
```

The test verifies that:
- A reasonable number of contacts are returned (> 0 and < 10,000)
- Contacts have the expected structure (names, phone numbers, emails)
- The snapshot captures the structure without exposing actual contact data

## Permissions

When your app first attempts to access Contacts, macOS will display a permission dialog. The user must grant access for the library to work. If access is denied, the functions will return errors.

## Notes

- The Swift script is embedded in the compiled binary at build time
- The script is executed as a subprocess for each request, which is simple but not the fastest approach
- For production use with frequent access, consider caching results or using a more direct FFI binding
- Phone number matching strips non-numeric characters for comparison
- Email matching is case-insensitive
- The binary is self-contained and doesn't require external script files at runtime
