use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::process::{Command, Stdio};

const SWIFT_SCRIPT: &str = include_str!("../contacts_helper.swift");

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Contact {
    pub given_name: String,
    pub family_name: String,
    pub phone_numbers: Vec<String>,
    pub email_addresses: Vec<String>,
}

impl Contact {
    pub fn full_name(&self) -> String {
        format!("{} {}", self.given_name, self.family_name)
            .trim()
            .to_string()
    }
}

pub struct ContactMap(HashMap<String, String>);

/// Normalizes a phone number to E.164 format (+1XXXXXXXXXX for US numbers)
///
/// Takes numbers in various formats like:
/// - 555-555-0100
/// - (555) 555-0101
/// - (555) 555-0102
/// - 555 555 0103
///
/// And converts them to:
/// - +15555550107
/// - +15555550101
/// - +15555550102
/// - +15555550103
pub fn normalize_number(number: &str) -> Option<String> {
    // Strip all non-numeric characters
    let digits: String = number.chars().filter(|c| c.is_ascii_digit()).collect();

    // Handle empty or too short numbers
    if digits.is_empty() {
        return None;
    }

    // Handle different length cases
    let normalized = match digits.len() {
        // 10 digits - assume US number, add +1
        10 => format!("+1{}", digits),

        // 11 digits starting with 1 - US number with country code
        11 if digits.starts_with('1') => format!("+{}", digits),

        // 11 digits not starting with 1 - might be international, add +
        11 => format!("+{}", digits),

        // 12+ digits - international number, add +
        12.. => format!("+{}", digits),

        // Less than 10 digits - could be short code or invalid
        _ => return None,
    };

    Some(normalized)
}

impl ContactMap {
    pub fn fetch() -> Result<Self> {
        let mut child = Command::new("swift")
            .arg("-")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn swift command")?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(SWIFT_SCRIPT.as_bytes())
                .context("Failed to write script to swift stdin")?;
        }

        let output = child
            .wait_with_output()
            .context("Failed to wait for swift command")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Contacts helper failed: {}", stderr);
        }

        let stdout = String::from_utf8(output.stdout)
            .context("Failed to parse contacts helper output as UTF-8")?;

        let mut contacts: Vec<Contact> =
            serde_json::from_str(&stdout).context("Failed to parse contacts JSON")?;

        // Normalize all phone numbers in each contact
        for contact in &mut contacts {
            contact.phone_numbers = contact
                .phone_numbers
                .iter()
                .filter_map(|num| normalize_number(num))
                .collect();
        }

        let mut contact_map = HashMap::new();
        for contact in contacts {
            let full_name = contact.full_name();

            for phone_number in contact.phone_numbers {
                contact_map.insert(phone_number, full_name.clone());
            }

            for email_address in contact.email_addresses {
                contact_map.insert(email_address, full_name.clone());
            }
        }

        Ok(ContactMap(contact_map))
    }

    pub fn get(&self, identifier: &str) -> Option<&String> {
        self.0.get(identifier)
    }

    #[allow(unused)]
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fetch() {
        let contacts = ContactMap::fetch().expect("Failed to fetch contacts");

        // Verify we got a reasonable number of contacts
        assert!(
            contacts.len() > 0,
            "Should have at least one contact, got {}",
            contacts.len()
        );
    }

    #[test]
    fn test_normalize_number() {
        // Test various input formats
        assert_eq!(
            normalize_number("555-555-0100"),
            Some("+15555550107".to_string())
        );
        assert_eq!(
            normalize_number("(555) 555-0101"),
            Some("+15555550101".to_string())
        );
        assert_eq!(
            normalize_number("(555) 555-0102"),
            Some("+15555550102".to_string())
        );
        assert_eq!(
            normalize_number("555 555 0103"),
            Some("+15555550103".to_string())
        );

        // Test 11-digit number with leading 1
        assert_eq!(
            normalize_number("15555550104"),
            Some("+15555550104".to_string())
        );
        assert_eq!(
            normalize_number("1 (555) 555-0105"),
            Some("+15555550105".to_string())
        );
        assert_eq!(
            normalize_number("+15555550106"),
            Some("+15555550106".to_string())
        );

        // Test already normalized number
        assert_eq!(
            normalize_number("+15555550107"),
            Some("+15555550107".to_string())
        );

        // Test edge cases
        assert_eq!(normalize_number(""), None); // Empty string
        assert_eq!(normalize_number("123"), None); // Too short
        assert_eq!(normalize_number("abc-def-ghij"), None); // No digits

        // Test with extra characters
        assert_eq!(
            normalize_number("+1 (555) 555-0108"),
            Some("+15555550107".to_string())
        );
        assert_eq!(
            normalize_number("1-555-555-0100"),
            Some("+15555550107".to_string())
        );
    }
}
