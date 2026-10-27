use anyhow::{Context, Result};
use async_channel::Sender;
use grammers_client::Client;
use grammers_mtsender::{SenderPool, UpdatesConfiguration};
use grammers_session::storages::SqliteSession;
use grammers_session::types::PeerId;
use std::sync::Arc;
use tokio::sync::Mutex as AsyncMutex;
use tracing::{error, info, warn};

use crate::models::{AppEvent, ChatSummary};
use crate::privacy::GhostManager;
use crate::storage::Database;
use crate::telegram::interceptor::UpdateInterceptor;

#[derive(Clone)]
pub struct TelegramService {
    client: Arc<AsyncMutex<Option<Client>>>,
    storage: Database,
    ghost: GhostManager,
    ui_tx: Sender<AppEvent>,
    api_id: i32,
    api_hash: String,
}

impl TelegramService {
    pub fn new(
        storage: Database,
        ghost: GhostManager,
        ui_tx: Sender<AppEvent>,
        api_id: i32,
        api_hash: String,
    ) -> Self {
        Self {
            client: Arc::new(AsyncMutex::new(None)),
            storage,
            ghost,
            ui_tx,
            api_id,
            api_hash,
        }
    }

    /// Connects to MTProto and starts update processing
    pub async fn connect_with_bot(&self, session_path: &str, token: &str) -> Result<()> {
        info!(
            "Spectra: Connecting to Telegram MTProto session at '{}'...",
            session_path
        );
        let session = Arc::new(
            SqliteSession::open(session_path)
                .await
                .context("Failed to open session db")?,
        );

        let SenderPool {
            runner,
            updates,
            handle,
        } = SenderPool::new(Arc::clone(&session), self.api_id);

        let client = Client::new(handle.clone());
        tokio::spawn(runner.run());

        if !client.is_authorized().await.context("Auth check failed")? {
            info!("Spectra: Signing in via token...");
            client
                .bot_sign_in(token, &self.api_hash)
                .await
                .context("Bot sign-in failed")?;
            info!("Spectra: Signed in successfully!");
        }

        let mut lock = self.client.lock().await;
        *lock = Some(client.clone());
        drop(lock);

        let interceptor = Arc::new(UpdateInterceptor::new(
            self.storage.clone(),
            self.ghost.clone(),
            self.ui_tx.clone(),
        ));

        let mut update_stream = client
            .stream_updates(updates, UpdatesConfiguration { catch_up: true })
            .await
            .map_err(|e| anyhow::anyhow!("Failed to stream updates: {e}"))?;

        let _ = self
            .ui_tx
            .send(AppEvent::ConnectionStatusChanged(
                "Connected [MTProto]".to_string(),
            ))
            .await;

        tokio::spawn(async move {
            loop {
                match update_stream.next().await {
                    Ok(update) => {
                        if let Err(e) = interceptor.process_update(update).await {
                            error!("Spectra interceptor update error: {e}");
                        }
                    }
                    Err(e) => {
                        warn!("Spectra update notice: {e}");
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    }
                }
            }
        });

        // Sync dialogs initially
        self.sync_dialogs().await?;

        Ok(())
    }

    /// Fetches dialogs from Telegram and updates local storage
    pub async fn sync_dialogs(&self) -> Result<Vec<ChatSummary>> {
        let lock = self.client.lock().await;
        if let Some(client) = lock.as_ref() {
            let mut dialogs = client.iter_dialogs();
            while let Some(dialog) = dialogs.next().await? {
                let chat_id = dialog.peer.id().bot_api_dialog_id().unwrap_or(0);
                let title = dialog
                    .peer
                    .name()
                    .map(String::from)
                    .unwrap_or_else(|| format!("Chat {chat_id}"));
                let username = dialog.peer.username().map(String::from);
                let chat_type = if chat_id > 0 {
                    "user".to_string()
                } else {
                    "group".to_string()
                };
                let last_date = dialog.last_message.as_ref().map(|m| m.date().as_second());
                let preview = dialog.last_message.as_ref().map(|m| m.text().to_string());

                let summary = ChatSummary {
                    id: chat_id,
                    title,
                    username,
                    chat_type,
                    last_message_date: last_date,
                    last_message_preview: preview,
                    deleted_messages_count: 0,
                };
                let _ = self.storage.upsert_chat(&summary);
            }
        }

        let chats = self.storage.get_chats()?;
        let _ = self
            .ui_tx
            .send(AppEvent::DialogsUpdated(chats.clone()))
            .await;
        Ok(chats)
    }

    /// Sends a message stealthily (suppressing typing indicators if ghost mode is on)
    pub async fn send_message(&self, chat_id: i64, text: &str) -> Result<()> {
        let lock = self.client.lock().await;
        let client = lock.as_ref().context("Client not connected")?;

        if !self.ghost.should_send_typing() {
            info!(chat_id = chat_id, "Ghost Mode: Typing indicator suppressed");
        }

        if let Some(peer_id) = PeerId::from_bot_api_dialog_id(chat_id) {
            let peer_ref = peer_id.to_ambient_ref();
            client
                .send_message(peer_ref, text)
                .await
                .context("Failed to send message")?;
            info!(chat_id = chat_id, "Sent message stealthily");
        }

        Ok(())
    }

    /// Explicit manual "Mark as Read" (breaks ghost mode for this specific user action)
    pub async fn manual_mark_as_read(&self, chat_id: i64) -> Result<()> {
        let lock = self.client.lock().await;
        let client = lock.as_ref().context("Client not connected")?;

        if let Some(peer_id) = PeerId::from_bot_api_dialog_id(chat_id) {
            let peer_ref = peer_id.to_ambient_ref();
            client
                .mark_as_read(peer_ref)
                .await
                .context("Failed to mark read")?;
            info!(
                chat_id = chat_id,
                "Dispatched manual read receipt to server on user demand"
            );
        }

        Ok(())
    }
}
