use chrono::DateTime;
use serde::{Deserialize, Serialize};

/// Represents a message preserved in Spectra's local history database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpectraMessage {
    pub id: i32,
    pub chat_id: i64,
    pub sender_id: Option<i64>,
    pub sender_name: String,
    pub text: String,
    pub date: i64,
    pub is_deleted: bool,
    pub deleted_at: Option<i64>,
    pub is_edited: bool,
    pub media_type: Option<String>,
    pub outgoing: bool,
}

impl SpectraMessage {
    pub fn formatted_date(&self) -> String {
        DateTime::from_timestamp(self.date, 0)
            .map(|dt| dt.format("%H:%M").to_string())
            .unwrap_or_else(|| "??:??".to_string())
    }

    pub fn formatted_full_date(&self) -> String {
        DateTime::from_timestamp(self.date, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "Unknown".to_string())
    }

    pub fn formatted_deleted_at(&self) -> Option<String> {
        self.deleted_at.and_then(|ts| {
            DateTime::from_timestamp(ts, 0).map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        })
    }
}

/// Historical revision of an edited message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageRevision {
    pub id: i64,
    pub message_id: i32,
    pub chat_id: i64,
    pub old_text: String,
    pub edited_at: i64,
}

impl MessageRevision {
    pub fn formatted_edited_at(&self) -> String {
        DateTime::from_timestamp(self.edited_at, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "Unknown".to_string())
    }
}

/// Conversation/Chat summary for sidebar
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSummary {
    pub id: i64,
    pub title: String,
    pub username: Option<String>,
    pub chat_type: String, // "user", "group", "channel"
    pub last_message_date: Option<i64>,
    pub last_message_preview: Option<String>,
    pub deleted_messages_count: usize,
}

/// Ghost mode configuration and stealth policies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GhostSettings {
    pub ghost_mode_enabled: bool,
    pub send_read_messages: bool,
    pub send_typing_indicators: bool,
    pub send_online_packets: bool,
    pub send_read_stories: bool,
    pub anti_recall_enabled: bool,
    pub anti_edit_enabled: bool,
    pub bypass_noforwards: bool,
    pub streamer_mode: bool,
}

impl Default for GhostSettings {
    fn default() -> Self {
        Self {
            ghost_mode_enabled: true,
            send_read_messages: false,
            send_typing_indicators: false,
            send_online_packets: false,
            send_read_stories: false,
            anti_recall_enabled: true,
            anti_edit_enabled: true,
            bypass_noforwards: true,
            streamer_mode: false,
        }
    }
}

/// Events transmitted from background async worker to GTK UI loop
#[derive(Debug, Clone)]
pub enum AppEvent {
    NewMessage(SpectraMessage),
    MessageDeleted {
        chat_id: i64,
        message_id: i32,
        preserved: Option<SpectraMessage>,
    },
    MessageEdited {
        chat_id: i64,
        message_id: i32,
        old_text: String,
        new_text: String,
    },
    DialogsUpdated(Vec<ChatSummary>),
    ConnectionStatusChanged(String),
    GhostModeToggled(bool),
}
