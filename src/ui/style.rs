use gtk4::gdk::Display;
use gtk4::CssProvider;

pub const SPECTRA_CSS: &str = "
.msg-bubble {
    border-radius: 12px;
    padding: 10px 14px;
    margin: 4px 12px;
}

.msg-incoming {
    background-color: #1e2a38;
    color: #ffffff;
    border: 1px solid #283747;
}

.msg-outgoing {
    background-color: #2b5278;
    color: #ffffff;
    border: 1px solid #376595;
}

.msg-deleted {
    background-color: #3b1820;
    border: 1.5px solid #e03131;
    color: #ffc9c9;
}

.msg-deleted-text {
    text-decoration: line-through;
    color: #ffa8a8;
}

.badge-deleted {
    background-color: #c92a2a;
    color: #ffffff;
    font-size: 10px;
    font-weight: bold;
    border-radius: 4px;
    padding: 2px 6px;
    margin-bottom: 4px;
}

.badge-edited {
    background-color: #e67700;
    color: #ffffff;
    font-size: 10px;
    font-weight: bold;
    border-radius: 4px;
    padding: 2px 6px;
}

.ghost-active {
    background-color: #7950f2;
    color: #ffffff;
    font-weight: bold;
}

.ghost-badge {
    background-color: rgba(121, 80, 242, 0.2);
    color: #b197fc;
    font-size: 11px;
    border-radius: 6px;
    padding: 2px 8px;
}

.avatar-circle {
    border-radius: 20px;
    background: linear-gradient(135deg, #3b5bdb 0%, #7048e8 100%);
    color: #ffffff;
    font-weight: bold;
    font-size: 14px;
    min-width: 40px;
    min-height: 40px;
    padding: 8px;
}

.chat-list-row {
    padding: 8px 10px;
    border-radius: 8px;
    margin: 2px 4px;
}

.chat-deleted-badge {
    background-color: #e03131;
    color: #ffffff;
    font-size: 10px;
    font-weight: bold;
    border-radius: 10px;
    padding: 2px 6px;
}

.stealth-status-banner {
    background-color: #2b1d3a;
    color: #d0bfff;
    padding: 6px 12px;
    font-size: 12px;
    font-weight: 500;
    border-bottom: 1px solid #492e6d;
}
";

pub fn load_css() {
    if let Some(display) = Display::default() {
        let provider = CssProvider::new();
        provider.load_from_data(SPECTRA_CSS);
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}
