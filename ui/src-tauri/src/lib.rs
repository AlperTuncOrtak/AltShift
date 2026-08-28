use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::State;

#[cfg(target_os = "windows")]
use platform_windows as platform;

#[cfg(target_os = "macos")]
use platform_macos as platform;

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
            blacklist: vec![
                "code.exe".into(),
                "wezterm.exe".into(),
                "1password.exe".into(),
            ],
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
    platform::active_window::update_blacklist(settings.blacklist.clone());
}

#[derive(Serialize)]
struct AppStats {
    total_corrections: usize,
}

/// Programın kendi durumu hakkında söyleyebilecekleri.
///
/// "Çalışmıyor" demek kolay, sebebini bulmak zordu: log dosyasının yerini
/// bulmak, PowerShell açmak, çıktıyı okumak. Bunların hepsi kullanıcıdan
/// beklenemeyecek şeyler ve her turu bir derleme-indirme-kurulum döngüsüne
/// mal ediyordu. Aynı bilgi artık ayarlar ekranında.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostics {
    version: String,
    hook_installed: bool,
    installed_layouts: Vec<String>,
    last_decision: String,
}

#[tauri::command]
fn get_diagnostics() -> Diagnostics {
    let version = env!("CARGO_PKG_VERSION").to_string();

    #[cfg(target_os = "windows")]
    {
        use std::sync::atomic::Ordering;
        Diagnostics {
            version,
            hook_installed: platform::hook::HOOK_INSTALLED.load(Ordering::SeqCst),
            installed_layouts: platform::layout::get_installed_layouts()
                .iter()
                .map(|l| format!("{l:?}"))
                .collect(),
            last_decision: platform::hook::LAST_DECISION.lock().unwrap().clone(),
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        Diagnostics {
            version,
            hook_installed: false,
            installed_layouts: Vec::new(),
            last_decision: "bu platformda klavye katmanı henüz yok".to_string(),
        }
    }
}

/// Pencereyi tepsiye indirir.
///
/// Arayüzdeki X düğmesi bunu Rust tarafından yapıyor, çünkü webview'dan
/// `getCurrentWindow().hide()` çağrısı Tauri'nin izin sistemine takılıp
/// sessizce hata veriyordu -- try-catch onu yutunca düğme hiçbir şey
/// yapmıyormuş gibi görünüyordu.
#[tauri::command]
fn hide_to_tray(window: tauri::Window) {
    let _ = window.hide();
}

#[tauri::command]
fn get_stats() -> AppStats {
    use std::sync::atomic::Ordering;
    AppStats {
        total_corrections: platform::hook::TOTAL_CORRECTIONS.load(Ordering::SeqCst),
    }
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
        .invoke_handler(tauri::generate_handler![
            get_settings,
            update_settings,
            get_stats,
            get_diagnostics,
            hide_to_tray
        ])
        .setup(|app| {
            // Logging is on in release too, not just debug. A keyboard hook
            // that silently does nothing is the hardest kind of bug to report:
            // the first "it doesn't work" cost a full build, download and
            // install cycle to diagnose, with nothing on disk to look at.
            //
            // What gets written is decision metadata only -- word *length*,
            // layouts, whether the field was a password, why a word was left
            // alone. Never the word. A program that watches keystrokes writing
            // user text to disk is precisely what this project promises not to
            // do, and a log file is still disk.
            app.handle().plugin(
                tauri_plugin_log::Builder::default()
                    .level(log::LevelFilter::Info)
                    .target(tauri_plugin_log::Target::new(
                        tauri_plugin_log::TargetKind::LogDir {
                            file_name: Some("altshift".into()),
                        },
                    ))
                    .build(),
            )?;

            // Initialize Language Models (trains trigrams in memory)
            platform::hook::init_engine();

            // Start the keyboard hook in a background thread.
            std::thread::spawn(|| {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    platform::hook::run_hook_loop()
                }));
                match result {
                    Ok(Ok(())) => {}
                    Ok(Err(e)) => log::error!("Hook loop error: {}", e),
                    Err(_) => log::error!("Hook loop panicked — continuing without hook"),
                }
            });

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

            // Apply Mica backdrop on Windows 11 for native glass effect
            #[cfg(target_os = "windows")]
            {
                use window_vibrancy::apply_mica;
                if let Some(window) = app.get_webview_window("main") {
                    let _ = apply_mica(&window, Some(true)); // true = dark mica
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }

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
