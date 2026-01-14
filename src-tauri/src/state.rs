use directories::ProjectDirs;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use crate::asr::providers::{DoubaoConfig, WhisperApiConfig, WhisperLocalConfig};
use crate::postprocess::PostProcessConfig;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum RecordingState {
    Idle,
    Recording,
    Processing,
}

/// ASR 配置
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AsrConfig {
    /// 当前激活的 Provider ID ("doubao", "whisper_local", "whisper_api")
    #[serde(default = "default_active_provider")]
    pub active_provider: String,
    /// 豆包配置
    #[serde(default)]
    pub doubao: Option<DoubaoConfig>,
    /// Whisper 本地配置
    #[serde(default)]
    pub whisper_local: Option<WhisperLocalConfig>,
    /// Whisper API 配置
    #[serde(default)]
    pub whisper_api: Option<WhisperApiConfig>,
}

fn default_active_provider() -> String {
    "doubao".to_string()
}

impl Default for AsrConfig {
    fn default() -> Self {
        Self {
            active_provider: default_active_provider(),
            doubao: Some(DoubaoConfig::default()),
            whisper_local: None,
            whisper_api: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppConfig {
    /// ASR 配置（新）
    #[serde(default)]
    pub asr: AsrConfig,
    /// 旧字段，用于向后兼容迁移
    #[serde(default, skip_serializing)]
    pub app_id: String,
    #[serde(default, skip_serializing)]
    pub access_token: String,
    #[serde(default, skip_serializing)]
    pub secret_key: String,
    pub shortcut: String,
    pub auto_type: bool,
    pub auto_copy: bool,
    #[serde(default)]
    pub auto_start: bool,
    #[serde(default)]
    pub silent_start: bool,
    #[serde(default = "default_show_indicator")]
    pub show_indicator: bool,
    #[serde(default)]
    pub realtime_input: bool,
    #[serde(default)]
    pub postprocess: PostProcessConfig,
    /// 选择的音频设备名称，空字符串表示使用系统默认设备
    #[serde(default)]
    pub audio_device: String,
    /// 是否启用日志记录到文件
    #[serde(default = "default_enable_logging")]
    pub enable_logging: bool,
    /// ASR 识别语言 ("auto", "zh", "en", "ja", "ko", etc.)
    #[serde(default = "default_asr_language")]
    pub asr_language: String,
}

fn default_asr_language() -> String {
    "zh".to_string()
}

fn default_show_indicator() -> bool {
    true
}

fn default_enable_logging() -> bool {
    true
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            asr: AsrConfig::default(),
            app_id: String::new(),
            access_token: String::new(),
            secret_key: String::new(),
            shortcut: "Alt+Space".to_string(),
            auto_type: true,
            auto_copy: true,
            auto_start: false,
            silent_start: false,
            show_indicator: true,
            realtime_input: false,
            postprocess: PostProcessConfig::default(),
            audio_device: String::new(),
            enable_logging: true,
            asr_language: default_asr_language(),
        }
    }
}

impl AppConfig {
    /// 获取配置文件路径
    fn config_path() -> Option<PathBuf> {
        ProjectDirs::from("com", "speaky", "Speaky")
            .map(|dirs| dirs.config_dir().join("config.toml"))
    }

    /// 从文件加载配置
    pub fn load() -> Self {
        if let Some(path) = Self::config_path() {
            if path.exists() {
                match fs::read_to_string(&path) {
                    Ok(content) => match toml::from_str::<AppConfig>(&content) {
                        Ok(mut config) => {
                            log::info!("Config loaded from {:?}", path);
                            // 迁移旧配置到新的 ASR 配置
                            config.migrate_legacy_asr_config();
                            return config;
                        }
                        Err(e) => {
                            log::error!("Failed to parse config: {}", e);
                        }
                    },
                    Err(e) => {
                        log::error!("Failed to read config file: {}", e);
                    }
                }
            }
        }
        Self::default()
    }

    /// 迁移旧的 ASR 配置到新结构
    fn migrate_legacy_asr_config(&mut self) {
        // 如果旧字段有值，迁移到新的 asr.doubao 配置
        if !self.app_id.is_empty() || !self.access_token.is_empty() {
            let doubao_config = DoubaoConfig {
                app_id: std::mem::take(&mut self.app_id),
                access_token: std::mem::take(&mut self.access_token),
                secret_key: std::mem::take(&mut self.secret_key),
            };

            // 只有当 doubao 配置为空或未配置时才迁移
            if self.asr.doubao.as_ref().map(|c| !c.is_configured()).unwrap_or(true) {
                self.asr.doubao = Some(doubao_config);
                log::info!("Migrated legacy ASR config to new format");
            }
        }
    }

    /// 保存配置到文件
    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path().ok_or("Failed to get config path")?;

        // 创建配置目录
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config dir: {}", e))?;
        }

        let content = toml::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;

        fs::write(&path, content).map_err(|e| format!("Failed to write config: {}", e))?;

        log::info!("Config saved to {:?}", path);
        Ok(())
    }
}

pub struct AppState {
    pub recording_state: Arc<RwLock<RecordingState>>,
    pub current_transcript: Arc<RwLock<String>>,
    pub config: Arc<RwLock<AppConfig>>,
}

impl AppState {
    pub fn new() -> Self {
        // 启动时加载配置
        let config = AppConfig::load();
        Self {
            recording_state: Arc::new(RwLock::new(RecordingState::Idle)),
            current_transcript: Arc::new(RwLock::new(String::new())),
            config: Arc::new(RwLock::new(config)),
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

    pub fn update_config(&self, config: AppConfig) -> Result<(), String> {
        // 保存到文件
        config.save()?;
        // 更新内存中的配置
        *self.config.write() = config;
        Ok(())
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
