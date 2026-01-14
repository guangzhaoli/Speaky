//! Whisper 本地语音识别 Provider
//!
//! 使用 whisper.cpp 进行离线语音识别

use async_trait::async_trait;
use directories::ProjectDirs;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

use crate::asr::provider::{
    AsrError, AsrProvider, AsrResult, DownloadProgress, ModelDownloadable, ModelInfo,
    ProviderStatus,
};

/// Whisper 模型大小
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum WhisperModelSize {
    Tiny,
    #[default]
    Base,
    Small,
    Medium,
    Large,
    LargeV3,
}

impl WhisperModelSize {
    /// 所有可用的模型大小
    pub fn all() -> Vec<Self> {
        vec![
            Self::Tiny,
            Self::Base,
            Self::Small,
            Self::Medium,
            Self::Large,
            Self::LargeV3,
        ]
    }

    /// 模型文件名
    pub fn filename(&self) -> &str {
        match self {
            Self::Tiny => "ggml-tiny.bin",
            Self::Base => "ggml-base.bin",
            Self::Small => "ggml-small.bin",
            Self::Medium => "ggml-medium.bin",
            Self::Large => "ggml-large.bin",
            Self::LargeV3 => "ggml-large-v3.bin",
        }
    }

    /// 模型大小（字节）
    pub fn size_bytes(&self) -> u64 {
        match self {
            Self::Tiny => 75_000_000,
            Self::Base => 142_000_000,
            Self::Small => 466_000_000,
            Self::Medium => 1_500_000_000,
            Self::Large => 2_900_000_000,
            Self::LargeV3 => 3_100_000_000,
        }
    }

    /// 显示名称
    pub fn display_name(&self) -> String {
        match self {
            Self::Tiny => format!("Tiny ({} MB)", self.size_bytes() / 1_000_000),
            Self::Base => format!("Base ({} MB)", self.size_bytes() / 1_000_000),
            Self::Small => format!("Small ({} MB)", self.size_bytes() / 1_000_000),
            Self::Medium => format!("Medium ({} GB)", self.size_bytes() / 1_000_000_000),
            Self::Large => format!("Large ({} GB)", self.size_bytes() / 1_000_000_000),
            Self::LargeV3 => format!("Large V3 ({} GB)", self.size_bytes() / 1_000_000_000),
        }
    }

    /// Hugging Face 下载 URL
    pub fn download_url(&self) -> String {
        format!(
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/{}",
            self.filename()
        )
    }

    /// 从文件名解析模型大小
    pub fn from_filename(filename: &str) -> Option<Self> {
        match filename {
            "ggml-tiny.bin" => Some(Self::Tiny),
            "ggml-base.bin" => Some(Self::Base),
            "ggml-small.bin" => Some(Self::Small),
            "ggml-medium.bin" => Some(Self::Medium),
            "ggml-large.bin" => Some(Self::Large),
            "ggml-large-v3.bin" => Some(Self::LargeV3),
            _ => None,
        }
    }
}

/// Whisper 本地配置
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WhisperLocalConfig {
    /// 模型大小
    #[serde(default)]
    pub model_size: WhisperModelSize,
    /// 自定义模型路径（可选）
    #[serde(default)]
    pub model_path: Option<PathBuf>,
    /// 识别语言 ("auto", "zh", "en", "ja", "ko", etc.)
    #[serde(default = "default_language")]
    pub language: String,
    /// 是否翻译为英语
    #[serde(default)]
    pub translate_to_english: bool,
}

fn default_language() -> String {
    "zh".to_string()
}

impl Default for WhisperLocalConfig {
    fn default() -> Self {
        Self {
            model_size: WhisperModelSize::default(),
            model_path: None,
            language: default_language(),
            translate_to_english: false,
        }
    }
}

/// Whisper 本地 Provider
pub struct WhisperLocalProvider {
    config: RwLock<WhisperLocalConfig>,
    models_dir: PathBuf,
    cancel_flag: Arc<AtomicBool>,
}

impl WhisperLocalProvider {
    pub fn new(config: WhisperLocalConfig) -> Self {
        // 模型存储目录: ~/.config/speaky/models/whisper/
        let models_dir = ProjectDirs::from("com", "speaky", "Speaky")
            .map(|dirs| dirs.config_dir().join("models").join("whisper"))
            .unwrap_or_else(|| PathBuf::from("./models/whisper"));

        Self {
            config: RwLock::new(config),
            models_dir,
            cancel_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    /// 获取模型文件路径
    fn model_path(&self) -> PathBuf {
        let config = self.config.read();
        config
            .model_path
            .clone()
            .unwrap_or_else(|| self.models_dir.join(config.model_size.filename()))
    }

    /// 检查模型是否已下载
    fn is_model_downloaded(&self) -> bool {
        let path = self.model_path();
        path.exists() && std::fs::metadata(&path).map(|m| m.len() > 0).unwrap_or(false)
    }

    /// 检查指定模型是否已下载
    fn is_model_file_downloaded(&self, filename: &str) -> bool {
        let path = self.models_dir.join(filename);
        path.exists() && std::fs::metadata(&path).map(|m| m.len() > 0).unwrap_or(false)
    }
}

#[async_trait]
impl AsrProvider for WhisperLocalProvider {
    fn id(&self) -> &str {
        "whisper_local"
    }

    fn display_name(&self) -> &str {
        "Whisper 本地"
    }

    fn status(&self) -> ProviderStatus {
        if !self.is_model_downloaded() {
            let config = self.config.read();
            ProviderStatus::NeedsModelDownload {
                model: config.model_size.filename().to_string(),
                size_mb: config.model_size.size_bytes() / 1_000_000,
            }
        } else {
            ProviderStatus::Ready
        }
    }

    fn validate(&self) -> Result<(), AsrError> {
        if !self.is_model_downloaded() {
            let config = self.config.read();
            return Err(AsrError::ModelNotFound(format!(
                "需要先下载 {} 模型",
                config.model_size.filename()
            )));
        }
        Ok(())
    }

    async fn transcribe_stream(
        &self,
        mut audio_rx: mpsc::Receiver<Vec<u8>>,
        result_tx: mpsc::Sender<AsrResult>,
    ) -> Result<(), AsrError> {
        self.validate()?;

        let model_path = self.model_path();
        let language = self.config.read().language.clone();
        let translate = self.config.read().translate_to_english;

        // Whisper 不支持真正的流式识别，需要累积音频后批量处理
        let mut audio_buffer: Vec<i16> = Vec::new();

        while let Some(chunk) = audio_rx.recv().await {
            // PCM bytes -> i16 samples
            let samples: Vec<i16> = chunk
                .chunks_exact(2)
                .map(|c| i16::from_le_bytes([c[0], c[1]]))
                .collect();
            audio_buffer.extend(samples);
        }

        if audio_buffer.is_empty() {
            return Ok(());
        }

        // 转换为 f32 (whisper-rs 要求)
        let audio_f32: Vec<f32> = audio_buffer
            .iter()
            .map(|&s| s as f32 / 32768.0)
            .collect();

        // 在阻塞线程中运行 Whisper
        let result = tokio::task::spawn_blocking(move || {
            // 加载模型
            let params = WhisperContextParameters::default();
            let ctx = WhisperContext::new_with_params(model_path.to_str().unwrap(), params)
                .map_err(|e| AsrError::Transcription(format!("模型加载失败: {}", e)))?;

            let mut state = ctx
                .create_state()
                .map_err(|e| AsrError::Transcription(format!("创建状态失败: {}", e)))?;

            // 配置识别参数
            let mut full_params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });

            // 设置语言
            if language != "auto" {
                full_params.set_language(Some(&language));
            }
            full_params.set_translate(translate);
            full_params.set_print_special(false);
            full_params.set_print_progress(false);
            full_params.set_print_realtime(false);
            full_params.set_print_timestamps(false);

            // 执行识别
            state
                .full(full_params, &audio_f32)
                .map_err(|e| AsrError::Transcription(format!("识别失败: {}", e)))?;

            // 收集所有片段
            let num_segments = state.full_n_segments();

            let mut full_text = String::new();
            for i in 0..num_segments {
                if let Some(segment) = state.get_segment(i) {
                    if let Ok(text) = segment.to_str_lossy() {
                        full_text.push_str(&text);
                    }
                }
            }

            Ok::<String, AsrError>(full_text.trim().to_string())
        })
        .await
        .map_err(|e| AsrError::Transcription(format!("任务执行失败: {}", e)))??;

        // 发送最终结果
        let _ = result_tx
            .send(AsrResult {
                text: result,
                is_final: true,
            })
            .await;

        Ok(())
    }
}

#[async_trait]
impl ModelDownloadable for WhisperLocalProvider {
    fn available_models(&self) -> Vec<ModelInfo> {
        let current_model = self.config.read().model_size.clone();

        WhisperModelSize::all()
            .into_iter()
            .map(|size| {
                let filename = size.filename();
                ModelInfo {
                    id: filename.to_string(),
                    name: size.display_name(),
                    size_bytes: size.size_bytes(),
                    is_downloaded: self.is_model_file_downloaded(filename),
                    is_selected: size == current_model,
                }
            })
            .collect()
    }

    fn models_dir(&self) -> PathBuf {
        self.models_dir.clone()
    }

    async fn download_model(
        &self,
        model_id: &str,
        progress_tx: mpsc::Sender<DownloadProgress>,
    ) -> Result<PathBuf, AsrError> {
        let size = WhisperModelSize::from_filename(model_id)
            .ok_or_else(|| AsrError::ModelNotFound(format!("未知模型: {}", model_id)))?;

        let url = size.download_url();
        let dest_path = self.models_dir.join(model_id);
        let temp_path = dest_path.with_extension("tmp");

        // 创建目录
        std::fs::create_dir_all(&self.models_dir)?;

        // 重置取消标志
        self.cancel_flag.store(false, Ordering::SeqCst);
        let cancel_flag = self.cancel_flag.clone();

        // 使用模型管理器下载
        crate::asr::model_manager::download_file(
            &url,
            &temp_path,
            &dest_path,
            model_id,
            progress_tx,
            cancel_flag,
        )
        .await?;

        Ok(dest_path)
    }

    async fn delete_model(&self, model_id: &str) -> Result<(), AsrError> {
        let path = self.models_dir.join(model_id);
        if path.exists() {
            std::fs::remove_file(&path)?;
            log::info!("已删除模型: {:?}", path);
        }
        Ok(())
    }

    fn cancel_download(&self) {
        self.cancel_flag.store(true, Ordering::SeqCst);
    }
}
