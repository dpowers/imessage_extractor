#!/usr/bin/env swift
import Contacts
import Foundation

struct ContactData: Codable {
    let givenName: String
    let familyName: String
    let phoneNumbers: [String]
    let emailAddresses: [String]
}

let store = CNContactStore()
let keys =
    [
        CNContactGivenNameKey,
        CNContactFamilyNameKey,
        CNContactPhoneNumbersKey,
        CNContactEmailAddressesKey,
    ] as [CNKeyDescriptor]

var contacts: [ContactData] = []

let fetchRequest = CNContactFetchRequest(keysToFetch: keys)

do {
    try store.enumerateContacts(with: fetchRequest) { contact, _ in
        let contactData = ContactData(
            givenName: contact.givenName,
            familyName: contact.familyName,
            phoneNumbers: contact.phoneNumbers.map { $0.value.stringValue },
            emailAddresses: contact.emailAddresses.map { $0.value as String }
        )
        contacts.append(contactData)
    }

    let encoder = JSONEncoder()
    encoder.outputFormatting = .prettyPrinted
    let jsonData = try encoder.encode(contacts)

    if let jsonString = String(data: jsonData, encoding: .utf8) {
        print(jsonString)
    }
} catch {
    fputs("Error: \(error.localizedDescription)\n", stderr)
    exit(1)
}
