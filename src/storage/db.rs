use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tracing::{debug, info};

use crate::models::{ChatSummary, GhostSettings, MessageRevision, SpectraMessage};
use crate::storage::schema::run_migrations;

#[derive(Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut conn = Connection::open(&path)
            .with_context(|| format!("Failed to open SQLite database at {:?}", path.as_ref()))?;

        run_migrations(&mut conn)?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn in_memory() -> Result<Self> {
        let mut conn = Connection::open_in_memory()?;
        run_migrations(&mut conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Saves or updates a message in local storage
    pub fn save_message(&self, msg: &SpectraMessage) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO messages (id, chat_id, sender_id, sender_name, text, date, is_deleted, deleted_at, is_edited, media_type, outgoing)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
             ON CONFLICT(chat_id, id) DO UPDATE SET
                text = excluded.text,
                media_type = excluded.media_type",
            params![
                msg.id,
                msg.chat_id,
                msg.sender_id,
                msg.sender_name,
                msg.text,
                msg.date,
                msg.is_deleted as i32,
                msg.deleted_at,
                msg.is_edited as i32,
                msg.media_type,
                msg.outgoing as i32,
            ],
        )?;

        // Update preview in chats table
        let preview = if msg.text.len() > 60 {
            format!("{}...", &msg.text[..57])
        } else {
            msg.text.clone()
        };

        let _ = conn.execute(
            "UPDATE chats SET last_message_date = ?1, last_message_preview = ?2 WHERE id = ?3",
            params![msg.date, preview, msg.chat_id],
        );

        debug!(chat_id = msg.chat_id, msg_id = msg.id, "Saved message");
        Ok(())
    }

    /// Fetches a specific message
    pub fn get_message(&self, chat_id: i64, message_id: i32) -> Result<Option<SpectraMessage>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, chat_id, sender_id, sender_name, text, date, is_deleted, deleted_at, is_edited, media_type, outgoing
             FROM messages WHERE chat_id = ?1 AND id = ?2",
        )?;

        let mut rows = stmt.query(params![chat_id, message_id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(SpectraMessage {
                id: row.get(0)?,
                chat_id: row.get(1)?,
                sender_id: row.get(2)?,
                sender_name: row.get(3)?,
                text: row.get(4)?,
                date: row.get(5)?,
                is_deleted: row.get::<_, i32>(6)? != 0,
                deleted_at: row.get(7)?,
                is_edited: row.get::<_, i32>(8)? != 0,
                media_type: row.get(9)?,
                outgoing: row.get::<_, i32>(10)? != 0,
            }))
        } else {
            Ok(None)
        }
    }

    /// Core Anti-Recall: marks a message as deleted while retaining all text and media info
    pub fn mark_message_deleted(
        &self,
        chat_id: i64,
        message_id: i32,
    ) -> Result<Option<SpectraMessage>> {
        let now = Utc::now().timestamp();
        let existing = self.get_message(chat_id, message_id)?;

        let conn = self.conn.lock().unwrap();
        if existing.is_some() {
            conn.execute(
                "UPDATE messages SET is_deleted = 1, deleted_at = ?1 WHERE chat_id = ?2 AND id = ?3",
                params![now, chat_id, message_id],
            )?;
        } else {
            // Uncached message registered as deleted stub
            conn.execute(
                "INSERT OR IGNORE INTO messages (id, chat_id, sender_name, text, date, is_deleted, deleted_at)
                 VALUES (?1, ?2, '[Unknown]', '[Message deleted before sync]', ?3, 1, ?4)",
                params![message_id, chat_id, now, now],
            )?;
        }
        drop(conn);

        info!(
            chat_id = chat_id,
            message_id = message_id,
            "Anti-Recall preserved message"
        );
        self.get_message(chat_id, message_id)
    }

    /// Core Anti-Edit: records prior version in revisions table and updates text
    pub fn record_message_edit(
        &self,
        chat_id: i64,
        message_id: i32,
        new_text: &str,
    ) -> Result<Option<String>> {
        let now = Utc::now().timestamp();
        let existing = self.get_message(chat_id, message_id)?;

        let old_text = if let Some(msg) = &existing {
            let conn = self.conn.lock().unwrap();
            conn.execute(
                "INSERT INTO message_edits (message_id, chat_id, old_text, edited_at)
                 VALUES (?1, ?2, ?3, ?4)",
                params![message_id, chat_id, msg.text, now],
            )?;

            conn.execute(
                "UPDATE messages SET text = ?1, is_edited = 1 WHERE chat_id = ?2 AND id = ?3",
                params![new_text, chat_id, message_id],
            )?;
            info!(
                chat_id = chat_id,
                message_id = message_id,
                "Anti-Edit logged revision"
            );
            Some(msg.text.clone())
        } else {
            None
        };

        Ok(old_text)
    }

    /// Retrieves messages for a chat in chronological order
    pub fn get_chat_messages(&self, chat_id: i64, limit: usize) -> Result<Vec<SpectraMessage>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, chat_id, sender_id, sender_name, text, date, is_deleted, deleted_at, is_edited, media_type, outgoing
             FROM messages WHERE chat_id = ?1 ORDER BY date DESC LIMIT ?2",
        )?;

        let rows = stmt.query_map(params![chat_id, limit as i64], |row| {
            Ok(SpectraMessage {
                id: row.get(0)?,
                chat_id: row.get(1)?,
                sender_id: row.get(2)?,
                sender_name: row.get(3)?,
                text: row.get(4)?,
                date: row.get(5)?,
                is_deleted: row.get::<_, i32>(6)? != 0,
                deleted_at: row.get(7)?,
                is_edited: row.get::<_, i32>(8)? != 0,
                media_type: row.get(9)?,
                outgoing: row.get::<_, i32>(10)? != 0,
            })
        })?;

        let mut messages = Vec::new();
        for r in rows {
            messages.push(r?);
        }
        messages.reverse();
        Ok(messages)
    }

    /// Retrieves all messages that were deleted
    pub fn get_deleted_messages(
        &self,
        chat_id: Option<i64>,
        limit: Option<usize>,
    ) -> Result<Vec<SpectraMessage>> {
        let conn = self.conn.lock().unwrap();
        let mut messages = Vec::new();
        let lim = limit.unwrap_or(500) as i64;

        if let Some(cid) = chat_id {
            let mut stmt = conn.prepare(
                "SELECT id, chat_id, sender_id, sender_name, text, date, is_deleted, deleted_at, is_edited, media_type, outgoing
                 FROM messages WHERE chat_id = ?1 AND is_deleted = 1 ORDER BY deleted_at DESC LIMIT ?2",
            )?;
            let rows = stmt.query_map(params![cid, lim], |row| {
                Ok(SpectraMessage {
                    id: row.get(0)?,
                    chat_id: row.get(1)?,
                    sender_id: row.get(2)?,
                    sender_name: row.get(3)?,
                    text: row.get(4)?,
                    date: row.get(5)?,
                    is_deleted: true,
                    deleted_at: row.get(7)?,
                    is_edited: row.get::<_, i32>(8)? != 0,
                    media_type: row.get(9)?,
                    outgoing: row.get::<_, i32>(10)? != 0,
                })
            })?;
            for r in rows {
                messages.push(r?);
            }
        } else {
            let mut stmt = conn.prepare(
                "SELECT id, chat_id, sender_id, sender_name, text, date, is_deleted, deleted_at, is_edited, media_type, outgoing
                 FROM messages WHERE is_deleted = 1 ORDER BY deleted_at DESC LIMIT ?1",
            )?;
            let rows = stmt.query_map(params![lim], |row| {
                Ok(SpectraMessage {
                    id: row.get(0)?,
                    chat_id: row.get(1)?,
                    sender_id: row.get(2)?,
                    sender_name: row.get(3)?,
                    text: row.get(4)?,
                    date: row.get(5)?,
                    is_deleted: true,
                    deleted_at: row.get(7)?,
                    is_edited: row.get::<_, i32>(8)? != 0,
                    media_type: row.get(9)?,
                    outgoing: row.get::<_, i32>(10)? != 0,
                })
            })?;
            for r in rows {
                messages.push(r?);
            }
        }

        Ok(messages)
    }

    /// Retrieves all recorded edits for a message
    pub fn get_message_revisions(
        &self,
        chat_id: i64,
        message_id: i32,
    ) -> Result<Vec<MessageRevision>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, message_id, chat_id, old_text, edited_at
             FROM message_edits WHERE chat_id = ?1 AND message_id = ?2 ORDER BY edited_at ASC",
        )?;

        let rows = stmt.query_map(params![chat_id, message_id], |row| {
            Ok(MessageRevision {
                id: row.get(0)?,
                message_id: row.get(1)?,
                chat_id: row.get(2)?,
                old_text: row.get(3)?,
                edited_at: row.get(4)?,
            })
        })?;

        let mut revisions = Vec::new();
        for r in rows {
            revisions.push(r?);
        }
        Ok(revisions)
    }

    /// Upserts a chat in the sidebar catalog
    pub fn upsert_chat(&self, chat: &ChatSummary) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO chats (id, title, username, chat_type, last_message_date, last_message_preview)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET
                title = excluded.title,
                username = excluded.username,
                chat_type = excluded.chat_type,
                last_message_date = COALESCE(excluded.last_message_date, chats.last_message_date),
                last_message_preview = COALESCE(excluded.last_message_preview, chats.last_message_preview)",
            params![chat.id, chat.title, chat.username, chat.chat_type, chat.last_message_date, chat.last_message_preview],
        )?;
        Ok(())
    }

    /// Fetches all dialog summaries
    pub fn get_chats(&self) -> Result<Vec<ChatSummary>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT c.id, c.title, c.username, c.chat_type, c.last_message_date, c.last_message_preview,
                    (SELECT COUNT(*) FROM messages m WHERE m.chat_id = c.id AND m.is_deleted = 1) AS del_count
             FROM chats c ORDER BY c.last_message_date DESC",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(ChatSummary {
                id: row.get(0)?,
                title: row.get(1)?,
                username: row.get(2)?,
                chat_type: row.get(3)?,
                last_message_date: row.get(4)?,
                last_message_preview: row.get(5)?,
                deleted_messages_count: row.get::<_, i64>(6)? as usize,
            })
        })?;

        let mut chats = Vec::new();
        for r in rows {
            chats.push(r?);
        }
        Ok(chats)
    }

    /// Fetches a specific chat summary
    pub fn get_chat(&self, id: i64) -> Result<Option<ChatSummary>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT c.id, c.title, c.username, c.chat_type, c.last_message_date, c.last_message_preview,
                    (SELECT COUNT(*) FROM messages m WHERE m.chat_id = c.id AND m.is_deleted = 1) AS del_count
             FROM chats c WHERE c.id = ?1",
        )?;

        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(ChatSummary {
                id: row.get(0)?,
                title: row.get(1)?,
                username: row.get(2)?,
                chat_type: row.get(3)?,
                last_message_date: row.get(4)?,
                last_message_preview: row.get(5)?,
                deleted_messages_count: row.get::<_, i64>(6)? as usize,
            }))
        } else {
            Ok(None)
        }
    }

    /// Explicit local history deletion (User privacy control requirement)
    pub fn purge_chat_history(&self, chat_id: i64) -> Result<()> {
        let tx = self.conn.lock().unwrap();
        tx.execute("DELETE FROM messages WHERE chat_id = ?1", params![chat_id])?;
        tx.execute(
            "DELETE FROM message_edits WHERE chat_id = ?1",
            params![chat_id],
        )?;
        info!(
            chat_id = chat_id,
            "Purged local chat history on user request"
        );
        Ok(())
    }

    /// Loads GhostSettings from database
    pub fn load_settings(&self) -> GhostSettings {
        let conn = self.conn.lock().unwrap();
        let val: Option<String> = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'ghost_settings'",
                [],
                |r| r.get(0),
            )
            .ok();

        if let Some(v) = val {
            serde_json::from_str(&v).unwrap_or_default()
        } else {
            GhostSettings::default()
        }
    }

    /// Saves GhostSettings to database
    pub fn save_settings(&self, settings: &GhostSettings) -> Result<()> {
        let val = serde_json::to_string(settings)?;
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO settings (key, value) VALUES ('ghost_settings', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![val],
        )?;
        Ok(())
    }
}
