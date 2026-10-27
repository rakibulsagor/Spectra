use std::sync::{Arc, RwLock};
use tracing::info;

use crate::models::GhostSettings;
use crate::storage::Database;

#[derive(Clone)]
pub struct GhostManager {
    settings: Arc<RwLock<GhostSettings>>,
    storage: Database,
}

impl GhostManager {
    pub fn new(storage: Database) -> Self {
        let initial = storage.load_settings();
        Self {
            settings: Arc::new(RwLock::new(initial)),
            storage,
        }
    }

    pub fn get_settings(&self) -> GhostSettings {
        self.settings.read().unwrap().clone()
    }

    pub fn update_settings(&self, new_settings: GhostSettings) {
        let mut w = self.settings.write().unwrap();
        *w = new_settings.clone();
        let _ = self.storage.save_settings(&new_settings);
        info!(
            ghost_enabled = new_settings.ghost_mode_enabled,
            "Spectra ghost settings updated"
        );
    }

    pub fn is_ghost_mode_enabled(&self) -> bool {
        self.settings.read().unwrap().ghost_mode_enabled
    }

    pub fn set_ghost_mode_enabled(&self, enabled: bool) {
        let mut w = self.settings.write().unwrap();
        w.ghost_mode_enabled = enabled;
        let _ = self.storage.save_settings(&w);
        info!(active = enabled, "Spectra Ghost Mode state set");
    }

    pub fn toggle_ghost_mode(&self) -> bool {
        let mut w = self.settings.write().unwrap();
        w.ghost_mode_enabled = !w.ghost_mode_enabled;
        let new_state = w.ghost_mode_enabled;
        let _ = self.storage.save_settings(&w);
        info!(active = new_state, "Spectra Ghost Mode toggled");
        new_state
    }

    pub fn should_send_read(&self) -> bool {
        let s = self.settings.read().unwrap();
        if !s.ghost_mode_enabled {
            return true;
        }
        s.send_read_messages
    }

    pub fn should_send_typing(&self) -> bool {
        let s = self.settings.read().unwrap();
        if !s.ghost_mode_enabled {
            return true;
        }
        s.send_typing_indicators
    }

    pub fn should_send_online(&self) -> bool {
        let s = self.settings.read().unwrap();
        if !s.ghost_mode_enabled {
            return true;
        }
        s.send_online_packets
    }

    pub fn should_send_stories(&self) -> bool {
        let s = self.settings.read().unwrap();
        if !s.ghost_mode_enabled {
            return true;
        }
        s.send_read_stories
    }

    pub fn is_anti_recall_enabled(&self) -> bool {
        self.settings.read().unwrap().anti_recall_enabled
    }

    pub fn is_anti_edit_enabled(&self) -> bool {
        self.settings.read().unwrap().anti_edit_enabled
    }

    pub fn bypass_noforwards(&self) -> bool {
        self.settings.read().unwrap().bypass_noforwards
    }

    pub fn is_streamer_mode(&self) -> bool {
        self.settings.read().unwrap().streamer_mode
    }
}

/// Redacts sensitive information (phone numbers, balances, tokens) for logs and streamer mode
pub fn redact_sensitive(input: &str) -> String {
    if input.len() <= 4 {
        return "***".to_string();
    }
    format!("{}***{}", &input[..2], &input[input.len() - 2..])
}
