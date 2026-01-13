use crate::asr::client::{AsrClient, AsrResult};
use crate::audio::capture::AudioCaptureController;
use crate::input::keyboard::KeyboardSimulator;
use crate::state::{AppConfig, AppState, RecordingState};
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tauri::{command, AppHandle, Emitter, Manager};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tokio::sync::mpsc;

// 全局状态
lazy_static::lazy_static! {
    static ref STOP_SIGNAL: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    static ref AUDIO_TX: Arc<Mutex<Option<mpsc::Sender<Vec<u8>>>>> = Arc::new(Mutex::new(None));
    static ref ASR_COMPLETE_RX: Arc<Mutex<Option<tokio::sync::oneshot::Receiver<()>>>> = Arc::new(Mutex::new(None));
    // 全局键盘模拟器（复用）
    static ref KEYBOARD: Arc<Mutex<Option<KeyboardSimulator>>> = Arc::new(Mutex::new(None));
}

/// 获取或创建键盘模拟器
fn get_keyboard() -> Result<parking_lot::MutexGuard<'static, Option<KeyboardSimulator>>, String> {
    let mut guard = KEYBOARD.lock();
    if guard.is_none() {
        *guard = Some(KeyboardSimulator::new()?);
    }
    Ok(guard)
}

#[command]
pub async fn start_recording(app: AppHandle) -> Result<(), String> {
    handle_start_recording(&app).await
}

#[command]
pub async fn stop_recording(app: AppHandle) -> Result<String, String> {
    handle_stop_recording(&app).await
}

#[command]
pub fn get_state(app: AppHandle) -> Result<String, String> {
    let state = app.state::<AppState>();
    let recording_state = state.get_recording_state();
    serde_json::to_string(&recording_state).map_err(|e| e.to_string())
}

#[command]
pub fn get_config(app: AppHandle) -> Result<AppConfig, String> {
    let state = app.state::<AppState>();
    Ok(state.get_config())
}

#[command]
pub fn update_config(app: AppHandle, config: AppConfig) -> Result<(), String> {
    let state = app.state::<AppState>();
    state.update_config(config);
    Ok(())
}

#[command]
pub fn get_transcript(app: AppHandle) -> Result<String, String> {
    let state = app.state::<AppState>();
    Ok(state.get_transcript())
}

pub async fn handle_start_recording(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();

    if state.get_recording_state() == RecordingState::Recording {
        return Err("Already recording".to_string());
    }

    let config = state.get_config();
    if config.app_id.is_empty() || config.access_token.is_empty() {
        return Err("Please configure App ID and Access Token first".to_string());
    }

    state.set_recording_state(RecordingState::Recording);
    state.clear_transcript();
    STOP_SIGNAL.store(false, Ordering::SeqCst);

    app.emit("recording-started", ())
        .map_err(|e| e.to_string())?;

    // 创建通道
    let (audio_tx, audio_rx) = mpsc::channel::<Vec<u8>>(100);
    let (result_tx, mut result_rx) = mpsc::channel::<AsrResult>(10);

    // ASR 完成通知
    let (complete_tx, complete_rx) = tokio::sync::oneshot::channel::<()>();
    *ASR_COMPLETE_RX.lock() = Some(complete_rx);

    *AUDIO_TX.lock() = Some(audio_tx.clone());

    // 启动音频采集
    let (pcm_tx, pcm_rx) = std::sync::mpsc::channel();
    let mut capture = AudioCaptureController::new();
    capture.start_recording(pcm_tx)?;

    // 音频转发线程 - 使用 bytemuck 零拷贝
    let audio_tx_clone = audio_tx.clone();
    let stop_signal = STOP_SIGNAL.clone();
    std::thread::spawn(move || {
        while let Ok(samples) = pcm_rx.recv() {
            if stop_signal.load(Ordering::SeqCst) {
                break;
            }
            // 零拷贝转换: &[i16] -> &[u8]
            let bytes: &[u8] = bytemuck::cast_slice(&samples);
            if audio_tx_clone.blocking_send(bytes.to_vec()).is_err() {
                break;
            }
        }
        drop(capture);
    });

    // ASR 客户端
    let asr_client = AsrClient::new(config.app_id, config.access_token, config.secret_key);
    tokio::spawn(async move {
        if let Err(e) = asr_client.connect_and_stream(audio_rx, result_tx).await {
            log::error!("ASR session error: {}", e);
        }
    });

    // 处理识别结果 - 带节流和 prefetch 检测
    let app_clone = app.clone();
    tokio::spawn(async move {
        let mut final_text = String::new();
        let mut prefetch_text: Option<String> = None;
        let mut last_emit = Instant::now();
        const THROTTLE_MS: u128 = 100;

        while let Some(result) = result_rx.recv().await {
            final_text = result.text.clone();

            // 如果收到 prefetch，保存它
            if result.is_prefetch {
                prefetch_text = Some(result.text.clone());
                log::info!("Prefetch result received: {}", result.text);
            }

            // 更新 state
            let state = app_clone.state::<AppState>();
            state.set_transcript(result.text.clone());

            // 节流：每 100ms 最多发送一次事件
            if last_emit.elapsed().as_millis() >= THROTTLE_MS {
                let _ = app_clone.emit("transcript-update", &result.text);
                last_emit = Instant::now();
            }
        }

        // 使用 prefetch 结果（如果有）或最终结果
        let final_result = prefetch_text.unwrap_or(final_text);
        if !final_result.is_empty() {
            let state = app_clone.state::<AppState>();
            state.set_transcript(final_result.clone());
            log::info!("ASR completed: {}", final_result);
        }

        // 通知完成
        let _ = complete_tx.send(());
    });

    log::info!("Recording started");
    Ok(())
}

pub async fn handle_stop_recording(app: &AppHandle) -> Result<String, String> {
    let state = app.state::<AppState>();

    if state.get_recording_state() != RecordingState::Recording {
        return Err("Not recording".to_string());
    }

    state.set_recording_state(RecordingState::Processing);
    STOP_SIGNAL.store(true, Ordering::SeqCst);

    // 关闭音频通道
    {
        let mut tx = AUDIO_TX.lock();
        *tx = None;
    }

    // 等待 ASR 完成（最多 2 秒）
    let complete_rx = ASR_COMPLETE_RX.lock().take();
    if let Some(rx) = complete_rx {
        let _ = tokio::time::timeout(
            tokio::time::Duration::from_millis(2000),
            rx,
        ).await;
    }

    let transcript = state.get_transcript();
    let config = state.get_config();

    if !transcript.is_empty() {
        // 复制到剪贴板
        if config.auto_copy {
            if let Err(e) = app.clipboard().write_text(&transcript) {
                log::error!("Failed to copy to clipboard: {}", e);
            } else {
                log::info!("Text copied to clipboard");
            }
        }

        // 键盘输入（复用全局实例）
        if config.auto_type && config.auto_copy {
            match get_keyboard() {
                Ok(mut guard) => {
                    if let Some(keyboard) = guard.as_mut() {
                        if let Err(e) = keyboard.paste() {
                            log::error!("Failed to paste text: {}", e);
                        } else {
                            log::info!("Text pasted successfully");
                        }
                    }
                }
                Err(e) => {
                    log::error!("Failed to get keyboard simulator: {}", e);
                }
            }
        } else if config.auto_type {
            match get_keyboard() {
                Ok(mut guard) => {
                    if let Some(keyboard) = guard.as_mut() {
                        if let Err(e) = keyboard.type_text(&transcript) {
                            log::error!("Failed to type text: {}", e);
                        } else {
                            log::info!("Text typed successfully");
                        }
                    }
                }
                Err(e) => {
                    log::error!("Failed to get keyboard simulator: {}", e);
                }
            }
        }
    }

    state.set_recording_state(RecordingState::Idle);

    app.emit("recording-stopped", &transcript)
        .map_err(|e| e.to_string())?;

    log::info!("Recording stopped, transcript: {}", transcript);
    Ok(transcript)
}
