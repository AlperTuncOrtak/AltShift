use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use windows::Win32::Foundation::{CloseHandle, HWND, MAX_PATH};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

/// Varsayılan kara liste (Terminal, IDE, Şifre Yöneticileri, Sanal Makineler)
const DEFAULT_BLACKLIST: &[&str] = &[
    // Terminaller
    "cmd.exe",
    "pwsh.exe",
    "powershell.exe",
    "WindowsTerminal.exe",
    "mintty.exe",
    // IDE'ler
    "Code.exe",
    "idea64.exe",
    "datagrip64.exe",
    "pycharm64.exe",
    "rider64.exe",
    "devenv.exe", // Visual Studio
    // Şifre Yöneticileri
    "1Password.exe",
    "Bitwarden.exe",
    "KeePassXC.exe",
    "KeePass.exe",
    // Sanal Makineler / Uzak Masaüstü
    "mstsc.exe",
    "vmware-view.exe",
    "vmconnect.exe",
];

/// O an ekranda aktif (odaklanmış) olan pencerenin exe adını döndürür
pub fn get_active_process_name() -> Option<String> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0 == 0 {
            return None;
        }

        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 {
            return None;
        }

        let process_handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        
        let mut buffer = [0u16; MAX_PATH as usize];
        let mut size = buffer.len() as u32;

        let success = QueryFullProcessImageNameW(
            process_handle,
            PROCESS_NAME_WIN32,
            windows::core::PWSTR(buffer.as_mut_ptr()),
            &mut size,
        );

        CloseHandle(process_handle).ok();

        if success.is_ok() && size > 0 {
            let path = OsString::from_wide(&buffer[..size as usize]);
            if let Some(path_str) = path.to_str() {
                // Sadece dosya adını (exe) ayıkla
                let exe_name = path_str.split('\\').last().unwrap_or(path_str);
                return Some(exe_name.to_string());
            }
        }
        None
    }
}

/// Aktif uygulamanın kara listede olup olmadığını söyler
pub fn is_active_app_blocked() -> bool {
    if let Some(exe_name) = get_active_process_name() {
        let exe_lower = exe_name.to_lowercase();
        DEFAULT_BLACKLIST
            .iter()
            .any(|&blocked| blocked.to_lowercase() == exe_lower)
    } else {
        false
    }
}
