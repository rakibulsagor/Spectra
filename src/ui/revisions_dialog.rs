use adw::prelude::*;
use gtk4::{Box, Label, Orientation, ScrolledWindow, Window};

use crate::models::MessageRevision;

pub fn show_revisions_dialog(parent: &Window, message_id: i32, revisions: Vec<MessageRevision>) {
    let dialog = adw::Window::builder()
        .transient_for(parent)
        .modal(true)
        .title(format!("📝 Edit Revisions: Message #{}", message_id))
        .default_width(450)
        .default_height(350)
        .build();

    let content_box = Box::new(Orientation::Vertical, 12);
    content_box.set_margin_top(16);
    content_box.set_margin_bottom(16);
    content_box.set_margin_start(16);
    content_box.set_margin_end(16);

    let header_desc = Label::new(Some(
        "Prior drafts captured before the message was modified:",
    ));
    header_desc.add_css_class("dim-label");
    header_desc.set_halign(gtk4::Align::Start);
    content_box.append(&header_desc);

    let scrolled = ScrolledWindow::builder()
        .vexpand(true)
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .build();

    let list_box = Box::new(Orientation::Vertical, 8);

    if revisions.is_empty() {
        let empty_lbl = Label::new(Some("No previous revisions found for this message."));
        empty_lbl.add_css_class("dim-label");
        list_box.append(&empty_lbl);
    } else {
        for (idx, rev) in revisions.iter().enumerate() {
            let rev_card = Box::new(Orientation::Vertical, 4);
            rev_card.add_css_class("card");
            rev_card.set_margin_start(4);
            rev_card.set_margin_end(4);

            let header_row = Box::new(Orientation::Horizontal, 8);
            let draft_lbl = Label::new(Some(&format!("Draft #{}", idx + 1)));
            draft_lbl.add_css_class("heading");
            draft_lbl.set_halign(gtk4::Align::Start);

            let time_lbl = Label::new(Some(&rev.formatted_edited_at()));
            time_lbl.add_css_class("dim-label");
            time_lbl.add_css_class("caption");
            time_lbl.set_hexpand(true);
            time_lbl.set_halign(gtk4::Align::End);

            header_row.append(&draft_lbl);
            header_row.append(&time_lbl);
            rev_card.append(&header_row);

            let text_lbl = Label::new(Some(&rev.old_text));
            text_lbl.set_wrap(true);
            text_lbl.set_halign(gtk4::Align::Start);
            text_lbl.set_selectable(true);
            rev_card.append(&text_lbl);

            list_box.append(&rev_card);
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
