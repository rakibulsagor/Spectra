use adw::prelude::*;
use anyhow::Result;
use async_channel::unbounded;
use chrono::Utc;
use std::path::PathBuf;
use tracing::{error, info};

use crate::models::{AppEvent, ChatSummary, SpectraMessage};
use crate::privacy::GhostManager;
use crate::storage::Database;
use crate::telegram::TelegramService;
use crate::ui::style::load_css;
use crate::ui::SpectraWindow;

pub struct App {
    adw_app: adw::Application,
    storage: Database,
    ghost: GhostManager,
    telegram: TelegramService,
    ui_rx: async_channel::Receiver<AppEvent>,
}

impl App {
    pub fn new(db_path: PathBuf, api_id: i32, api_hash: String) -> Result<Self> {
        let storage = Database::open(&db_path)?;
        let ghost = GhostManager::new(storage.clone());
        let (ui_tx, ui_rx) = unbounded::<AppEvent>();

        let telegram =
            TelegramService::new(storage.clone(), ghost.clone(), ui_tx, api_id, api_hash);

        let adw_app = adw::Application::builder()
            .application_id("org.spectra.Spectra")
            .build();

        Ok(Self {
            adw_app,
            storage,
            ghost,
            telegram,
            ui_rx,
        })
    }

    pub fn run(self, bot_token: Option<String>, session_path: Option<String>) -> glib::ExitCode {
        let storage = self.storage.clone();
        let ghost = self.ghost.clone();
        let telegram = self.telegram.clone();
        let ui_rx = self.ui_rx.clone();

        self.adw_app.connect_activate(move |adw_app| {
            load_css();

            // Seed demo data if database is brand new so the UI is immediately functional and reviewable
            seed_demo_data_if_empty(&storage);

            let window =
                SpectraWindow::new(adw_app, storage.clone(), ghost.clone(), telegram.clone());

            window.load_initial_data();
            window.present();

            // Setup async event receiver on GTK main loop
            let rx = ui_rx.clone();
            let win = window.clone();
            glib::MainContext::default().spawn_local(async move {
                while let Ok(event) = rx.recv().await {
                    win.handle_event(event);
                }
            });

            // Start Telegram MTProto connection in background if token is provided
            if let Some(token) = bot_token.clone() {
                let s_path = session_path
                    .clone()
                    .unwrap_or_else(|| "spectra_session.db".to_string());
                let tg = telegram.clone();

                tokio::spawn(async move {
                    if let Err(e) = tg.connect_with_bot(&s_path, &token).await {
                        error!("Spectra MTProto connection error: {e:#}");
                    }
                });
            }
        });

        self.adw_app.run()
    }
}

/// Seeds realistic sample chats with Anti-Recall and Anti-Edit data if database is empty
fn seed_demo_data_if_empty(storage: &Database) {
    if let Ok(chats) = storage.get_chats() {
        if !chats.is_empty() {
            return;
        }
    }

    info!("Seeding demo conversations demonstrating Ghost Mode, Anti-Recall, and Anti-Edit...");

    let now = Utc::now().timestamp();

    // 1. Alice (Anti-Recall Demo)
    let chat1 = ChatSummary {
        id: 1001,
        title: "Alice [Anti-Recall Demo]".to_string(),
        username: Some("alice_crypto".to_string()),
        chat_type: "user".to_string(),
        last_message_date: Some(now - 60),
        last_message_preview: Some("Did you catch that before I deleted it? 😉".to_string()),
        deleted_messages_count: 1,
    };
    let _ = storage.upsert_chat(&chat1);

    let m1 = SpectraMessage {
        id: 1,
        chat_id: 1001,
        sender_id: Some(1001),
        sender_name: "Alice".to_string(),
        text: "Hey! Did you check out the new Spectra client?".to_string(),
        date: now - 300,
        is_deleted: false,
        deleted_at: None,
        is_edited: false,
        media_type: None,
        outgoing: false,
    };
    let _ = storage.save_message(&m1);

    let m2 = SpectraMessage {
        id: 2,
        chat_id: 1001,
        sender_id: Some(1001),
        sender_name: "Alice".to_string(),
        text: "Wait, let me send you the secret code and delete it immediately for everyone:"
            .to_string(),
        date: now - 240,
        is_deleted: false,
        deleted_at: None,
        is_edited: false,
        media_type: None,
        outgoing: false,
    };
    let _ = storage.save_message(&m2);

    let m3 = SpectraMessage {
        id: 3,
        chat_id: 1001,
        sender_id: Some(1001),
        sender_name: "Alice".to_string(),
        text: "SUPER_SECRET_TOKEN_987654 (Sender deleted this from Telegram servers!)".to_string(),
        date: now - 180,
        is_deleted: true,
        deleted_at: Some(now - 120),
        is_edited: false,
        media_type: None,
        outgoing: false,
    };
    let _ = storage.save_message(&m3);

    let m4 = SpectraMessage {
        id: 4,
        chat_id: 1001,
        sender_id: Some(1001),
        sender_name: "Alice".to_string(),
        text: "Did you catch that before I deleted it? 😉 (Saved in Spectra Anti-Recall!)"
            .to_string(),
        date: now - 60,
        is_deleted: false,
        deleted_at: None,
        is_edited: false,
        media_type: None,
        outgoing: false,
    };
    let _ = storage.save_message(&m4);

    // 2. Dev Team (Anti-Edit Demo)
    let chat2 = ChatSummary {
        id: 1002,
        title: "Spectra Core Team [Anti-Edit]".to_string(),
        username: Some("spectra_devs".to_string()),
        chat_type: "group".to_string(),
        last_message_date: Some(now - 30),
        last_message_preview: Some("Check out the revisions history in GTK4!".to_string()),
        deleted_messages_count: 0,
    };
    let _ = storage.upsert_chat(&chat2);

    let edit_msg = SpectraMessage {
        id: 10,
        chat_id: 1002,
        sender_id: Some(2001),
        sender_name: "Lead Architect".to_string(),
        text: "Status Update: v1.0.0 will be delayed until next month due to C++ Qt migration."
            .to_string(),
        date: now - 500,
        is_deleted: false,
        deleted_at: None,
        is_edited: false,
        media_type: None,
        outgoing: false,
    };
    let _ = storage.save_message(&edit_msg);

    // Record past edit revisions
    let _ = storage.record_message_edit(
        1002,
        10,
        "Status Update: v1.0.0 rewritten in pure Rust, targeting release this Friday.",
    );
    let _ = storage.record_message_edit(
        1002,
        10,
        "Status Update: v1.0.0 is officially released with full libadwaita UI.",
    );

    let edit_msg2 = SpectraMessage {
        id: 11,
        chat_id: 1002,
        sender_id: Some(2002),
        sender_name: "Security Lead".to_string(),
        text: "Check out the revisions history in GTK4! Click 'View Revisions' above.".to_string(),
        date: now - 30,
        is_deleted: false,
        deleted_at: None,
        is_edited: false,
        media_type: None,
        outgoing: false,
    };
    let _ = storage.save_message(&edit_msg2);

    // 3. Bob (Ghost Mode / Stealth Demo)
    let chat3 = ChatSummary {
        id: 1003,
        title: "Bob [Ghost Mode Demo]".to_string(),
        username: Some("bob_privacy".to_string()),
        chat_type: "user".to_string(),
        last_message_date: Some(now - 10),
        last_message_preview: Some("Ghost mode is completely silent.".to_string()),
        deleted_messages_count: 0,
    };
    let _ = storage.upsert_chat(&chat3);

    let b1 = SpectraMessage {
        id: 20,
        chat_id: 1003,
        sender_id: Some(1003),
        sender_name: "Bob".to_string(),
        text: "Are you online right now? I don't see any read ticks or online status!".to_string(),
        date: now - 120,
        is_deleted: false,
        deleted_at: None,
        is_edited: false,
        media_type: None,
        outgoing: false,
    };
    let _ = storage.save_message(&b1);

    let b2 = SpectraMessage {
        id: 21,
        chat_id: 1003,
        sender_id: None,
        sender_name: "You".to_string(),
        text: "Ghost mode is completely silent. Reading leaves messages unread on server."
            .to_string(),
        date: now - 10,
        is_deleted: false,
        deleted_at: None,
        is_edited: false,
        media_type: None,
        outgoing: true,
    };
    let _ = storage.save_message(&b2);
}
