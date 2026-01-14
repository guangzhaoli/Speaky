//! ASR Provider 统一抽象层
//!
//! 定义语音识别服务的通用接口，支持多种后端实现。

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;
use tokio::sync::mpsc;

/// ASR 识别结果（统一格式）
#[derive(Clone, Debug, Serialize)]
pub struct AsrResult {
    /// 识别出的文本
    pub text: String,
    /// 是否是最终结果（false 表示中间结果/prefetch）
    pub is_final: bool,
}

/// ASR Provider 错误类型
#[derive(Error, Debug)]
pub enum AsrError {
    #[error("连接错误: {0}")]
    Connection(String),
    #[error("配置错误: {0}")]
    Configuration(String),
    #[error("识别错误: {0}")]
    Transcription(String),
    #[error("模型未找到: {0}")]
    ModelNotFound(String),
    #[error("模型下载失败: {0}")]
    ModelDownload(String),
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),
}

/// ASR Provider 状态（用于前端显示）
#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(tag = "type", content = "data")]
#[allow(dead_code)]
pub enum ProviderStatus {
    /// 就绪可用
    Ready,
    /// 需要配置
    NeedsConfiguration,
    /// 需要下载模型
    NeedsModelDownload { model: String, size_mb: u64 },
    /// 正在下载
    Downloading { progress: f32 },
    /// 发生错误
    Error(String),
}

/// ASR Provider 基本信息
#[derive(Clone, Debug, Serialize)]
pub struct ProviderInfo {
    pub id: String,
    pub display_name: String,
    pub status: ProviderStatus,
}

/// ASR Provider 统一接口
#[async_trait]
pub trait AsrProvider: Send + Sync {
    /// Provider 唯一标识 (e.g., "doubao", "whisper_local", "whisper_api")
    fn id(&self) -> &str;

    /// Provider 显示名称
    fn display_name(&self) -> &str;

    /// 获取当前状态
    fn status(&self) -> ProviderStatus;

    /// 检查是否已就绪
    fn is_ready(&self) -> bool {
        matches!(self.status(), ProviderStatus::Ready)
    }

    /// 验证配置是否有效
    fn validate(&self) -> Result<(), AsrError>;

    /// 流式语音识别
    /// - audio_rx: 接收 16kHz/16bit/单声道 PCM 音频数据
    /// - result_tx: 发送识别结果
    async fn transcribe_stream(
        &self,
        audio_rx: mpsc::Receiver<Vec<u8>>,
        result_tx: mpsc::Sender<AsrResult>,
    ) -> Result<(), AsrError>;

    /// 获取 Provider 信息
    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            id: self.id().to_string(),
            display_name: self.display_name().to_string(),
            status: self.status(),
        }
    }
}

/// 模型信息
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelInfo {
    /// 模型 ID (e.g., "ggml-base.bin")
    pub id: String,
    /// 模型显示名称 (e.g., "Base (142 MB)")
    pub name: String,
    /// 模型文件大小（字节）
    pub size_bytes: u64,
    /// 是否已下载
    pub is_downloaded: bool,
    /// 是否是当前选中的模型
    pub is_selected: bool,
}

/// 模型下载进度
#[derive(Clone, Debug, Serialize)]
pub struct DownloadProgress {
    /// 模型 ID
    pub model_id: String,
    /// 已下载字节数
    pub downloaded_bytes: u64,
    /// 总字节数
    pub total_bytes: u64,
    /// 下载百分比 (0-100)
    pub percent: f32,
}

/// 支持模型下载的 Provider 扩展 trait
#[async_trait]
pub trait ModelDownloadable: AsrProvider {
    /// 获取可用模型列表
    fn available_models(&self) -> Vec<ModelInfo>;

    /// 获取已下载模型列表
    #[allow(dead_code)]
    fn downloaded_models(&self) -> Vec<ModelInfo> {
        self.available_models()
            .into_iter()
            .filter(|m| m.is_downloaded)
            .collect()
    }

    /// 获取模型存储目录
    #[allow(dead_code)]
    fn models_dir(&self) -> PathBuf;

    /// 下载模型
    async fn download_model(
        &self,
        model_id: &str,
        progress_tx: mpsc::Sender<DownloadProgress>,
    ) -> Result<PathBuf, AsrError>;

    /// 删除模型
    async fn delete_model(&self, model_id: &str) -> Result<(), AsrError>;

    /// 取消正在进行的下载
    fn cancel_download(&self);
}
