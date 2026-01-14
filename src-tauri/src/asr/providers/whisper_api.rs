//! Whisper API Provider
//!
//! 使用 OpenAI Whisper API 或兼容接口进行语音识别

use async_trait::async_trait;
use reqwest::multipart;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::asr::provider::{AsrError, AsrProvider, AsrResult, ProviderStatus};

/// Whisper API 配置
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WhisperApiConfig {
    /// API Key
    #[serde(default)]
    pub api_key: String,
    /// API Base URL
    #[serde(default = "default_api_base")]
    pub api_base: String,
    /// 模型名称
    #[serde(default = "default_model")]
    pub model: String,
    /// 识别语言（可选）
    #[serde(default)]
    pub language: Option<String>,
}

fn default_api_base() -> String {
    "https://api.openai.com/v1".to_string()
}

fn default_model() -> String {
    "whisper-1".to_string()
}

impl Default for WhisperApiConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            api_base: default_api_base(),
            model: default_model(),
            language: None,
        }
    }
}

impl WhisperApiConfig {
    pub fn is_configured(&self) -> bool {
        !self.api_key.is_empty()
    }
}

/// Whisper API Provider
pub struct WhisperApiProvider {
    config: WhisperApiConfig,
    client: reqwest::Client,
}

impl WhisperApiProvider {
    pub fn new(config: WhisperApiConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl AsrProvider for WhisperApiProvider {
    fn id(&self) -> &str {
        "whisper_api"
    }

    fn display_name(&self) -> &str {
        "Whisper API"
    }

    fn status(&self) -> ProviderStatus {
        if !self.config.is_configured() {
            ProviderStatus::NeedsConfiguration
        } else {
            ProviderStatus::Ready
        }
    }

    fn validate(&self) -> Result<(), AsrError> {
        if self.config.api_key.is_empty() {
            return Err(AsrError::Configuration("API Key 不能为空".into()));
        }
        Ok(())
    }

    async fn transcribe_stream(
        &self,
        mut audio_rx: mpsc::Receiver<Vec<u8>>,
        result_tx: mpsc::Sender<AsrResult>,
    ) -> Result<(), AsrError> {
        self.validate()?;

        // 累积所有音频数据
        let mut audio_buffer = Vec::new();
        while let Some(chunk) = audio_rx.recv().await {
            audio_buffer.extend(chunk);
        }

        if audio_buffer.is_empty() {
            return Ok(());
        }

        // 转换为 WAV 格式（OpenAI API 需要）
        let wav_data = pcm_to_wav(&audio_buffer, 16000, 1, 16);

        // 构建 multipart 请求
        let file_part = multipart::Part::bytes(wav_data)
            .file_name("audio.wav")
            .mime_str("audio/wav")
            .map_err(|e| AsrError::Transcription(e.to_string()))?;

        let mut form = multipart::Form::new()
            .part("file", file_part)
            .text("model", self.config.model.clone());

        // 添加语言参数（如果指定）
        if let Some(ref lang) = self.config.language {
            form = form.text("language", lang.clone());
        }

        let url = format!("{}/audio/transcriptions", self.config.api_base);

        let response = self
            .client
            .post(&url)
            .bearer_auth(&self.config.api_key)
            .multipart(form)
            .send()
            .await
            .map_err(|e| AsrError::Connection(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(AsrError::Transcription(format!(
                "API 请求失败 ({}): {}",
                status, error_text
            )));
        }

        #[derive(Deserialize)]
        struct TranscriptionResponse {
            text: String,
        }

        let result: TranscriptionResponse = response
            .json()
            .await
            .map_err(|e| AsrError::Transcription(format!("解析响应失败: {}", e)))?;

        let _ = result_tx
            .send(AsrResult {
                text: result.text,
                is_final: true,
            })
            .await;

        Ok(())
    }
}

/// PCM 转 WAV 格式
fn pcm_to_wav(pcm_data: &[u8], sample_rate: u32, channels: u16, bits_per_sample: u16) -> Vec<u8> {
    let data_size = pcm_data.len() as u32;
    let byte_rate = sample_rate * channels as u32 * bits_per_sample as u32 / 8;
    let block_align = channels * bits_per_sample / 8;

    let mut wav = Vec::with_capacity(44 + pcm_data.len());

    // RIFF header
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_size).to_le_bytes());
    wav.extend_from_slice(b"WAVE");

    // fmt chunk
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
    wav.extend_from_slice(&channels.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&byte_rate.to_le_bytes());
    wav.extend_from_slice(&block_align.to_le_bytes());
    wav.extend_from_slice(&bits_per_sample.to_le_bytes());

    // data chunk
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_size.to_le_bytes());
    wav.extend_from_slice(pcm_data);

    wav
}
