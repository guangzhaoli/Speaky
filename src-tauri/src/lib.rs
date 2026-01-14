use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager,
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

mod asr;
mod audio;
mod commands;
mod history;
mod input;
mod logging;
mod postprocess;
mod state;

pub use state::AppState;

static SHORTCUT_PROCESSING: std::sync::LazyLock<Arc<AtomicBool>> =
    std::sync::LazyLock::new(|| Arc::new(AtomicBool::new(false)));

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 加载配置
    let config = state::AppConfig::load();

    // 初始化日志系统（使用配置中的设置）
    logging::init_logger(config.enable_logging);

    let shortcut = commands::parse_shortcut(&config.shortcut)
        .unwrap_or_else(|_| Shortcut::new(Some(Modifiers::ALT), Code::Space));

    // 检查是否为静默启动
    let silent_mode = commands::is_silent_mode();

    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, hotkey, event| {
                    if hotkey == &shortcut {
                        let processing = SHORTCUT_PROCESSING.clone();
                        let app_clone = app.clone();

                        match event.state() {
                            ShortcutState::Pressed => {
                                // 使用 compare_exchange 确保只有一个线程能启动录音
                                if processing.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
                                    return; // 已经在处理中
                                }
                                log::info!("Shortcut pressed - starting recording");
                                tauri::async_runtime::spawn(async move {
                                    if let Err(e) = commands::handle_start_recording(&app_clone).await {
                                        log::error!("Failed to start recording: {}", e);
                                        // 如果启动失败，重置状态
                                        SHORTCUT_PROCESSING.store(false, Ordering::SeqCst);
                                    }
                                });
                            }
                            ShortcutState::Released => {
                                // 只有在录音中才处理释放事件
                                if !processing.load(Ordering::SeqCst) {
                                    return;
                                }
                                log::info!("Shortcut released - stopping recording");
                                tauri::async_runtime::spawn(async move {
                                    if let Err(e) = commands::handle_stop_recording(&app_clone).await {
                                        log::error!("Failed to stop recording: {}", e);
                                    }
                                    SHORTCUT_PROCESSING.store(false, Ordering::SeqCst);
                                });
                            }
                        }
                    }
                })
                .build(),
        )
        .manage(AppState::default())
        .setup(move |app| {
            // 设置系统托盘
            setup_tray(app)?;

            let config = app.state::<AppState>().get_config();
            let shortcut = commands::parse_shortcut(&config.shortcut)
                .unwrap_or_else(|_| Shortcut::new(Some(Modifiers::ALT), Code::Space));
            app.global_shortcut().register(shortcut)?;
            log::info!("Global shortcut {} registered", config.shortcut);

            // 如果不是静默模式，显示窗口
            if !silent_mode {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }

            // 预热 LLM 连接（后台异步执行）
            let postprocess_config = config.postprocess.clone();
            tauri::async_runtime::spawn(async move {
                postprocess::warmup(&postprocess_config).await;
            });

            log::info!("Audio Input application started (silent: {})", silent_mode);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::start_recording,
            commands::stop_recording,
            commands::get_state,
            commands::get_config,
            commands::update_config,
            commands::get_transcript,
            commands::test_llm_connection,
            commands::get_audio_devices,
            commands::get_history,
            commands::delete_history_entry,
            commands::clear_history,
            commands::get_config_file_path,
            commands::get_config_file_content,
            commands::save_config_file_content,
            commands::get_log_info,
            commands::get_logs,
            commands::clear_logs,
            commands::set_logging_enabled,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let show = MenuItemBuilder::with_id("show", "显示窗口").build(app)?;
    let settings = MenuItemBuilder::with_id("settings", "设置").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "退出").build(app)?;

    let menu = MenuBuilder::new(app)
        .items(&[&show, &settings, &quit])
        .build()?;

    TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .tooltip("Audio Input - Alt+Space 开始录音")
        .on_menu_event(|app, event| match event.id().as_ref() {
            "quit" => {
                log::info!("Quit requested");
                app.exit(0);
            }
            "show" | "settings" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app)?;

    log::info!("System tray initialized");
    Ok(())
}
