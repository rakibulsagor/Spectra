use anyhow::Result;
use async_channel::Sender;
use grammers_client::update::{Message, MessageDeletion, Update};
use tracing::{info, warn};

use crate::models::{AppEvent, SpectraMessage};
use crate::privacy::GhostManager;
use crate::storage::Database;

pub struct UpdateInterceptor {
    storage: Database,
    ghost: GhostManager,
    ui_tx: Sender<AppEvent>,
}

impl UpdateInterceptor {
    pub fn new(storage: Database, ghost: GhostManager, ui_tx: Sender<AppEvent>) -> Self {
        Self {
            storage,
            ghost,
            ui_tx,
        }
    }

    pub async fn process_update(&self, update: Update) -> Result<()> {
        match update {
            Update::NewMessage(msg) => {
                self.handle_new_message(msg).await?;
            }
            Update::MessageEdited(msg) => {
                self.handle_message_edited(msg).await?;
            }
            Update::MessageDeleted(deletion) => {
                self.handle_message_deleted(deletion).await?;
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_new_message(&self, msg: Message) -> Result<()> {
        let chat_id = msg.peer_id().bot_api_dialog_id().unwrap_or(0);
        let sender_id = msg.sender_id().and_then(|id| id.bot_api_dialog_id());
        let sender_name = msg
            .sender()
            .and_then(|p| p.name().map(String::from))
            .unwrap_or_else(|| {
                if msg.outgoing() {
                    "You".to_string()
                } else {
                    format!("User_{}", sender_id.unwrap_or(0))
                }
            });

        let media_type = msg.media().map(|m| format!("{:?}", m));

        let spectra_msg = SpectraMessage {
            id: msg.id(),
            chat_id,
            sender_id,
            sender_name,
            text: msg.text().to_string(),
            date: msg.date().as_second(),
            is_deleted: false,
            deleted_at: None,
            is_edited: false,
            media_type,
            outgoing: msg.outgoing(),
        };

        self.storage.save_message(&spectra_msg)?;
        let _ = self.ui_tx.send(AppEvent::NewMessage(spectra_msg)).await;
        Ok(())
    }

    async fn handle_message_edited(&self, msg: Message) -> Result<()> {
        let chat_id = msg.peer_id().bot_api_dialog_id().unwrap_or(0);
        let message_id = msg.id();
        let new_text = msg.text().to_string();

        if self.ghost.is_anti_edit_enabled() {
            if let Some(old_text) = self
                .storage
                .record_message_edit(chat_id, message_id, &new_text)?
            {
                info!(
                    chat_id = chat_id,
                    msg_id = message_id,
                    "Anti-Edit: Captured modified message"
                );
                let _ = self
                    .ui_tx
                    .send(AppEvent::MessageEdited {
                        chat_id,
                        message_id,
                        old_text,
                        new_text,
                    })
                    .await;
            } else {
                let spectra_msg = SpectraMessage {
                    id: message_id,
                    chat_id,
                    sender_id: msg.sender_id().and_then(|id| id.bot_api_dialog_id()),
                    sender_name: "Unknown".to_string(),
                    text: new_text,
                    date: msg.date().as_second(),
                    is_deleted: false,
                    deleted_at: None,
                    is_edited: true,
                    media_type: None,
                    outgoing: msg.outgoing(),
                };
                self.storage.save_message(&spectra_msg)?;
            }
        }
        Ok(())
    }

    async fn handle_message_deleted(&self, deletion: MessageDeletion) -> Result<()> {
        let channel_id = deletion.channel_id();
        let deleted_msg_ids = deletion.messages();

        if self.ghost.is_anti_recall_enabled() {
            for &msg_id in deleted_msg_ids {
                let chat_id = channel_id.unwrap_or(0);
                if let Some(preserved) = self.storage.mark_message_deleted(chat_id, msg_id)? {
                    warn!(
                        chat_id = chat_id,
                        msg_id = msg_id,
                        sender = %preserved.sender_name,
                        "Anti-Recall: Intercepted deleted message and preserved locally"
                    );

                    let _ = self
                        .ui_tx
                        .send(AppEvent::MessageDeleted {
                            chat_id,
                            message_id: msg_id,
                            preserved: Some(preserved),
                        })
                        .await;
                }
            }
        }
        Ok(())
    }
}
