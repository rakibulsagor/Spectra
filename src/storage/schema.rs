use anyhow::Result;
use rusqlite::Connection;

pub const CURRENT_SCHEMA_VERSION: i32 = 1;

pub fn run_migrations(conn: &mut Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS schema_version (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            version INTEGER NOT NULL
        );
        ",
    )?;

    let current_version: i32 = conn
        .query_row(
            "SELECT version FROM schema_version WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if current_version < 1 {
        let tx = conn.transaction()?;
        tx.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS messages (
                id INTEGER NOT NULL,
                chat_id INTEGER NOT NULL,
                sender_id INTEGER,
                sender_name TEXT NOT NULL,
                text TEXT NOT NULL,
                date INTEGER NOT NULL,
                is_deleted INTEGER NOT NULL DEFAULT 0,
                deleted_at INTEGER,
                is_edited INTEGER NOT NULL DEFAULT 0,
                media_type TEXT,
                outgoing INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (chat_id, id)
            );

            CREATE INDEX IF NOT EXISTS idx_messages_chat_date ON messages(chat_id, date ASC);
            CREATE INDEX IF NOT EXISTS idx_messages_deleted ON messages(chat_id, is_deleted);

            CREATE TABLE IF NOT EXISTS message_edits (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                message_id INTEGER NOT NULL,
                chat_id INTEGER NOT NULL,
                old_text TEXT NOT NULL,
                edited_at INTEGER NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_edits_lookup ON message_edits(chat_id, message_id, edited_at ASC);

            CREATE TABLE IF NOT EXISTS chats (
                id INTEGER PRIMARY KEY,
                title TEXT NOT NULL,
                username TEXT,
                chat_type TEXT NOT NULL,
                last_message_date INTEGER,
                last_message_preview TEXT,
                is_pinned INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            INSERT OR REPLACE INTO schema_version (id, version) VALUES (1, 1);
            ",
        )?;
        tx.commit()?;
    }

    Ok(())
}
