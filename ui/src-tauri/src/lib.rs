use std::sync::Mutex;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    enabled: bool,
    default_layout: String,
    aggressiveness: u8,
    blacklist: Vec<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            default_layout: "us-qwerty".into(),
            aggressiveness: 65,
            blacklist: vec!["code.exe".into(), "wezterm.exe".into(), "1password.exe".into()],
        }
    }
}

impl AppSettings {
    fn load_from_disk() -> Self {
        if let Ok(data) = std::fs::read_to_string("altshift_settings.json") {
            if let Ok(settings) = serde_json::from_str(&data) {
                return settings;
            }
        }
        Self::default()
    }

    fn save_to_disk(&self) {
        if let Ok(data) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write("altshift_settings.json", data);
        }
    }
}

pub struct AppState(Mutex<AppSettings>);

#[tauri::command]
fn get_settings(state: State<AppState>) -> AppSettings {
    state.0.lock().unwrap().clone()
}

#[tauri::command]
fn update_settings(settings: AppSettings, state: State<AppState>) {
    *state.0.lock().unwrap() = settings.clone();
    settings.save_to_disk();
    
    // Notify the engine/platform
    platform_windows::active_window::update_blacklist(settings.blacklist.clone());
}

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .manage(AppState(Mutex::new(AppSettings::load_from_disk())))
    .invoke_handler(tauri::generate_handler![get_settings, update_settings])
    .setup(|app| {
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }

      let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
      let menu = Menu::with_items(app, &[&quit_i])?;

      let _tray = TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            if event.id() == "quit" {
                app.exit(0);
            }
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

      Ok(())
    })
    .on_window_event(|window, event| match event {
        tauri::WindowEvent::CloseRequested { api, .. } => {
            window.hide().unwrap();
            api.prevent_close();
        }
        _ => {}
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
