use gtk4::prelude::*;
use gtk4::{Align, Box, Button, Entry, Label, Orientation, PolicyType, ScrolledWindow, Window};
use std::cell::RefCell;
use std::rc::Rc;

use crate::models::{ChatSummary, SpectraMessage};
use crate::privacy::GhostManager;
use crate::storage::Database;
use crate::telegram::TelegramService;
use crate::ui::revisions_dialog::show_revisions_dialog;

#[derive(Clone)]
pub struct ChatView {
    container: Box,
    header_title: Label,
    header_subtitle: Label,
    mark_read_btn: Button,
    purge_btn: Button,
    messages_box: Box,
    _scrolled_window: ScrolledWindow,
    entry: Entry,
    send_btn: Button,

    current_chat: Rc<RefCell<Option<ChatSummary>>>,
    storage: Database,
    _ghost: GhostManager,
    telegram: TelegramService,
}

impl ChatView {
    pub fn new(storage: Database, ghost: GhostManager, telegram: TelegramService) -> Self {
        let container = Box::new(Orientation::Vertical, 0);

        // Header
        let header_bar = Box::new(Orientation::Horizontal, 12);
        header_bar.set_margin_top(10);
        header_bar.set_margin_bottom(10);
        header_bar.set_margin_start(16);
        header_bar.set_margin_end(16);

        let title_box = Box::new(Orientation::Vertical, 2);
        let header_title = Label::new(Some("Select a chat"));
        header_title.add_css_class("heading");
        header_title.set_halign(Align::Start);

        let header_subtitle = Label::new(Some(
            "👻 Stealth View: Reading messages leaves them unread on Telegram",
        ));
        header_subtitle.add_css_class("caption");
        header_subtitle.add_css_class("dim-label");
        header_subtitle.set_halign(Align::Start);

        title_box.append(&header_title);
        title_box.append(&header_subtitle);
        title_box.set_hexpand(true);
        header_bar.append(&title_box);

        let mark_read_btn = Button::with_label("📩 Mark Read");
        mark_read_btn.set_tooltip_text(Some(
            "Manually dispatch read receipt to Telegram servers for this chat",
        ));
        header_bar.append(&mark_read_btn);

        let purge_btn = Button::with_label("🗑️ Purge Local");
        purge_btn.add_css_class("destructive-action");
        purge_btn.set_tooltip_text(Some("Permanently erase local cached history for this chat"));
        header_bar.append(&purge_btn);

        container.append(&header_bar);

        let sep = gtk4::Separator::new(Orientation::Horizontal);
        container.append(&sep);

        // Messages Box inside Scrolled Window
        let scrolled_window = ScrolledWindow::builder()
            .vexpand(true)
            .hscrollbar_policy(PolicyType::Never)
            .build();

        let messages_box = Box::new(Orientation::Vertical, 6);
        messages_box.set_margin_top(12);
        messages_box.set_margin_bottom(12);
        scrolled_window.set_child(Some(&messages_box));
        container.append(&scrolled_window);

        let sep2 = gtk4::Separator::new(Orientation::Horizontal);
        container.append(&sep2);

        // Bottom Composer
        let input_bar = Box::new(Orientation::Horizontal, 10);
        input_bar.set_margin_top(10);
        input_bar.set_margin_bottom(10);
        input_bar.set_margin_start(16);
        input_bar.set_margin_end(16);

        let entry = Entry::builder()
            .placeholder_text("Write a message (Stealth typing active: recipient sees nothing)...")
            .hexpand(true)
            .build();

        let send_btn = Button::builder()
            .label("🚀 Send")
            .css_classes(["suggested-action"])
            .build();

        input_bar.append(&entry);
        input_bar.append(&send_btn);
        container.append(&input_bar);

        let current_chat = Rc::new(RefCell::new(None));

        let view = Self {
            container,
            header_title,
            header_subtitle,
            mark_read_btn,
            purge_btn,
            messages_box,
            _scrolled_window: scrolled_window,
            entry,
            send_btn,
            current_chat,
            storage,
            _ghost: ghost,
            telegram,
        };

        view.setup_signals();
        view
    }

    pub fn widget(&self) -> &Box {
        &self.container
    }

    fn setup_signals(&self) {
        let current_chat = self.current_chat.clone();
        let tg = self.telegram.clone();
        self.mark_read_btn.connect_clicked(move |_| {
            if let Some(chat) = current_chat.borrow().as_ref() {
                let chat_id = chat.id;
                let t = tg.clone();
                glib::MainContext::default().spawn_local(async move {
                    let _ = t.manual_mark_as_read(chat_id).await;
                });
            }
        });

        let view_clone = self.clone();
        let storage = self.storage.clone();
        let current_chat = self.current_chat.clone();
        self.purge_btn.connect_clicked(move |_| {
            if let Some(chat) = current_chat.borrow().as_ref() {
                let _ = storage.purge_chat_history(chat.id);
                view_clone.reload_messages();
            }
        });

        let view_clone = self.clone();
        let tg = self.telegram.clone();
        let entry_clone = self.entry.clone();
        let current_chat = self.current_chat.clone();
        let on_send = move || {
            let text = entry_clone.text().to_string();
            if text.trim().is_empty() {
                return;
            }

            if let Some(chat) = current_chat.borrow().as_ref() {
                let chat_id = chat.id;
                let t = tg.clone();
                let txt = text.clone();
                let v = view_clone.clone();

                // Save locally first
                let new_id = chrono::Utc::now().timestamp() as i32;
                let new_msg = SpectraMessage {
                    id: new_id,
                    chat_id,
                    sender_id: None,
                    sender_name: "You".to_string(),
                    text: text.clone(),
                    date: chrono::Utc::now().timestamp(),
                    is_deleted: false,
                    deleted_at: None,
                    is_edited: false,
                    media_type: None,
                    outgoing: true,
                };
                let _ = v.storage.save_message(&new_msg);
                v.append_message(&new_msg);

                entry_clone.set_text("");

                glib::MainContext::default().spawn_local(async move {
                    let _ = t.send_message(chat_id, &txt).await;
                });
            }
        };

        let send_fn = on_send.clone();
        self.send_btn.connect_clicked(move |_| {
            send_fn();
        });

        self.entry.connect_activate(move |_| {
            on_send();
        });
    }

    pub fn set_chat(&self, chat: ChatSummary) {
        self.header_title.set_text(&chat.title);
        let subtitle = if let Some(u) = &chat.username {
            format!("@{} | 👻 Stealth Active", u)
        } else {
            format!("ID: {} | 👻 Stealth Active", chat.id)
        };
        self.header_subtitle.set_text(&subtitle);

        *self.current_chat.borrow_mut() = Some(chat);
        self.reload_messages();
    }

    pub fn reload_messages(&self) {
        // Clear existing messages
        while let Some(child) = self.messages_box.first_child() {
            self.messages_box.remove(&child);
        }

        if let Some(chat) = self.current_chat.borrow().as_ref() {
            if let Ok(messages) = self.storage.get_chat_messages(chat.id, 100) {
                for msg in messages {
                    self.append_message(&msg);
                }
            }
        }
    }

    pub fn append_message(&self, msg: &SpectraMessage) {
        let is_outgoing = msg.outgoing;
        let bubble = Box::new(Orientation::Vertical, 4);
        bubble.add_css_class("msg-bubble");

        if msg.is_deleted {
            bubble.add_css_class("msg-deleted");
        } else if is_outgoing {
            bubble.add_css_class("msg-outgoing");
        } else {
            bubble.add_css_class("msg-incoming");
        }

        bubble.set_halign(if is_outgoing {
            Align::End
        } else {
            Align::Start
        });

        // Header row inside bubble: Sender + Time
        let meta_row = Box::new(Orientation::Horizontal, 8);
        let sender_lbl = Label::new(Some(&msg.sender_name));
        sender_lbl.add_css_class("caption");
        sender_lbl.add_css_class("heading");

        let time_lbl = Label::new(Some(&msg.formatted_date()));
        time_lbl.add_css_class("caption");
        time_lbl.add_css_class("dim-label");

        meta_row.append(&sender_lbl);
        meta_row.append(&time_lbl);
        bubble.append(&meta_row);

        // Anti-Recall Alert Tag
        if msg.is_deleted {
            let del_time = msg
                .formatted_deleted_at()
                .unwrap_or_else(|| "N/A".to_string());
            let badge = Label::new(Some(&format!(
                "🚨 DELETED by sender at {} [PRESERVED]",
                del_time
            )));
            badge.add_css_class("badge-deleted");
            badge.set_halign(Align::Start);
            bubble.append(&badge);
        }

        // Message Text
        let text_lbl = Label::new(Some(&msg.text));
        text_lbl.set_wrap(true);
        text_lbl.set_selectable(true);
        text_lbl.set_halign(Align::Start);
        if msg.is_deleted {
            text_lbl.add_css_class("msg-deleted-text");
        }
        bubble.append(&text_lbl);

        // Anti-Edit Badge & Revisions Button
        if msg.is_edited {
            let edit_row = Box::new(Orientation::Horizontal, 6);
            let edit_badge = Label::new(Some("✏️ EDITED"));
            edit_badge.add_css_class("badge-edited");
            edit_row.append(&edit_badge);

            let rev_btn = Button::builder()
                .label("View Revisions")
                .has_frame(false)
                .css_classes(["caption", "suggested-action"])
                .build();

            let storage = self.storage.clone();
            let chat_id = msg.chat_id;
            let msg_id = msg.id;
            let b_clone = bubble.clone();

            rev_btn.connect_clicked(move |_| {
                if let Ok(revs) = storage.get_message_revisions(chat_id, msg_id) {
                    if let Some(root) = b_clone.root() {
                        if let Ok(window) = root.downcast::<Window>() {
                            show_revisions_dialog(&window, msg_id, revs);
                        }
                    }
                }
            });

            edit_row.append(&rev_btn);
            bubble.append(&edit_row);
        }

        self.messages_box.append(&bubble);
    }
}
