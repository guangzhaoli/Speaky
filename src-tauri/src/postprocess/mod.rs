pub mod client;
pub mod config;
pub mod prompts;

use std::time::Duration;
use tokio::time::timeout;

pub use config::{LlmProvider, PostProcessConfig};

use client::LlmClient;
use prompts::get_prompt;

/// 根据文本长度计算动态超时时间
fn calculate_timeout(text_len: usize) -> Duration {
    // 基础 3 秒 + 每 100 字符增加 0.5 秒，最长 10 秒
    let base = 3.0;
    let per_char = 0.005; // 每个字符 5ms
    let extra = (text_len as f64 * per_char).min(7.0);
    Duration::from_secs_f64(base + extra)
}

/// 对文本进行后处理
///
/// 如果后处理失败或超时，返回原文本
pub async fn process_text(text: &str, config: &PostProcessConfig) -> Result<String, String> {
    // 空文本直接返回
    if text.trim().is_empty() {
        return Ok(text.to_string());
    }

    // 禁用后处理时直接返回原文
    if !config.enabled {
        return Ok(text.to_string());
    }

    // 获取激活的 Provider
    let provider = match config.get_active_provider() {
        Some(p) => p,
        None => {
            log::warn!("No active LLM provider configured");
            return Ok(text.to_string());
        }
    };

    // API Key 为空时跳过
    if provider.api_key.is_empty() {
        log::warn!("LLM provider API key is empty");
        return Ok(text.to_string());
    }

    let client = LlmClient::new(provider);
    let prompt = get_prompt(&config.mode);
    let timeout_duration = calculate_timeout(text.len());

    log::debug!(
        "Starting LLM postprocess: {} chars, timeout: {:?}",
        text.len(),
        timeout_duration
    );

    // 使用非流式 API（已经复用连接池，延迟已优化）
    match timeout(timeout_duration, client.process(text, prompt)).await {
        Ok(Ok(result)) => {
            log::info!(
                "LLM postprocess completed in ~{:?}: {} -> {}",
                timeout_duration,
                text,
                result
            );
            Ok(result)
        }
        Ok(Err(e)) => {
            log::error!("LLM postprocess failed: {}", e);
            // 失败时返回原文，不阻断流程
            Ok(text.to_string())
        }
        Err(_) => {
            log::warn!(
                "LLM postprocess timeout after {:?}, using original text",
                timeout_duration
            );
            Ok(text.to_string())
        }
    }
}

/// 测试 LLM 连接
pub async fn test_connection(provider: &LlmProvider) -> Result<String, String> {
    let client = LlmClient::new(provider);

    match timeout(
        Duration::from_secs(10),
        client.process("测试连接", "回复 'OK' 两个字母"),
    )
    .await
    {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(e)) => Err(e),
        Err(_) => Err("Connection timeout".to_string()),
    }
}

/// 预热连接（应用启动时调用）
pub async fn warmup(config: &PostProcessConfig) {
    if !config.enabled {
        return;
    }

    if let Some(provider) = config.get_active_provider() {
        if !provider.api_key.is_empty() {
            client::warmup_connection(&provider.api_base).await;
        }
    }
}
