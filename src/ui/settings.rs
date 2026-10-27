use adw::prelude::*;
use adw::{ActionRow, PreferencesGroup, PreferencesPage, PreferencesWindow};
use gtk4::{Switch, Window};

use crate::privacy::GhostManager;

fn create_switch_row(title: &str, subtitle: &str, active: bool) -> (ActionRow, Switch) {
    let row = ActionRow::builder().title(title).subtitle(subtitle).build();
    let switch = Switch::builder()
        .active(active)
        .valign(gtk4::Align::Center)
        .build();
    row.add_suffix(&switch);
    row.set_activatable_widget(Some(&switch));
    (row, switch)
}

pub fn show_settings_window(parent: &Window, ghost: GhostManager) {
    let window = PreferencesWindow::builder()
        .transient_for(parent)
        .modal(true)
        .title("⚙️ Spectra Privacy & Ghost Settings")
        .default_width(500)
        .default_height(450)
        .build();

    let page = PreferencesPage::new();
    let current = ghost.get_settings();

    // 1. Ghost Mode Group
    let ghost_group = PreferencesGroup::builder()
        .title("👻 Ghost Mode (Stealth Controls)")
        .description("Control outgoing MTProto requests and avoid sending activity receipts.")
        .build();

    let (master_row, master_sw) = create_switch_row(
        "Master Ghost Mode",
        "Enables stealth policies across all conversations",
        current.ghost_mode_enabled,
    );
    let (read_row, read_sw) = create_switch_row(
        "Send Read Receipts",
        "When OFF, viewing messages never marks them read on Telegram servers",
        current.send_read_messages,
    );
    let (typing_row, typing_sw) = create_switch_row(
        "Send Typing Indicators",
        "When OFF, typing never notifies the recipient",
        current.send_typing_indicators,
    );
    let (online_row, online_sw) = create_switch_row(
        "Send Online Presence",
        "When OFF, app never sends online packets on startup or focus",
        current.send_online_packets,
    );
    let (story_row, story_sw) = create_switch_row(
        "Send Story View Receipts",
        "When OFF, stories can be opened anonymously",
        current.send_read_stories,
    );

    ghost_group.add(&master_row);
    ghost_group.add(&read_row);
    ghost_group.add(&typing_row);
    ghost_group.add(&online_row);
    ghost_group.add(&story_row);
    page.add(&ghost_group);

    // 2. Data Retention Group
    let retention_group = PreferencesGroup::builder()
        .title("🛡️ Anti-Recall & Data Retention")
        .description("Local storage overrides matching AyuGram Desktop functionality.")
        .build();

    let (recall_row, recall_sw) = create_switch_row(
        "Anti-Recall (Save Deleted Messages)",
        "Retain messages locally when the sender deletes them for everyone",
        current.anti_recall_enabled,
    );
    let (edit_row, edit_sw) = create_switch_row(
        "Anti-Edit (Save Revisions History)",
        "Preserve previous message drafts when sender modifies content",
        current.anti_edit_enabled,
    );
    let (noforwards_row, noforwards_sw) = create_switch_row(
        "Bypass Restricted Channels (noforwards)",
        "Permit copying, forwarding, and saving from restricted channels",
        current.bypass_noforwards,
    );
    let (streamer_row, streamer_sw) = create_switch_row(
        "Streamer Mode",
        "Mask sensitive phone numbers, user handles, and sensitive balances",
        current.streamer_mode,
    );

    retention_group.add(&recall_row);
    retention_group.add(&edit_row);
    retention_group.add(&noforwards_row);
    retention_group.add(&streamer_row);
    page.add(&retention_group);

    // Connect signals
    let g = ghost.clone();
    let m_sw = master_sw.clone();
    let r_sw = read_sw.clone();
    let t_sw = typing_sw.clone();
    let o_sw = online_sw.clone();
    let s_sw = story_sw.clone();
    let rc_sw = recall_sw.clone();
    let e_sw = edit_sw.clone();
    let nf_sw = noforwards_sw.clone();
    let st_sw = streamer_sw.clone();

    let save_settings = move || {
        let mut s = g.get_settings();
        s.ghost_mode_enabled = m_sw.is_active();
        s.send_read_messages = r_sw.is_active();
        s.send_typing_indicators = t_sw.is_active();
        s.send_online_packets = o_sw.is_active();
        s.send_read_stories = s_sw.is_active();
        s.anti_recall_enabled = rc_sw.is_active();
        s.anti_edit_enabled = e_sw.is_active();
        s.bypass_noforwards = nf_sw.is_active();
        s.streamer_mode = st_sw.is_active();
        g.update_settings(s);
    };

    let cb = save_settings.clone();
    master_sw.connect_active_notify(move |_| cb());

    let cb = save_settings.clone();
    read_sw.connect_active_notify(move |_| cb());

    let cb = save_settings.clone();
    typing_sw.connect_active_notify(move |_| cb());

    let cb = save_settings.clone();
    online_sw.connect_active_notify(move |_| cb());

    let cb = save_settings.clone();
    story_sw.connect_active_notify(move |_| cb());

    let cb = save_settings.clone();
    recall_sw.connect_active_notify(move |_| cb());

    let cb = save_settings.clone();
    edit_sw.connect_active_notify(move |_| cb());

    let cb = save_settings.clone();
    noforwards_sw.connect_active_notify(move |_| cb());

    let cb = save_settings;
    streamer_sw.connect_active_notify(move |_| cb());

    window.add(&page);
    window.present();
}
