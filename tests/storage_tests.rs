use spectra::models::{ChatSummary, SpectraMessage};
use spectra::storage::Database;

#[test]
fn test_database_migrations_and_in_memory() {
    let db = Database::in_memory().expect("Failed to create in-memory db");
    let chats = db.get_chats().expect("Failed to query chats");
    assert_eq!(chats.len(), 0);
}

#[test]
fn test_message_save_and_retrieve() {
    let db = Database::in_memory().unwrap();

    let msg = SpectraMessage {
        id: 101,
        chat_id: 500,
        sender_id: Some(999),
        sender_name: "Alice".to_string(),
        text: "Testing Spectra Storage".to_string(),
        date: 1700000000,
        is_deleted: false,
        deleted_at: None,
        is_edited: false,
        media_type: None,
        outgoing: false,
    };

    db.save_message(&msg).unwrap();

    let fetched = db
        .get_message(500, 101)
        .unwrap()
        .expect("Message not found");
    assert_eq!(fetched.id, 101);
    assert_eq!(fetched.chat_id, 500);
    assert_eq!(fetched.sender_name, "Alice");
    assert_eq!(fetched.text, "Testing Spectra Storage");
    assert!(!fetched.is_deleted);
}

#[test]
fn test_anti_recall_deletion_preservation() {
    let db = Database::in_memory().unwrap();

    let msg = SpectraMessage {
        id: 202,
        chat_id: 600,
        sender_id: Some(888),
        sender_name: "Bob".to_string(),
        text: "Sensitive secret token".to_string(),
        date: 1700000100,
        is_deleted: false,
        deleted_at: None,
        is_edited: false,
        media_type: None,
        outgoing: false,
    };

    db.save_message(&msg).unwrap();

    // Mark as deleted by sender (anti-recall)
    let preserved = db.mark_message_deleted(600, 202).unwrap();
    assert!(preserved.is_some());
    let preserved_msg = preserved.unwrap();
    assert!(preserved_msg.is_deleted);
    assert!(preserved_msg.deleted_at.is_some());
    assert_eq!(preserved_msg.text, "Sensitive secret token");

    // Check message in chat view
    let in_chat = db.get_message(600, 202).unwrap().unwrap();
    assert!(in_chat.is_deleted);
    assert!(in_chat.deleted_at.is_some());
    assert_eq!(in_chat.text, "Sensitive secret token");

    // Check deleted messages query
    let deleted_list = db.get_deleted_messages(Some(600), None).unwrap();
    assert_eq!(deleted_list.len(), 1);
    assert_eq!(deleted_list[0].id, 202);

    let all_deleted = db.get_deleted_messages(None, None).unwrap();
    assert_eq!(all_deleted.len(), 1);
}

#[test]
fn test_anti_edit_revisions_tracking() {
    let db = Database::in_memory().unwrap();

    let msg = SpectraMessage {
        id: 303,
        chat_id: 700,
        sender_id: Some(777),
        sender_name: "Charlie".to_string(),
        text: "Version 1: The meeting is at 2 PM".to_string(),
        date: 1700000200,
        is_deleted: false,
        deleted_at: None,
        is_edited: false,
        media_type: None,
        outgoing: false,
    };

    db.save_message(&msg).unwrap();

    // Edit 1
    let old1 = db
        .record_message_edit(700, 303, "Version 2: Meeting moved to 3 PM")
        .unwrap();
    assert_eq!(old1, Some("Version 1: The meeting is at 2 PM".to_string()));

    // Edit 2
    let old2 = db
        .record_message_edit(700, 303, "Version 3: Meeting cancelled")
        .unwrap();
    assert_eq!(old2, Some("Version 2: Meeting moved to 3 PM".to_string()));

    // Verify current message state
    let cur = db.get_message(700, 303).unwrap().unwrap();
    assert!(cur.is_edited);
    assert_eq!(cur.text, "Version 3: Meeting cancelled");

    // Verify historical revisions
    let revs = db.get_message_revisions(700, 303).unwrap();
    assert_eq!(revs.len(), 2);
    assert_eq!(revs[0].old_text, "Version 1: The meeting is at 2 PM");
    assert_eq!(revs[1].old_text, "Version 2: Meeting moved to 3 PM");
}

#[test]
fn test_chats_catalog_and_purge() {
    let db = Database::in_memory().unwrap();

    let chat = ChatSummary {
        id: 800,
        title: "Test Group".to_string(),
        username: Some("test_group".to_string()),
        chat_type: "group".to_string(),
        last_message_date: Some(1700000300),
        last_message_preview: Some("Hello all".to_string()),
        deleted_messages_count: 0,
    };

    db.upsert_chat(&chat).unwrap();

    let chats = db.get_chats().unwrap();
    assert_eq!(chats.len(), 1);
    assert_eq!(chats[0].title, "Test Group");

    let specific = db.get_chat(800).unwrap().expect("Chat not found");
    assert_eq!(specific.id, 800);
    assert_eq!(specific.username, Some("test_group".to_string()));

    // Add message and then purge
    let msg = SpectraMessage {
        id: 1,
        chat_id: 800,
        sender_id: None,
        sender_name: "You".to_string(),
        text: "Purge me".to_string(),
        date: 1700000350,
        is_deleted: false,
        deleted_at: None,
        is_edited: false,
        media_type: None,
        outgoing: true,
    };
    db.save_message(&msg).unwrap();

    let msgs = db.get_chat_messages(800, 10).unwrap();
    assert_eq!(msgs.len(), 1);

    // Purge history
    db.purge_chat_history(800).unwrap();
    let msgs_after = db.get_chat_messages(800, 10).unwrap();
    assert_eq!(msgs_after.len(), 0);
}

#[test]
fn test_settings_persistence() {
    let db = Database::in_memory().unwrap();
    let initial = db.load_settings();
    assert!(initial.ghost_mode_enabled);
    assert!(!initial.send_read_messages);

    let mut modified = initial;
    modified.ghost_mode_enabled = false;
    modified.send_typing_indicators = true;
    db.save_settings(&modified).unwrap();

    let reloaded = db.load_settings();
    assert!(!reloaded.ghost_mode_enabled);
    assert!(reloaded.send_typing_indicators);
}
