use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum RecordingState {
    Idle,
    Recording,
    Processing,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub app_id: String,
    pub access_token: String,
    #[serde(default)]
    pub secret_key: String,
    pub shortcut: String,
    pub auto_type: bool,
    pub auto_copy: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            app_id: "8897393220".to_string(),
            access_token: "RI16bILxNn9WKFoW-OTKONeRAJo1RTcf".to_string(),
            secret_key: "p43qdmieHVS6fB5BmfwbeI6o1_RVobMn".to_string(),
            shortcut: "Alt+Space".to_string(),
            auto_type: true,
            auto_copy: true,
        }
    }
}

pub struct AppState {
    pub recording_state: Arc<RwLock<RecordingState>>,
    pub current_transcript: Arc<RwLock<String>>,
    pub config: Arc<RwLock<AppConfig>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            recording_state: Arc::new(RwLock::new(RecordingState::Idle)),
            current_transcript: Arc::new(RwLock::new(String::new())),
            config: Arc::new(RwLock::new(AppConfig::default())),
        }
    }

    pub fn set_recording_state(&self, state: RecordingState) {
        *self.recording_state.write() = state;
    }

    pub fn get_recording_state(&self) -> RecordingState {
        self.recording_state.read().clone()
    }

    pub fn set_transcript(&self, text: String) {
        *self.current_transcript.write() = text;
    }

    pub fn get_transcript(&self) -> String {
        self.current_transcript.read().clone()
    }

    pub fn clear_transcript(&self) {
        self.current_transcript.write().clear();
    }

    pub fn get_config(&self) -> AppConfig {
        self.config.read().clone()
    }

    pub fn update_config(&self, config: AppConfig) {
        *self.config.write() = config;
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
