use serde::{Deserialize, Serialize};

/// 单个 LLM Provider 配置
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LlmProvider {
    /// 唯一标识 (uuid)
    pub id: String,
    /// 显示名称 ("DeepSeek", "GPT-4o")
    pub name: String,
    /// API 基础 URL ("https://api.deepseek.com/v1")
    pub api_base: String,
    /// API Key
    pub api_key: String,
    /// 模型名称 ("deepseek-chat")
    pub model: String,
}

/// 处理模式
#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq)]
pub enum PostProcessMode {
    #[default]
    General,  // 日常输入
    Code,     // 代码注释
    Meeting,  // 会议记录
}

/// 后处理总配置
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PostProcessConfig {
    /// 是否启用后处理
    pub enabled: bool,
    /// Provider 列表
    pub providers: Vec<LlmProvider>,
    /// 当前激活的 Provider ID
    pub active_provider_id: String,
    /// 处理模式
    pub mode: PostProcessMode,
}

impl Default for PostProcessConfig {
    fn default() -> Self {
        let default_provider = LlmProvider {
            id: "default".to_string(),
            name: "DeepSeek".to_string(),
            api_base: "https://api.deepseek.com/v1".to_string(),
            api_key: String::new(),
            model: "deepseek-chat".to_string(),
        };
        Self {
            enabled: false,
            providers: vec![default_provider],
            active_provider_id: "default".to_string(),
            mode: PostProcessMode::General,
        }
    }
}

impl PostProcessConfig {
    /// 获取当前激活的 Provider
    pub fn get_active_provider(&self) -> Option<&LlmProvider> {
        self.providers
            .iter()
            .find(|p| p.id == self.active_provider_id)
    }
}
