use gtk4::pango::EllipsizeMode;
use gtk4::prelude::*;
use gtk4::{
    Align, Box, Entry, Label, ListBox, ListBoxRow, Orientation, PolicyType, ScrolledWindow,
};
use std::cell::RefCell;
use std::rc::Rc;

use crate::models::ChatSummary;

#[derive(Clone)]
pub struct DialogsList {
    container: Box,
    search_entry: Entry,
    list_box: ListBox,
    all_chats: Rc<RefCell<Vec<ChatSummary>>>,
    filtered_chats: Rc<RefCell<Vec<ChatSummary>>>,
    selected_chat_id: Rc<RefCell<Option<i64>>>,
    on_select: Rc<dyn Fn(ChatSummary)>,
}

impl DialogsList {
    pub fn new<F>(on_select: F) -> Self
    where
        F: Fn(ChatSummary) + 'static,
    {
        let container = Box::new(Orientation::Vertical, 6);
        container.set_width_request(280);

        // Header / Search
        let search_box = Box::new(Orientation::Horizontal, 6);
        search_box.set_margin_top(8);
        search_box.set_margin_bottom(6);
        search_box.set_margin_start(10);
        search_box.set_margin_end(10);

        let search_entry = Entry::builder()
            .placeholder_text("🔍 Search chats...")
            .hexpand(true)
            .build();
        search_box.append(&search_entry);
        container.append(&search_box);

        // List Box inside ScrolledWindow
        let scrolled = ScrolledWindow::builder()
            .vexpand(true)
            .hscrollbar_policy(PolicyType::Never)
            .build();

        let list_box = ListBox::builder()
            .selection_mode(gtk4::SelectionMode::Single)
            .build();
        list_box.add_css_class("navigation-sidebar");

        scrolled.set_child(Some(&list_box));
        container.append(&scrolled);

        let all_chats = Rc::new(RefCell::new(Vec::new()));
        let filtered_chats = Rc::new(RefCell::new(Vec::new()));
        let selected_chat_id = Rc::new(RefCell::new(None));
        let on_select = Rc::new(on_select) as Rc<dyn Fn(ChatSummary)>;

        let dialogs = Self {
            container,
            search_entry,
            list_box,
            all_chats,
            filtered_chats,
            selected_chat_id,
            on_select,
        };

        dialogs.setup_signals();
        dialogs
    }

    pub fn widget(&self) -> &Box {
        &self.container
    }

    fn setup_signals(&self) {
        let d_clone = self.clone();
        self.search_entry.connect_changed(move |_| {
            d_clone.apply_filter();
        });

        let filtered = self.filtered_chats.clone();
        let on_sel = self.on_select.clone();
        let sel_id = self.selected_chat_id.clone();
        self.list_box.connect_row_activated(move |_, row| {
            let idx = row.index();
            if idx >= 0 {
                let chats = filtered.borrow();
                if let Some(chat) = chats.get(idx as usize) {
                    *sel_id.borrow_mut() = Some(chat.id);
                    on_sel(chat.clone());
                }
            }
        });
    }

    pub fn set_chats(&self, chats: Vec<ChatSummary>) {
        *self.all_chats.borrow_mut() = chats;
        self.apply_filter();
    }

    pub fn update_chat(&self, updated: ChatSummary) {
        let mut chats = self.all_chats.borrow_mut();
        if let Some(existing) = chats.iter_mut().find(|c| c.id == updated.id) {
            *existing = updated;
        } else {
            chats.insert(0, updated);
        }
        drop(chats);
        self.apply_filter();
    }

    pub fn select_first_if_none(&self) {
        if self.selected_chat_id.borrow().is_none() {
            let chats = self.filtered_chats.borrow();
            if let Some(first) = chats.first() {
                *self.selected_chat_id.borrow_mut() = Some(first.id);
                (self.on_select)(first.clone());

                if let Some(row) = self.list_box.row_at_index(0) {
                    self.list_box.select_row(Some(&row));
                }
            }
        }
    }

    fn apply_filter(&self) {
        let query = self.search_entry.text().to_lowercase();
        let all = self.all_chats.borrow();

        let filtered: Vec<ChatSummary> = all
            .iter()
            .filter(|c| {
                if query.is_empty() {
                    true
                } else {
                    c.title.to_lowercase().contains(&query)
                        || c.username
                            .as_ref()
                            .map(|u| u.to_lowercase().contains(&query))
                            .unwrap_or(false)
                }
            })
            .cloned()
            .collect();

        *self.filtered_chats.borrow_mut() = filtered.clone();

        // Clear existing rows
        while let Some(child) = self.list_box.first_child() {
            self.list_box.remove(&child);
        }

        let cur_id = *self.selected_chat_id.borrow();
        let mut row_to_select = None;

        for (idx, chat) in filtered.iter().enumerate() {
            let row = ListBoxRow::new();
            row.add_css_class("chat-list-row");

            let row_box = Box::new(Orientation::Horizontal, 10);
            row_box.set_margin_top(6);
            row_box.set_margin_bottom(6);
            row_box.set_margin_start(8);
            row_box.set_margin_end(8);

            // Avatar initial
            let initial = chat
                .title
                .chars()
                .next()
                .unwrap_or('?')
                .to_uppercase()
                .to_string();
            let avatar = Label::new(Some(&initial));
            avatar.add_css_class("avatar-circle");
            avatar.set_halign(Align::Center);
            avatar.set_valign(Align::Center);
            row_box.append(&avatar);

            // Info column
            let info_col = Box::new(Orientation::Vertical, 2);
            info_col.set_hexpand(true);

            // Row 1: Title + badges
            let title_row = Box::new(Orientation::Horizontal, 4);
            let title_lbl = Label::new(Some(&chat.title));
            title_lbl.add_css_class("heading");
            title_lbl.set_ellipsize(EllipsizeMode::End);
            title_lbl.set_halign(Align::Start);
            title_lbl.set_hexpand(true);
            title_row.append(&title_lbl);

            if chat.deleted_messages_count > 0 {
                let badge = Label::new(Some(&format!("🛡️ {}", chat.deleted_messages_count)));
                badge.add_css_class("chat-deleted-badge");
                badge.set_tooltip_text(Some(&format!(
                    "{} deleted messages captured and preserved locally",
                    chat.deleted_messages_count
                )));
                title_row.append(&badge);
            }
            info_col.append(&title_row);

            // Row 2: Preview / Username
            let preview_text = if let Some(preview) = &chat.last_message_preview {
                preview.clone()
            } else if let Some(username) = &chat.username {
                format!("@{}", username)
            } else {
                chat.chat_type.to_string()
            };

            let preview_lbl = Label::new(Some(&preview_text));
            preview_lbl.add_css_class("caption");
            preview_lbl.add_css_class("dim-label");
            preview_lbl.set_ellipsize(EllipsizeMode::End);
            preview_lbl.set_halign(Align::Start);
            info_col.append(&preview_lbl);

            row_box.append(&info_col);
            row.set_child(Some(&row_box));
            self.list_box.append(&row);

            if Some(chat.id) == cur_id {
                row_to_select = Some(idx as i32);
            }
        }

        if let Some(idx) = row_to_select {
            if let Some(row) = self.list_box.row_at_index(idx) {
                self.list_box.select_row(Some(&row));
            }
        }
    }
}
