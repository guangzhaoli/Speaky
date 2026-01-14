use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use std::time::Duration;

use super::config::LlmProvider;

/// 全局 HTTP 客户端（连接复用）
static HTTP_CLIENT: OnceLock<Client> = OnceLock::new();

fn get_http_client() -> &'static Client {
    HTTP_CLIENT.get_or_init(|| {
        Client::builder()
            .pool_max_idle_per_host(2)
            .pool_idle_timeout(Duration::from_secs(60))
            .tcp_keepalive(Duration::from_secs(30))
            .timeout(Duration::from_secs(15))
            .connect_timeout(Duration::from_secs(5))
            .build()
            .expect("Failed to create HTTP client")
    })
}

/// OpenAI 兼容的 Chat 请求结构
#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    temperature: f32,
    max_tokens: u32,
}

/// 消息结构
#[derive(Debug, Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

/// Chat 响应结构
#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
}

/// LLM 客户端
pub struct LlmClient {
    api_base: String,
    api_key: String,
    model: String,
}

impl LlmClient {
    /// 从 Provider 配置创建客户端
    pub fn new(provider: &LlmProvider) -> Self {
        Self {
            api_base: provider.api_base.clone(),
            api_key: provider.api_key.clone(),
            model: provider.model.clone(),
        }
    }

    /// 调用 LLM 处理文本
    pub async fn process(&self, text: &str, system_prompt: &str) -> Result<String, String> {
        let request = ChatRequest {
            model: self.model.clone(),
            messages: vec![
                Message {
                    role: "system".to_string(),
                    content: system_prompt.to_string(),
                },
                Message {
                    role: "user".to_string(),
                    content: text.to_string(),
                },
            ],
            temperature: 0.3,
            max_tokens: 1024,
        };

        let url = format!("{}/chat/completions", self.api_base.trim_end_matches('/'));
        let client = get_http_client();

        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!("API error {}: {}", status, body));
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .map_err(|e| format!("Parse response failed: {}", e))?;

        chat_response
            .choices
            .first()
            .map(|c| c.message.content.trim().to_string())
            .ok_or_else(|| "Empty response".to_string())
    }
}

/// 预热 HTTP 连接（可选，应用启动时调用）
pub async fn warmup_connection(api_base: &str) {
    let client = get_http_client();
    let url = format!("{}/models", api_base.trim_end_matches('/'));

    // 发送一个轻量请求预热连接
    let _ = client
        .get(&url)
        .timeout(Duration::from_secs(3))
        .send()
        .await;

    log::debug!("HTTP connection warmed up for {}", api_base);
}
