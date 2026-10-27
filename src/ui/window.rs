use adw::prelude::*;
use adw::{ApplicationWindow, HeaderBar};
use gtk4::{Align, Box, Button, Label, Orientation, Paned, ToggleButton, Window};
use std::cell::RefCell;
use std::rc::Rc;

use crate::models::{AppEvent, ChatSummary};
use crate::privacy::GhostManager;
use crate::storage::Database;
use crate::telegram::TelegramService;
use crate::ui::chat_view::ChatView;
use crate::ui::deleted_dialog::show_deleted_messages_dialog;
use crate::ui::dialogs_list::DialogsList;
use crate::ui::settings::show_settings_window;

pub struct SpectraWindow {
    window: ApplicationWindow,
    dialogs_list: DialogsList,
    chat_view: ChatView,
    status_label: Label,
    ghost_toggle: ToggleButton,
    storage: Database,
    ghost: GhostManager,
    current_chat: Rc<RefCell<Option<ChatSummary>>>,
}

impl SpectraWindow {
    pub fn new(
        app: &adw::Application,
        storage: Database,
        ghost: GhostManager,
        telegram: TelegramService,
    ) -> Rc<Self> {
        let window = ApplicationWindow::builder()
            .application(app)
            .title("Spectra — Native Privacy Telegram Client")
            .default_width(1080)
            .default_height(720)
            .build();

        let header_bar = HeaderBar::new();

        // Status Label (Left of header)
        let status_box = Box::new(Orientation::Horizontal, 8);
        status_box.set_margin_start(8);

        let status_label = Label::new(Some("🟢 Connected"));
        status_label.add_css_class("caption");
        status_label.add_css_class("dim-label");
        status_box.append(&status_label);

        // Ghost Mode Quick Toggle
        let is_ghost_on = ghost.is_ghost_mode_enabled();
        let ghost_toggle = ToggleButton::builder()
            .label("👻 Ghost Mode")
            .active(is_ghost_on)
            .tooltip_text("Toggle Ghost Mode (Anti-Read, Anti-Typing, Anti-Online)")
            .build();
        if is_ghost_on {
            ghost_toggle.add_css_class("ghost-active");
        }
        status_box.append(&ghost_toggle);

        header_bar.pack_start(&status_box);

        // Right of header: Anti-Recall Audit & Settings
        let actions_box = Box::new(Orientation::Horizontal, 6);
        actions_box.set_margin_end(8);

        let anti_recall_btn = Button::builder()
            .label("🛡️ Anti-Recall Log")
            .tooltip_text("View all messages preserved locally after sender deleted them")
            .build();
        anti_recall_btn.add_css_class("suggested-action");
        actions_box.append(&anti_recall_btn);

        let settings_btn = Button::builder()
            .label("⚙️ Settings")
            .tooltip_text("Spectra Privacy, Anti-Recall, and Network Settings")
            .build();
        actions_box.append(&settings_btn);

        header_bar.pack_end(&actions_box);

        // Main Layout: Top stealth banner + Paned (Chats | ChatView)
        let main_box = Box::new(Orientation::Vertical, 0);
        main_box.append(&header_bar);

        let banner = Label::new(Some(
            "👻 Stealth Active: Reading messages never marks them read. Typing suppressed. Online packets blocked. All deleted messages preserved.",
        ));
        banner.add_css_class("stealth-status-banner");
        banner.set_halign(Align::Fill);
        banner.set_justify(gtk4::Justification::Center);
        main_box.append(&banner);

        let current_chat = Rc::new(RefCell::new(None));

        // Create ChatView
        let chat_view = ChatView::new(storage.clone(), ghost.clone(), telegram.clone());

        // Create DialogsList
        let cv_clone = chat_view.clone();
        let cur_chat_clone = current_chat.clone();
        let dialogs_list = DialogsList::new(move |chat: ChatSummary| {
            *cur_chat_clone.borrow_mut() = Some(chat.clone());
            cv_clone.set_chat(chat);
        });

        // Paned layout
        let paned = Paned::new(Orientation::Horizontal);
        paned.set_start_child(Some(dialogs_list.widget()));
        paned.set_end_child(Some(chat_view.widget()));
        paned.set_position(320);
        paned.set_wide_handle(true);
        paned.set_vexpand(true);
        paned.set_hexpand(true);

        main_box.append(&paned);
        window.set_content(Some(&main_box));

        let app_window = Rc::new(Self {
            window,
            dialogs_list,
            chat_view,
            status_label,
            ghost_toggle,
            storage,
            ghost,
            current_chat,
        });

        app_window.setup_actions(anti_recall_btn, settings_btn);
        app_window
    }

    fn setup_actions(&self, anti_recall_btn: Button, settings_btn: Button) {
        let storage = self.storage.clone();
        let win = self.window.clone();
        anti_recall_btn.connect_clicked(move |_| {
            if let Ok(deleted) = storage.get_deleted_messages(None, Some(200)) {
                let gtk_win: Window = win.clone().upcast();
                show_deleted_messages_dialog(&gtk_win, deleted);
            }
        });

        let ghost = self.ghost.clone();
        let win2 = self.window.clone();
        settings_btn.connect_clicked(move |_| {
            let gtk_win: Window = win2.clone().upcast();
            show_settings_window(&gtk_win, ghost.clone());
        });

        let ghost_mgr = self.ghost.clone();
        let g_toggle = self.ghost_toggle.clone();
        self.ghost_toggle.connect_toggled(move |btn| {
            let is_active = btn.is_active();
            ghost_mgr.set_ghost_mode_enabled(is_active);
            if is_active {
                g_toggle.add_css_class("ghost-active");
            } else {
                g_toggle.remove_css_class("ghost-active");
            }
        });
    }

    pub fn present(&self) {
        self.window.present();
    }

    pub fn handle_event(&self, event: AppEvent) {
        match event {
            AppEvent::NewMessage(msg) => {
                let cur_chat_id = self.current_chat.borrow().as_ref().map(|c| c.id);
                if Some(msg.chat_id) == cur_chat_id {
                    self.chat_view.append_message(&msg);
                }

                // Update chat in sidebar
                if let Ok(Some(mut chat)) = self.storage.get_chat(msg.chat_id) {
                    chat.last_message_preview = Some(msg.text.clone());
                    chat.last_message_date = Some(msg.date);
                    self.dialogs_list.update_chat(chat);
                }
            }
            AppEvent::MessageDeleted {
                chat_id,
                message_id: _,
                preserved: _,
            } => {
                let cur_chat_id = self.current_chat.borrow().as_ref().map(|c| c.id);
                if Some(chat_id) == cur_chat_id {
                    self.chat_view.reload_messages();
                }

                if let Ok(Some(mut chat)) = self.storage.get_chat(chat_id) {
                    if let Ok(deleted_list) =
                        self.storage.get_deleted_messages(Some(chat_id), Some(500))
                    {
                        chat.deleted_messages_count = deleted_list.len();
                        self.dialogs_list.update_chat(chat);
                    }
                }
            }
            AppEvent::MessageEdited {
                chat_id,
                message_id: _,
                old_text: _,
                new_text: _,
            } => {
                let cur_chat_id = self.current_chat.borrow().as_ref().map(|c| c.id);
                if Some(chat_id) == cur_chat_id {
                    self.chat_view.reload_messages();
                }
            }
            AppEvent::DialogsUpdated(chats) => {
                self.dialogs_list.set_chats(chats);
                self.dialogs_list.select_first_if_none();
            }
            AppEvent::ConnectionStatusChanged(status) => {
                self.status_label.set_text(&status);
            }
            AppEvent::GhostModeToggled(enabled) => {
                self.ghost_toggle.set_active(enabled);
                if enabled {
                    self.ghost_toggle.add_css_class("ghost-active");
                } else {
                    self.ghost_toggle.remove_css_class("ghost-active");
                }
            }
        }
    }

    pub fn load_initial_data(&self) {
        if let Ok(chats) = self.storage.get_chats() {
            if !chats.is_empty() {
                self.dialogs_list.set_chats(chats);
                self.dialogs_list.select_first_if_none();
            }
        }
    }
}
