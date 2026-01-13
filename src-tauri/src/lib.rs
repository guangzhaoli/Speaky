use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager,
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

mod asr;
mod audio;
mod commands;
mod input;
mod state;

pub use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let shortcut = Shortcut::new(Some(Modifiers::ALT), Code::Space);

    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, hotkey, event| {
                    if hotkey == &shortcut {
                        let app_clone = app.clone();
                        match event.state() {
                            ShortcutState::Pressed => {
                                log::info!("Shortcut pressed - starting recording");
                                tauri::async_runtime::spawn(async move {
                                    if let Err(e) = commands::handle_start_recording(&app_clone).await {
                                        log::error!("Failed to start recording: {}", e);
                                    }
                                });
                            }
                            ShortcutState::Released => {
                                log::info!("Shortcut released - stopping recording");
                                tauri::async_runtime::spawn(async move {
                                    if let Err(e) = commands::handle_stop_recording(&app_clone).await {
                                        log::error!("Failed to stop recording: {}", e);
                                    }
                                });
                            }
                        }
                    }
                })
                .build(),
        )
        .manage(AppState::default())
        .setup(|app| {
            // 设置系统托盘
            setup_tray(app)?;

            // 注册全局快捷键
            let shortcut = Shortcut::new(Some(Modifiers::ALT), Code::Space);
            app.global_shortcut().register(shortcut)?;
            log::info!("Global shortcut Alt+Space registered");

            log::info!("Audio Input application started");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::start_recording,
            commands::stop_recording,
            commands::get_state,
            commands::get_config,
            commands::update_config,
            commands::get_transcript,
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
