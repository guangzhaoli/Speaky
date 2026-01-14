use crate::asr::client::AsrClient;
use crate::asr::provider::{AsrResult, DownloadProgress, ModelInfo, ProviderInfo};
use crate::asr::providers::{DoubaoProvider, WhisperApiProvider, WhisperLocalProvider, WhisperModelSize};
use crate::asr::{AsrProvider, ModelDownloadable};
use crate::audio::capture::{list_audio_devices, AudioCaptureController, AudioDevice};
use crate::history::{History, HistoryEntry};
use crate::input::keyboard::KeyboardSimulator;
use crate::postprocess::{self, LlmProvider};
use crate::state::{AppConfig, AppState, AsrConfig, RecordingState};
use auto_launch::AutoLaunchBuilder;
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock};
use std::time::Instant;
use tauri::{command, AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};
use tokio::sync::mpsc;

/// 键盘输入命令
pub enum KeyboardCommand {
    UpdateText(String),
    Finish,
}

// 全局状态 (使用标准库 LazyLock 替代 lazy_static)
static STOP_SIGNAL: LazyLock<Arc<AtomicBool>> = LazyLock::new(|| Arc::new(AtomicBool::new(false)));
static AUDIO_TX: LazyLock<Arc<Mutex<Option<mpsc::Sender<Vec<u8>>>>>> =
    LazyLock::new(|| Arc::new(Mutex::new(None)));
static ASR_COMPLETE_RX: LazyLock<Arc<Mutex<Option<tokio::sync::oneshot::Receiver<()>>>>> =
    LazyLock::new(|| Arc::new(Mutex::new(None)));
// 全局键盘模拟器（复用）
static KEYBOARD: LazyLock<Arc<Mutex<Option<KeyboardSimulator>>>> =
    LazyLock::new(|| Arc::new(Mutex::new(None)));
// 键盘输入命令通道
static KEYBOARD_TX: LazyLock<Arc<Mutex<Option<std::sync::mpsc::Sender<KeyboardCommand>>>>> =
    LazyLock::new(|| Arc::new(Mutex::new(None)));

/// 获取或创建键盘模拟器
fn get_keyboard() -> Result<parking_lot::MutexGuard<'static, Option<KeyboardSimulator>>, String> {
    let mut guard = KEYBOARD.lock();
    if guard.is_none() {
        *guard = Some(KeyboardSimulator::new()?);
    }
    Ok(guard)
}

/// 发送键盘命令（非阻塞）
fn send_keyboard_command(cmd: KeyboardCommand) {
    let tx = KEYBOARD_TX.lock();
    if let Some(sender) = tx.as_ref() {
        let _ = sender.send(cmd);
    }
}

/// 启动键盘输入后台线程
fn start_keyboard_thread() -> std::sync::mpsc::Sender<KeyboardCommand> {
    let (tx, rx) = std::sync::mpsc::channel::<KeyboardCommand>();

    std::thread::spawn(move || {
        loop {
            match rx.recv() {
                Ok(KeyboardCommand::UpdateText(text)) => {
                    if let Ok(mut guard) = get_keyboard() {
                        if let Some(keyboard) = guard.as_mut() {
                            if let Err(e) = keyboard.update_text(&text) {
                                log::error!("Failed to update text: {}", e);
                            }
                        }
                    }
                }
                Ok(KeyboardCommand::Finish) => {
                    if let Ok(mut guard) = get_keyboard() {
                        if let Some(keyboard) = guard.as_mut() {
                            keyboard.finish_realtime_input();
                        }
                    }
                }
                Err(_) => {
                    // 通道关闭，退出线程
                    break;
                }
            }
        }
    });

    tx
}

/// 确保键盘线程已启动
fn ensure_keyboard_thread() {
    let mut tx_guard = KEYBOARD_TX.lock();
    if tx_guard.is_none() {
        *tx_guard = Some(start_keyboard_thread());
    }
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
    let old_config = state.get_config();

    // 如果快捷键变更，更新注册
    if old_config.shortcut != config.shortcut {
        update_shortcut(&app, &old_config.shortcut, &config.shortcut)?;
    }

    // 如果开机启动变更，更新自启动设置
    if old_config.auto_start != config.auto_start {
        update_auto_launch(config.auto_start, config.silent_start)?;
    } else if old_config.silent_start != config.silent_start && config.auto_start {
        // 只有静默启动变更且开机启动开启时，更新启动参数
        update_auto_launch(config.auto_start, config.silent_start)?;
    }

    state.update_config(config)
}

#[command]
pub fn get_transcript(app: AppHandle) -> Result<String, String> {
    let state = app.state::<AppState>();
    Ok(state.get_transcript())
}

#[command]
pub async fn test_llm_connection(provider: LlmProvider) -> Result<String, String> {
    postprocess::test_connection(&provider).await
}

#[command]
pub fn get_audio_devices() -> Vec<AudioDevice> {
    list_audio_devices()
}

#[command]
pub fn get_history() -> Vec<HistoryEntry> {
    History::load().entries
}

#[command]
pub fn delete_history_entry(id: String) -> Result<(), String> {
    let mut history = History::load();
    if history.delete_entry(&id) {
        history.save()?;
        Ok(())
    } else {
        Err("Entry not found".to_string())
    }
}

#[command]
pub fn clear_history() -> Result<(), String> {
    let mut history = History::load();
    history.clear();
    history.save()
}

#[command]
pub fn get_config_file_path() -> Result<String, String> {
    use directories::ProjectDirs;
    ProjectDirs::from("com", "speaky", "Speaky")
        .map(|dirs| {
            dirs.config_dir()
                .join("config.toml")
                .to_string_lossy()
                .to_string()
        })
        .ok_or_else(|| "Failed to get config path".to_string())
}

#[command]
pub fn get_config_file_content() -> Result<String, String> {
    use directories::ProjectDirs;
    use std::fs;

    let path = ProjectDirs::from("com", "speaky", "Speaky")
        .map(|dirs| dirs.config_dir().join("config.toml"))
        .ok_or_else(|| "Failed to get config path".to_string())?;

    if path.exists() {
        fs::read_to_string(&path).map_err(|e| format!("Failed to read config file: {}", e))
    } else {
        Ok(String::new())
    }
}

#[command]
pub fn save_config_file_content(content: String, app: AppHandle) -> Result<(), String> {
    use directories::ProjectDirs;
    use std::fs;

    let path = ProjectDirs::from("com", "speaky", "Speaky")
        .map(|dirs| dirs.config_dir().join("config.toml"))
        .ok_or_else(|| "Failed to get config path".to_string())?;

    // 创建配置目录
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create config dir: {}", e))?;
    }

    // 先验证 TOML 格式
    let config: AppConfig =
        toml::from_str(&content).map_err(|e| format!("Invalid TOML format: {}", e))?;

    // 写入文件
    fs::write(&path, &content).map_err(|e| format!("Failed to write config file: {}", e))?;

    // 更新内存中的配置
    let state = app.state::<AppState>();
    *state.config.write() = config;

    log::info!("Config file saved and reloaded");
    Ok(())
}

#[derive(serde::Serialize)]
pub struct LogInfo {
    pub path: String,
    pub size: u64,
    pub enabled: bool,
}

#[command]
pub fn get_log_info(app: AppHandle) -> LogInfo {
    let state = app.state::<AppState>();
    let config = state.get_config();
    LogInfo {
        path: crate::logging::log_file_path()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default(),
        size: crate::logging::log_file_size(),
        enabled: config.enable_logging,
    }
}

#[command]
pub fn get_logs(max_lines: Option<usize>) -> Result<Vec<String>, String> {
    crate::logging::read_logs(max_lines.unwrap_or(500))
}

#[command]
pub fn clear_logs() -> Result<(), String> {
    crate::logging::clear_logs()
}

#[command]
pub fn set_logging_enabled(enabled: bool, app: AppHandle) -> Result<(), String> {
    // 更新运行时状态
    crate::logging::set_logging_enabled(enabled);

    // 更新配置
    let state = app.state::<AppState>();
    let mut config = state.get_config();
    config.enable_logging = enabled;
    state.update_config(config)?;

    log::info!(
        "Logging {} by user",
        if enabled { "enabled" } else { "disabled" }
    );
    Ok(())
}

// ============ ASR Provider 相关命令 ============

/// 获取 ASR 配置
#[command]
pub fn get_asr_config(app: AppHandle) -> AsrConfig {
    let state = app.state::<AppState>();
    state.get_config().asr
}

/// 更新 ASR 配置
#[command]
pub fn update_asr_config(app: AppHandle, asr_config: AsrConfig) -> Result<(), String> {
    let state = app.state::<AppState>();
    let mut config = state.get_config();
    config.asr = asr_config;
    state.update_config(config)
}

/// 列出所有可用的 ASR Provider
#[command]
pub fn list_asr_providers(app: AppHandle) -> Vec<ProviderInfo> {
    let state = app.state::<AppState>();
    let config = state.get_config();
    let mut providers = Vec::new();

    // 豆包
    if let Some(ref doubao_config) = config.asr.doubao {
        let provider = DoubaoProvider::new(doubao_config.clone());
        providers.push(provider.info());
    } else {
        // 即使没配置也显示
        let provider = DoubaoProvider::new(Default::default());
        providers.push(provider.info());
    }

    // Whisper 本地
    let whisper_local = WhisperLocalProvider::new(
        config.asr.whisper_local.clone().unwrap_or_default(),
    );
    providers.push(whisper_local.info());

    // Whisper API
    if let Some(ref api_config) = config.asr.whisper_api {
        let provider = WhisperApiProvider::new(api_config.clone());
        providers.push(provider.info());
    } else {
        let provider = WhisperApiProvider::new(Default::default());
        providers.push(provider.info());
    }

    providers
}

/// 获取 Whisper 模型列表
#[command]
pub fn get_whisper_models(app: AppHandle) -> Vec<ModelInfo> {
    let state = app.state::<AppState>();
    let config = state.get_config();
    let provider = WhisperLocalProvider::new(
        config.asr.whisper_local.clone().unwrap_or_default(),
    );
    provider.available_models()
}

/// 下载 Whisper 模型
#[command]
pub async fn download_whisper_model(app: AppHandle, model_id: String) -> Result<(), String> {
    let state = app.state::<AppState>();
    let config = state.get_config();
    let provider = WhisperLocalProvider::new(
        config.asr.whisper_local.clone().unwrap_or_default(),
    );

    let (progress_tx, mut progress_rx) = mpsc::channel::<DownloadProgress>(32);

    // 转发进度到前端
    let app_clone = app.clone();
    tokio::spawn(async move {
        while let Some(progress) = progress_rx.recv().await {
            let _ = app_clone.emit("model-download-progress", &progress);
        }
    });

    // 执行下载
    provider
        .download_model(&model_id, progress_tx)
        .await
        .map_err(|e| e.to_string())?;

    // 发送完成事件
    let _ = app.emit("model-download-complete", &model_id);
    Ok(())
}

/// 删除 Whisper 模型
#[command]
pub async fn delete_whisper_model(app: AppHandle, model_id: String) -> Result<(), String> {
    let state = app.state::<AppState>();
    let config = state.get_config();
    let provider = WhisperLocalProvider::new(
        config.asr.whisper_local.clone().unwrap_or_default(),
    );

    provider
        .delete_model(&model_id)
        .await
        .map_err(|e| e.to_string())
}

/// 取消 Whisper 模型下载
#[command]
pub fn cancel_whisper_download(app: AppHandle) {
    let state = app.state::<AppState>();
    let config = state.get_config();
    let provider = WhisperLocalProvider::new(
        config.asr.whisper_local.clone().unwrap_or_default(),
    );
    provider.cancel_download();
}

/// 设置当前使用的 Whisper 模型
#[command]
pub fn set_whisper_model(app: AppHandle, model_id: String) -> Result<(), String> {
    let model_size = WhisperModelSize::from_filename(&model_id)
        .ok_or_else(|| format!("未知模型: {}", model_id))?;

    let state = app.state::<AppState>();
    let mut config = state.get_config();

    let mut whisper_config = config.asr.whisper_local.unwrap_or_default();
    whisper_config.model_size = model_size;
    config.asr.whisper_local = Some(whisper_config);

    state.update_config(config)
}

/// 解析快捷键字符串为 Shortcut
pub fn parse_shortcut(shortcut_str: &str) -> Result<Shortcut, String> {
    let parts: Vec<&str> = shortcut_str.split('+').map(|s| s.trim()).collect();

    let mut modifiers: Option<Modifiers> = None;
    let mut key_code: Option<Code> = None;

    for part in parts {
        let part_lower = part.to_lowercase();
        match part_lower.as_str() {
            "alt" | "option" => {
                modifiers = Some(modifiers.unwrap_or(Modifiers::empty()) | Modifiers::ALT);
            }
            "ctrl" | "control" => {
                modifiers = Some(modifiers.unwrap_or(Modifiers::empty()) | Modifiers::CONTROL);
            }
            "shift" => {
                modifiers = Some(modifiers.unwrap_or(Modifiers::empty()) | Modifiers::SHIFT);
            }
            "super" | "meta" | "cmd" | "command" | "win" => {
                modifiers = Some(modifiers.unwrap_or(Modifiers::empty()) | Modifiers::SUPER);
            }
            "space" => key_code = Some(Code::Space),
            "enter" | "return" => key_code = Some(Code::Enter),
            "tab" => key_code = Some(Code::Tab),
            "escape" | "esc" => key_code = Some(Code::Escape),
            "backspace" => key_code = Some(Code::Backspace),
            "delete" => key_code = Some(Code::Delete),
            "up" => key_code = Some(Code::ArrowUp),
            "down" => key_code = Some(Code::ArrowDown),
            "left" => key_code = Some(Code::ArrowLeft),
            "right" => key_code = Some(Code::ArrowRight),
            "home" => key_code = Some(Code::Home),
            "end" => key_code = Some(Code::End),
            "pageup" => key_code = Some(Code::PageUp),
            "pagedown" => key_code = Some(Code::PageDown),
            "f1" => key_code = Some(Code::F1),
            "f2" => key_code = Some(Code::F2),
            "f3" => key_code = Some(Code::F3),
            "f4" => key_code = Some(Code::F4),
            "f5" => key_code = Some(Code::F5),
            "f6" => key_code = Some(Code::F6),
            "f7" => key_code = Some(Code::F7),
            "f8" => key_code = Some(Code::F8),
            "f9" => key_code = Some(Code::F9),
            "f10" => key_code = Some(Code::F10),
            "f11" => key_code = Some(Code::F11),
            "f12" => key_code = Some(Code::F12),
            // 字母键
            s if s.len() == 1 => {
                let c = s.chars().next().unwrap();
                key_code = match c {
                    'a' => Some(Code::KeyA),
                    'b' => Some(Code::KeyB),
                    'c' => Some(Code::KeyC),
                    'd' => Some(Code::KeyD),
                    'e' => Some(Code::KeyE),
                    'f' => Some(Code::KeyF),
                    'g' => Some(Code::KeyG),
                    'h' => Some(Code::KeyH),
                    'i' => Some(Code::KeyI),
                    'j' => Some(Code::KeyJ),
                    'k' => Some(Code::KeyK),
                    'l' => Some(Code::KeyL),
                    'm' => Some(Code::KeyM),
                    'n' => Some(Code::KeyN),
                    'o' => Some(Code::KeyO),
                    'p' => Some(Code::KeyP),
                    'q' => Some(Code::KeyQ),
                    'r' => Some(Code::KeyR),
                    's' => Some(Code::KeyS),
                    't' => Some(Code::KeyT),
                    'u' => Some(Code::KeyU),
                    'v' => Some(Code::KeyV),
                    'w' => Some(Code::KeyW),
                    'x' => Some(Code::KeyX),
                    'y' => Some(Code::KeyY),
                    'z' => Some(Code::KeyZ),
                    '0' => Some(Code::Digit0),
                    '1' => Some(Code::Digit1),
                    '2' => Some(Code::Digit2),
                    '3' => Some(Code::Digit3),
                    '4' => Some(Code::Digit4),
                    '5' => Some(Code::Digit5),
                    '6' => Some(Code::Digit6),
                    '7' => Some(Code::Digit7),
                    '8' => Some(Code::Digit8),
                    '9' => Some(Code::Digit9),
                    _ => return Err(format!("Unknown key: {}", part)),
                };
            }
            _ => return Err(format!("Unknown key or modifier: {}", part)),
        }
    }

    let code = key_code.ok_or_else(|| "No key specified in shortcut".to_string())?;
    Ok(Shortcut::new(modifiers, code))
}

/// 更新全局快捷键
fn update_shortcut(app: &AppHandle, old_shortcut: &str, new_shortcut: &str) -> Result<(), String> {
    let global_shortcut = app.global_shortcut();

    // 解析新快捷键
    let new = parse_shortcut(new_shortcut)?;

    // 先尝试注册新快捷键（检查是否被占用）
    if let Err(e) = global_shortcut.register(new.clone()) {
        return Err(format!(
            "Shortcut '{}' is already in use or invalid: {}",
            new_shortcut, e
        ));
    }

    // 注册成功后，注销旧快捷键
    if let Ok(old) = parse_shortcut(old_shortcut) {
        let _ = global_shortcut.unregister(old);
    }

    log::info!("Shortcut updated from {} to {}", old_shortcut, new_shortcut);
    Ok(())
}

/// 更新开机启动设置
fn update_auto_launch(enable: bool, silent: bool) -> Result<(), String> {
    let app_name = "Speaky";

    // 获取当前可执行文件路径
    let exe_path = std::env::current_exe().map_err(|e| format!("Failed to get exe path: {}", e))?;

    let exe_path_str = exe_path.to_string_lossy().to_string();

    // 构建启动参数
    let args: Vec<String> = if silent {
        vec!["--silent".to_string()]
    } else {
        vec![]
    };

    let auto_launch = AutoLaunchBuilder::new()
        .set_app_name(app_name)
        .set_app_path(&exe_path_str)
        .set_args(&args)
        .build()
        .map_err(|e| format!("Failed to build auto launch: {}", e))?;

    if enable {
        auto_launch
            .enable()
            .map_err(|e| format!("Failed to enable auto launch: {}", e))?;
        log::info!("Auto launch enabled (silent: {})", silent);
    } else {
        auto_launch
            .disable()
            .map_err(|e| format!("Failed to disable auto launch: {}", e))?;
        log::info!("Auto launch disabled");
    }

    Ok(())
}

/// 检查是否为静默启动模式
pub fn is_silent_mode() -> bool {
    std::env::args().any(|arg| arg == "--silent")
}

/// 显示指示器窗口（屏幕底部居中）
fn show_indicator(app: &AppHandle) {
    if let Some(indicator) = app.get_webview_window("indicator") {
        // 获取主显示器信息并定位到底部居中
        if let Ok(Some(monitor)) = indicator.primary_monitor() {
            let screen_size = monitor.size();
            let scale_factor = indicator.scale_factor().unwrap_or(1.0);

            // 设置窗口大小（考虑 HiDPI 缩放）
            let window_width = (140.0 * scale_factor) as u32;
            let window_height = (50.0 * scale_factor) as u32;
            let _ = indicator.set_size(PhysicalSize::new(window_width, window_height));

            // 计算屏幕中心底部位置
            let x = (screen_size.width as i32 - window_width as i32) / 2;
            // 距离底部 80 像素（逻辑像素）
            let y = screen_size.height as i32 - window_height as i32 - (80.0 * scale_factor) as i32;

            let _ = indicator.set_position(PhysicalPosition::new(x, y));
        }
        let _ = indicator.show();
    }
}

/// 隐藏指示器窗口
fn hide_indicator(app: &AppHandle) {
    if let Some(indicator) = app.get_webview_window("indicator") {
        let _ = indicator.hide();
    }
}

pub async fn handle_start_recording(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();

    if state.get_recording_state() == RecordingState::Recording {
        return Err("Already recording".to_string());
    }

    let config = state.get_config();

    // 显示指示器窗口（如果启用）- 在配置检查前显示，以便测试 UI
    if config.show_indicator {
        show_indicator(app);
    }

    // 根据 active_provider 选择 ASR Provider 并验证配置
    let provider_error: Option<&str> = match config.asr.active_provider.as_str() {
        "doubao" => {
            match &config.asr.doubao {
                Some(cfg) if cfg.is_configured() => None,
                _ => Some("请先配置豆包 App ID 和 Access Token"),
            }
        }
        "whisper_local" => {
            let whisper_config = config.asr.whisper_local.clone().unwrap_or_default();
            let provider = WhisperLocalProvider::new(whisper_config);
            if provider.is_ready() { None } else { Some("请先下载 Whisper 模型") }
        }
        "whisper_api" => {
            match &config.asr.whisper_api {
                Some(cfg) if cfg.is_configured() => None,
                _ => Some("请先配置 Whisper API Key"),
            }
        }
        _ => Some("未知的 ASR Provider"),
    };

    if let Some(error_msg) = provider_error {
        // 发送未配置事件
        let _ = app.emit("indicator-not-configured", ());
        // 延迟隐藏指示器
        let app_clone = app.clone();
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            hide_indicator(&app_clone);
        });
        return Err(error_msg.to_string());
    }

    state.set_recording_state(RecordingState::Recording);
    state.clear_transcript();

    // 如果启用实时输入，确保键盘线程已启动
    if config.realtime_input {
        ensure_keyboard_thread();
    }
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
    let mut capture = AudioCaptureController::with_device(config.audio_device.clone());
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

    // 根据 active_provider 启动对应的 ASR
    match config.asr.active_provider.as_str() {
        "doubao" => {
            // 使用原有的豆包 ASR 客户端（性能更好的流式实现）
            let doubao_config = config.asr.doubao.clone().unwrap_or_default();
            let asr_client = AsrClient::new(
                doubao_config.app_id,
                doubao_config.access_token,
                doubao_config.secret_key,
            );

            // 创建内部结果通道，转换格式
            let (internal_tx, mut internal_rx) = mpsc::channel::<crate::asr::client::AsrResult>(32);

            // 启动格式转换任务
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

            tokio::spawn(async move {
                if let Err(e) = asr_client.connect_and_stream(audio_rx, internal_tx).await {
                    log::error!("ASR session error: {}", e);
                }
            });
        }
        "whisper_local" => {
            let mut whisper_config = config.asr.whisper_local.clone().unwrap_or_default();
            // 使用统一的语言设置
            whisper_config.language = config.asr_language.clone();
            let provider = WhisperLocalProvider::new(whisper_config);
            tokio::spawn(async move {
                if let Err(e) = provider.transcribe_stream(audio_rx, result_tx).await {
                    log::error!("Whisper local ASR error: {}", e);
                }
            });
        }
        "whisper_api" => {
            let mut api_config = config.asr.whisper_api.clone().unwrap_or_default();
            // 使用统一的语言设置
            if config.asr_language != "auto" {
                api_config.language = Some(config.asr_language.clone());
            } else {
                api_config.language = None;
            }
            let provider = WhisperApiProvider::new(api_config);
            tokio::spawn(async move {
                if let Err(e) = provider.transcribe_stream(audio_rx, result_tx).await {
                    log::error!("Whisper API ASR error: {}", e);
                }
            });
        }
        _ => {
            return Err("未知的 ASR Provider".to_string());
        }
    }

    // 处理识别结果 - 带节流和 prefetch 检测
    let app_clone = app.clone();
    let realtime_input = config.auto_type && config.realtime_input;

    // 如果启用实时输入，重置键盘状态
    if realtime_input {
        if let Ok(mut guard) = get_keyboard() {
            if let Some(keyboard) = guard.as_mut() {
                keyboard.reset_input_state();
            }
        }
    }

    tokio::spawn(async move {
        let mut final_text = String::new();
        let mut last_emit = Instant::now();
        const THROTTLE_MS: u128 = 100;

        while let Some(result) = result_rx.recv().await {
            // 直接移动 result.text，避免多次 clone
            let text = result.text;
            let is_final = result.is_final;

            // 更新 state
            let state = app_clone.state::<AppState>();
            state.set_transcript(text.clone());

            // 节流：每 100ms 最多发送一次事件和实时输入
            if last_emit.elapsed().as_millis() >= THROTTLE_MS {
                let _ = app_clone.emit("transcript-update", &text);

                // 实时输入到当前焦点窗口（使用专用线程通道，避免频繁创建线程）
                if realtime_input && !text.is_empty() {
                    send_keyboard_command(KeyboardCommand::UpdateText(text.clone()));
                }

                last_emit = Instant::now();
            }

            // 如果是最终结果，保存它
            if is_final {
                final_text = text;
            } else {
                // 中间结果也更新
                final_text = text;
            }
        }

        // 使用最终结果
        if !final_text.is_empty() {
            let state = app_clone.state::<AppState>();
            let config = state.get_config();

            // 后处理（仅非实时输入模式）
            let processed_result = if config.postprocess.enabled && !realtime_input {
                match postprocess::process_text(&final_text, &config.postprocess).await {
                    Ok(text) => text,
                    Err(e) => {
                        log::error!("Postprocess failed: {}", e);
                        final_text.clone()
                    }
                }
            } else {
                final_text.clone()
            };

            log::info!("ASR completed: {} -> {}", final_text, processed_result);
            state.set_transcript(processed_result.clone());

            // 保存到历史记录
            {
                let mut history = crate::history::History::load();
                history.add_entry(processed_result.clone());
                if let Err(e) = history.save() {
                    log::error!("Failed to save history: {}", e);
                }
            }

            // 发送最终结果事件
            let _ = app_clone.emit("transcript-update", &processed_result);

            // 实时输入模式下，完成时再次更新确保最终文本正确
            if realtime_input {
                send_keyboard_command(KeyboardCommand::UpdateText(final_text.clone()));
                send_keyboard_command(KeyboardCommand::Finish);
            }
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
        let _ = tokio::time::timeout(tokio::time::Duration::from_millis(2000), rx).await;
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

        // 实时输入模式下跳过最后的粘贴/输入（已经实时输入了）
        if !config.realtime_input {
            // 键盘输入（在独立线程中执行以避免影响 X11 状态）
            if config.auto_type && config.auto_copy {
                let result = tokio::task::spawn_blocking(move || match get_keyboard() {
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
                })
                .await;
                if let Err(e) = result {
                    log::error!("Keyboard task failed: {}", e);
                }
            } else if config.auto_type {
                let transcript_clone = transcript.clone();
                let result = tokio::task::spawn_blocking(move || match get_keyboard() {
                    Ok(mut guard) => {
                        if let Some(keyboard) = guard.as_mut() {
                            if let Err(e) = keyboard.type_text(&transcript_clone) {
                                log::error!("Failed to type text: {}", e);
                            } else {
                                log::info!("Text typed successfully");
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to get keyboard simulator: {}", e);
                    }
                })
                .await;
                if let Err(e) = result {
                    log::error!("Keyboard task failed: {}", e);
                }
            }
        }
    }

    state.set_recording_state(RecordingState::Idle);

    // 隐藏指示器窗口
    hide_indicator(app);

    app.emit("recording-stopped", &transcript)
        .map_err(|e| e.to_string())?;

    log::info!("Recording stopped, transcript: {}", transcript);
    Ok(transcript)
}
