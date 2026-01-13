use serde::{Deserialize, Serialize};

/// 豆包 ASR 请求配置
#[derive(Serialize, Debug, Clone)]
pub struct AsrConfig {
    pub user: UserConfig,
    pub audio: AudioConfig,
    pub request: RequestConfig,
}

#[derive(Serialize, Debug, Clone)]
pub struct UserConfig {
    pub uid: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct AudioConfig {
    pub format: String,
    pub codec: String,
    pub rate: u32,
    pub bits: u32,
    pub channel: u32,
}

#[derive(Serialize, Debug, Clone)]
pub struct RequestConfig {
    pub model_name: String,
    pub enable_punc: bool,
    pub enable_itn: bool,
    pub result_type: String,
    pub show_utterances: bool,
}

impl Default for AsrConfig {
    fn default() -> Self {
        Self {
            user: UserConfig {
                uid: uuid::Uuid::new_v4().to_string(),
            },
            audio: AudioConfig {
                format: "pcm".to_string(),
                codec: "pcm".to_string(),
                rate: 16000,
                bits: 16,
                channel: 1,
            },
            request: RequestConfig {
                model_name: "bigmodel".to_string(),
                enable_punc: true,
                enable_itn: true,
                result_type: "single".to_string(),
                show_utterances: false,
            },
        }
    }
}

/// 豆包 ASR 响应
#[derive(Deserialize, Debug, Clone)]
pub struct AsrResponse {
    #[serde(default)]
    pub reqid: Option<String>,
    #[serde(default)]
    pub code: i32,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub sequence: Option<i32>,
    // 旧格式：result 是数组
    // 新格式：result 是对象
    #[serde(default)]
    pub result: Option<AsrResultWrapper>,
    #[serde(default)]
    pub audio_info: Option<AudioInfo>,
}

#[derive(Deserialize, Debug, Clone, Default)]
#[serde(untagged)]
pub enum AsrResultWrapper {
    #[default]
    None,
    Single(AsrResultSingle),
    Array(Vec<AsrResult>),
}

#[derive(Deserialize, Debug, Clone)]
pub struct AsrResultSingle {
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub prefetch: bool,
    #[serde(default)]
    pub additions: Option<serde_json::Value>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct AudioInfo {
    #[serde(default)]
    pub duration: u64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct AsrResult {
    pub text: String,
    #[serde(default)]
    pub utterances: Vec<Utterance>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Utterance {
    pub text: String,
    pub start_time: u64,
    pub end_time: u64,
    #[serde(default)]
    pub definite: bool,
}

impl AsrResponse {
    pub fn is_success(&self) -> bool {
        // 新格式没有 code 字段，只要有 result 就是成功
        self.result.is_some() || self.code == 1000
    }

    pub fn get_text(&self) -> String {
        match &self.result {
            Some(AsrResultWrapper::Single(r)) => r.text.clone(),
            Some(AsrResultWrapper::Array(results)) => {
                results.first().map(|r| r.text.clone()).unwrap_or_default()
            }
            _ => String::new(),
        }
    }

    /// 检查是否是 prefetch 结果（预取结果通常是最终结果）
    pub fn is_prefetch(&self) -> bool {
        match &self.result {
            Some(AsrResultWrapper::Single(r)) => r.prefetch,
            _ => false,
        }
    }
}
