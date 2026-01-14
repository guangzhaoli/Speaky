use crate::asr::protocol::{AsrConfig, AsrResponse};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use futures_util::{SinkExt, StreamExt};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::borrow::Cow;
use std::io::{Read, Write};
use tokio::sync::mpsc;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{
        http::{Request, Uri},
        Message,
    },
};

// 豆包流式语音识别模型 2.0 API 端点
const VOLCENGINE_ASR_URL: &str = "wss://openspeech.bytedance.com/api/v3/sauc/bigmodel";

// 豆包流式语音识别模型 2.0 资源 ID
const RESOURCE_ID: &str = "volc.bigasr.sauc.duration";

type HmacSha256 = Hmac<Sha256>;

// Seed 协议常量
const PROTOCOL_VERSION: u8 = 0x01;
const HEADER_SIZE: u8 = 0x01;
const MESSAGE_TYPE_FULL_CLIENT: u8 = 0x01;
const MESSAGE_TYPE_AUDIO_ONLY: u8 = 0x02;
const MESSAGE_SERIAL_JSON: u8 = 0x01;
const MESSAGE_COMPRESS_GZIP: u8 = 0x01;
const MESSAGE_COMPRESS_NONE: u8 = 0x00;

/// ASR 结果，包含文本和是否是 prefetch
#[derive(Clone, Debug)]
pub struct AsrResult {
    pub text: String,
    pub is_prefetch: bool,
}

pub struct AsrClient {
    app_id: String,
    access_token: String,
    secret_key: String,
}

impl AsrClient {
    pub fn new(app_id: String, access_token: String, secret_key: String) -> Self {
        Self {
            app_id,
            access_token,
            secret_key,
        }
    }

    fn generate_signature(&self, string_to_sign: &str) -> String {
        let mut mac = HmacSha256::new_from_slice(self.secret_key.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(string_to_sign.as_bytes());
        let result = mac.finalize();
        URL_SAFE_NO_PAD.encode(result.into_bytes())
    }

    fn build_auth_header(&self, method: &str, path: &str, headers_to_sign: &[(&str, &str)]) -> String {
        if !self.secret_key.is_empty() {
            let mut string_to_sign = format!("{} {} HTTP/1.1\n", method, path);
            let header_names: Vec<&str> = headers_to_sign.iter().map(|(k, _)| *k).collect();
            for (name, value) in headers_to_sign {
                string_to_sign.push_str(&format!("{}: {}\n", name, value));
            }
            let mac = self.generate_signature(&string_to_sign);
            let h_list = header_names.join(",");
            format!(
                "HMAC256; access_token=\"{}\"; mac=\"{}\"; h=\"{}\"",
                self.access_token, mac, h_list
            )
        } else {
            format!("Bearer; {}", self.access_token)
        }
    }

    fn build_seed_message(msg_type: u8, payload: &[u8], compress: bool) -> Vec<u8> {
        let compression = if compress { MESSAGE_COMPRESS_GZIP } else { MESSAGE_COMPRESS_NONE };

        let compressed_payload = if compress {
            let mut encoder = GzEncoder::new(Vec::with_capacity(payload.len()), Compression::default());
            encoder.write_all(payload).unwrap();
            encoder.finish().unwrap()
        } else {
            payload.to_vec()
        };

        let payload_len = compressed_payload.len() as u32;
        // 预分配精确大小: 4字节头 + 4字节长度 + payload
        let mut message = Vec::with_capacity(8 + compressed_payload.len());

        // Header
        message.push((PROTOCOL_VERSION << 4) | HEADER_SIZE);
        message.push((msg_type << 4) | 0x00);
        message.push((MESSAGE_SERIAL_JSON << 4) | compression);
        message.push(0x00);

        message.extend_from_slice(&payload_len.to_be_bytes());
        message.extend_from_slice(&compressed_payload);
        message
    }

    /// 构建音频消息 - 接受字节切片，避免额外分配
    fn build_audio_message(audio_data: &[u8]) -> Vec<u8> {
        let total_len = 8 + audio_data.len();
        let mut message = Vec::with_capacity(total_len);

        // Header
        message.push((PROTOCOL_VERSION << 4) | HEADER_SIZE);
        message.push((MESSAGE_TYPE_AUDIO_ONLY << 4) | 0x00);
        message.push(0x00);
        message.push(0x00);

        // Payload length
        message.extend_from_slice(&(audio_data.len() as u32).to_be_bytes());

        // Audio data
        message.extend_from_slice(audio_data);
        message
    }

    fn build_finish_message() -> Vec<u8> {
        vec![
            (PROTOCOL_VERSION << 4) | HEADER_SIZE,
            (MESSAGE_TYPE_AUDIO_ONLY << 4) | 0x02,
            0x00,
            0x00,
            0x00, 0x00, 0x00, 0x00, // payload length = 0
        ]
    }

    /// 解析服务器响应
    fn parse_response(data: &[u8]) -> Option<AsrResponse> {
        if data.len() < 4 {
            return None;
        }

        let header_size = (data[0] & 0x0f) as usize * 4;
        let message_type = data[1] >> 4;
        let message_type_specific_flags = data[1] & 0x0f;
        let message_compression = data[2] & 0x0f;

        if data.len() <= header_size {
            return None;
        }

        let payload = &data[header_size..];

        // 消息类型 0x09 是服务器响应（包含识别结果）
        if message_type == 0x09 {
            let skip_bytes = if message_type_specific_flags == 1 { 8 } else { 4 };
            if payload.len() < skip_bytes {
                return None;
            }

            let msg_data = &payload[skip_bytes..];
            let text_data = Self::decompress_if_needed(msg_data, message_compression);

            if let Ok(text) = String::from_utf8(text_data.into_owned()) {
                return serde_json::from_str(&text).ok();
            }
        }
        // 消息类型 0x0c 也可能是识别结果
        else if message_type == 0x0c {
            let text_data = Self::decompress_if_needed(payload, message_compression);
            if let Ok(text) = String::from_utf8(text_data.into_owned()) {
                return serde_json::from_str(&text).ok();
            }
        }

        None
    }

    fn decompress_if_needed(data: &[u8], compression: u8) -> Cow<'_, [u8]> {
        if compression == 1 {
            let mut decoder = GzDecoder::new(data);
            let mut decompressed = Vec::new();
            match decoder.read_to_end(&mut decompressed) {
                Ok(_) => Cow::Owned(decompressed),
                Err(_) => Cow::Borrowed(data),
            }
        } else {
            // 未压缩时直接借用，避免复制
            Cow::Borrowed(data)
        }
    }

    /// 连接并流式传输音频数据
    /// result_tx 发送 AsrResult，包含 prefetch 状态
    pub async fn connect_and_stream(
        &self,
        mut audio_rx: mpsc::Receiver<Vec<u8>>,
        result_tx: mpsc::Sender<AsrResult>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let connect_id = uuid::Uuid::new_v4().to_string();

        let uri: Uri = VOLCENGINE_ASR_URL.parse()?;
        let host = uri.host().unwrap_or("openspeech.bytedance.com");
        let path = uri.path();

        let headers_to_sign = vec![
            ("Host", host),
            ("X-Api-Resource-Id", RESOURCE_ID),
        ];

        let auth_header = self.build_auth_header("GET", path, &headers_to_sign);

        let request = Request::builder()
            .uri(VOLCENGINE_ASR_URL)
            .header("Host", host)
            .header("Authorization", &auth_header)
            .header("X-Api-App-Key", &self.app_id)
            .header("X-Api-Access-Key", &self.access_token)
            .header("X-Api-Resource-Id", RESOURCE_ID)
            .header("X-Api-Connect-Id", &connect_id)
            .header("Connection", "Upgrade")
            .header("Upgrade", "websocket")
            .header("Sec-WebSocket-Version", "13")
            .header(
                "Sec-WebSocket-Key",
                tokio_tungstenite::tungstenite::handshake::client::generate_key(),
            )
            .body(())?;

        log::info!("Connecting to ASR service");

        let (ws_stream, _response) = connect_async(request).await?;
        log::info!("WebSocket connected");

        let (mut write, mut read) = ws_stream.split();

        // 发送初始化配置
        let config = AsrConfig::default();
        let config_json = serde_json::to_vec(&config)?;
        let init_msg = Self::build_seed_message(MESSAGE_TYPE_FULL_CLIENT, &config_json, true);
        write.send(Message::Binary(init_msg)).await?;

        let (stop_tx, mut stop_rx) = mpsc::channel::<()>(1);

        // 发送音频数据的任务
        let send_task = tokio::spawn(async move {
            loop {
                tokio::select! {
                    audio_data = audio_rx.recv() => {
                        match audio_data {
                            Some(data) => {
                                let audio_msg = Self::build_audio_message(&data);
                                if write.send(Message::Binary(audio_msg)).await.is_err() {
                                    break;
                                }
                            }
                            None => {
                                log::info!("Audio channel closed, sending finish message");
                                let _ = write.send(Message::Binary(Self::build_finish_message())).await;
                                break;
                            }
                        }
                    }
                    _ = stop_rx.recv() => {
                        let _ = write.send(Message::Binary(Self::build_finish_message())).await;
                        break;
                    }
                }
            }
        });

        // 接收识别结果的任务
        let recv_task = tokio::spawn(async move {
            while let Some(msg) = read.next().await {
                match msg {
                    Ok(Message::Binary(data)) => {
                        if let Some(response) = Self::parse_response(&data) {
                            if response.is_success() {
                                let result_text = response.get_text();
                                if !result_text.is_empty() {
                                    let result = AsrResult {
                                        text: result_text,
                                        is_prefetch: response.is_prefetch(),
                                    };
                                    if result_tx.send(result).await.is_err() {
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    Ok(Message::Close(_)) => {
                        log::info!("WebSocket connection closed");
                        break;
                    }
                    Err(e) => {
                        log::error!("WebSocket error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }
            drop(stop_tx);
        });

        let _ = tokio::join!(send_task, recv_task);
        log::info!("ASR session completed");

        Ok(())
    }
}
