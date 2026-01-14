//! ASR Provider 实现模块

mod doubao;
mod whisper_api;
mod whisper_local;

pub use doubao::{DoubaoConfig, DoubaoProvider};
pub use whisper_api::{WhisperApiConfig, WhisperApiProvider};
pub use whisper_local::{WhisperLocalConfig, WhisperLocalProvider, WhisperModelSize};
