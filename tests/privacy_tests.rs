use spectra::models::GhostSettings;
use spectra::privacy::{redact_sensitive, GhostManager};
use spectra::storage::schema::CURRENT_SCHEMA_VERSION;
use spectra::storage::Database;

#[test]
fn test_schema_version_is_valid() {
    assert_eq!(CURRENT_SCHEMA_VERSION, 1);
}

#[test]
fn test_ghost_manager_policies() {
    let db = Database::in_memory().unwrap();
    let ghost = GhostManager::new(db);

    // Default: Ghost mode is enabled
    assert!(ghost.is_ghost_mode_enabled());
    assert!(!ghost.should_send_read());
    assert!(!ghost.should_send_typing());
    assert!(!ghost.should_send_online());
    assert!(!ghost.should_send_stories());
    assert!(ghost.is_anti_recall_enabled());
    assert!(ghost.is_anti_edit_enabled());
    assert!(ghost.bypass_noforwards());
    assert!(!ghost.is_streamer_mode());

    // Toggle ghost mode off
    let toggled = ghost.toggle_ghost_mode();
    assert!(!toggled);
    assert!(!ghost.is_ghost_mode_enabled());

    // When ghost mode is disabled, normal Telegram behavior resumes:
    assert!(ghost.should_send_read());
    assert!(ghost.should_send_typing());
    assert!(ghost.should_send_online());
    assert!(ghost.should_send_stories());

    // Toggle back on
    ghost.set_ghost_mode_enabled(true);
    assert!(ghost.is_ghost_mode_enabled());
    assert!(!ghost.should_send_read());

    // Update custom settings
    let custom = GhostSettings {
        streamer_mode: true,
        send_read_messages: true,
        ..GhostSettings::default()
    };
    ghost.update_settings(custom);

    assert!(ghost.is_streamer_mode());
    assert!(ghost.should_send_read());
    assert!(!ghost.should_send_typing());
}

#[test]
fn test_sensitive_data_redaction() {
    assert_eq!(redact_sensitive("test"), "***");
    assert_eq!(redact_sensitive("+1234567890"), "+1***90");
    assert_eq!(redact_sensitive("my_secret_token"), "my***en");
}
