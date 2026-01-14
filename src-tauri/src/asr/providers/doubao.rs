//! 豆包语音识别 Provider
//!
//! 使用字节跳动豆包流式语音识别 2.0 API

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::asr::client::AsrClient;
use crate::asr::provider::{AsrError, AsrProvider, AsrResult, ProviderStatus};

/// 豆包 ASR 配置
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DoubaoConfig {
    /// 应用 ID
    #[serde(default)]
    pub app_id: String,
    /// 访问令牌
    #[serde(default)]
    pub access_token: String,
    /// 密钥（可选，用于 HMAC 签名）
    #[serde(default)]
    pub secret_key: String,
}

impl DoubaoConfig {
    pub fn is_configured(&self) -> bool {
        !self.app_id.is_empty() && !self.access_token.is_empty()
    }
}

/// 豆包语音识别 Provider
pub struct DoubaoProvider {
    config: DoubaoConfig,
}

impl DoubaoProvider {
    pub fn new(config: DoubaoConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl AsrProvider for DoubaoProvider {
    fn id(&self) -> &str {
        "doubao"
    }

    fn display_name(&self) -> &str {
        "豆包语音识别"
    }

    fn status(&self) -> ProviderStatus {
        if !self.config.is_configured() {
            ProviderStatus::NeedsConfiguration
        } else {
            ProviderStatus::Ready
        }
    }

    fn validate(&self) -> Result<(), AsrError> {
        if self.config.app_id.is_empty() {
            return Err(AsrError::Configuration("App ID 不能为空".into()));
        }
        if self.config.access_token.is_empty() {
            return Err(AsrError::Configuration("Access Token 不能为空".into()));
        }
        Ok(())
    }

    async fn transcribe_stream(
        &self,
        audio_rx: mpsc::Receiver<Vec<u8>>,
        result_tx: mpsc::Sender<AsrResult>,
    ) -> Result<(), AsrError> {
        self.validate()?;

        let client = AsrClient::new(
            self.config.app_id.clone(),
            self.config.access_token.clone(),
            self.config.secret_key.clone(),
        );

        // 创建内部结果通道，转换格式
        let (internal_tx, mut internal_rx) =
            mpsc::channel::<crate::asr::client::AsrResult>(32);

        // 启动转换任务
        let result_tx_clone = result_tx.clone();
        tokio::spawn(async move {
            while let Some(internal_result) = internal_rx.recv().await {
                let result = AsrResult {
                    text: internal_result.text,
                    is_final: !internal_result.is_prefetch,
                };
                if result_tx_clone.send(result).await.is_err() {
                    break;
                }
            }
        });

        // 调用原有的 ASR 客户端
        client
            .connect_and_stream(audio_rx, internal_tx)
            .await
            .map_err(|e| AsrError::Transcription(e.to_string()))?;

        Ok(())
    }
}
