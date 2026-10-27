use adw::prelude::*;
use gtk4::{Box, Label, Orientation, ScrolledWindow, Window};

use crate::models::SpectraMessage;

pub fn show_deleted_messages_dialog(parent: &Window, deleted_messages: Vec<SpectraMessage>) {
    let dialog = adw::Window::builder()
        .transient_for(parent)
        .modal(true)
        .title("🛡️ Anti-Recall Log: Preserved Messages")
        .default_width(600)
        .default_height(450)
        .build();

    let content_box = Box::new(Orientation::Vertical, 12);
    content_box.set_margin_top(16);
    content_box.set_margin_bottom(16);
    content_box.set_margin_start(16);
    content_box.set_margin_end(16);

    let count_desc = format!(
        "Captured {} messages that senders tried to delete on Telegram:",
        deleted_messages.len()
    );
    let header_lbl = Label::new(Some(&count_desc));
    header_lbl.add_css_class("dim-label");
    header_lbl.set_halign(gtk4::Align::Start);
    content_box.append(&header_lbl);

    let scrolled = ScrolledWindow::builder()
        .vexpand(true)
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .build();

    let list_box = Box::new(Orientation::Vertical, 8);

    if deleted_messages.is_empty() {
        let empty_lbl = Label::new(Some("No deleted messages captured so far."));
        empty_lbl.add_css_class("dim-label");
        list_box.append(&empty_lbl);
    } else {
        for (idx, msg) in deleted_messages.iter().enumerate() {
            let card = Box::new(Orientation::Vertical, 6);
            card.add_css_class("card");
            card.add_css_class("msg-deleted");
            card.set_margin_start(4);
            card.set_margin_end(4);

            let row = Box::new(Orientation::Horizontal, 8);
            let meta_lbl = Label::new(Some(&format!(
                "#{}. From: {} [Chat ID: {} | Msg #{}]",
                idx + 1,
                msg.sender_name,
                msg.chat_id,
                msg.id
            )));
            meta_lbl.add_css_class("heading");
            meta_lbl.set_halign(gtk4::Align::Start);

            let del_time = msg
                .formatted_deleted_at()
                .unwrap_or_else(|| "N/A".to_string());
            let del_lbl = Label::new(Some(&format!("Deleted: {}", del_time)));
            del_lbl.add_css_class("badge-deleted");
            del_lbl.set_hexpand(true);
            del_lbl.set_halign(gtk4::Align::End);

            row.append(&meta_lbl);
            row.append(&del_lbl);
            card.append(&row);

            let text_lbl = Label::new(Some(&msg.text));
            text_lbl.set_wrap(true);
            text_lbl.set_selectable(true);
            text_lbl.set_halign(gtk4::Align::Start);
            card.append(&text_lbl);

            let sent_lbl = Label::new(Some(&format!(
                "Originally sent at: {}",
                msg.formatted_full_date()
            )));
            sent_lbl.add_css_class("dim-label");
            sent_lbl.add_css_class("caption");
            sent_lbl.set_halign(gtk4::Align::Start);
            card.append(&sent_lbl);

            list_box.append(&card);
        }
    }

    scrolled.set_child(Some(&list_box));
    content_box.append(&scrolled);

    let close_btn = gtk4::Button::with_label("Close");
    let d = dialog.clone();
    close_btn.connect_clicked(move |_| {
        d.close();
    });
    content_box.append(&close_btn);

    dialog.set_content(Some(&content_box));
    dialog.present();
}
