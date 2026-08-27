use std::ptr;
use windows::core::ComInterface;
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
};
use windows::Win32::UI::Accessibility::{CUIAutomation, IUIAutomation, UIA_IsPasswordPropertyId};
use windows::Win32::UI::WindowsAndMessaging::{
    GetGUIThreadInfo, GetWindowLongW, GUITHREADINFO, GWL_STYLE,
};

const ES_PASSWORD: i32 = 0x0020;

/// Şifre alanı tespitini yapar. WUL-20 Kuralları:
/// 1. Klasik Win32 kontrolü (Hızlı)
/// 2. UI Automation kontrolü (Chrome, Electron, vs için - Yavaş)
/// Sonuç emin olunamıyorsa `None` döner (Güvenli kalmak için).
pub fn is_password_field(hwnd: HWND) -> Option<bool> {
    if hwnd.0 == 0 {
        return None;
    }

    // 1. Önce klasik ve çok hızlı Win32 yöntemi
    if let Some(is_pwd) = check_win32_password(hwnd) {
        if is_pwd {
            return Some(true); // Win32 kesin şifre dedi
        }
    }

    // 2. Win32 bulamadıysa (örneğin Chrome ise), UIA'ya düş
    check_uia_password()
}

/// Klasik Win32 Edit kontrolleri için şifre alanı kontrolü
fn check_win32_password(hwnd: HWND) -> Option<bool> {
    unsafe {
        let thread_id =
            windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(hwnd, None);
        if thread_id == 0 {
            return None;
        }

        let mut gui_info = GUITHREADINFO {
            cbSize: std::mem::size_of::<GUITHREADINFO>() as u32,
            ..Default::default()
        };

        // İlgili uygulamanın thread bilgilerini alıyoruz
        if GetGUIThreadInfo(thread_id, &mut gui_info).is_err() {
            return None;
        }

        let focus_hwnd = gui_info.hwndFocus;
        if focus_hwnd.0 == 0 {
            return None;
        }

        let style = GetWindowLongW(focus_hwnd, GWL_STYLE);
        if (style & ES_PASSWORD) == ES_PASSWORD {
            Some(true)
        } else {
            Some(false)
        }
    }
}

/// UI Automation (Tarayıcılar, Electron, UWP) için şifre alanı kontrolü.
/// Not: Bu işlem görece yavaştır, bu yüzden hook içinde her tuşta DEĞİL,
/// sadece pencere veya odak değiştiğinde çağrılıp önbelleğe alınmalıdır.
fn check_uia_password() -> Option<bool> {
    unsafe {
        // COM başlat (zaten başlatılmışsa zararı yok)
        let _ = CoInitializeEx(Some(ptr::null()), COINIT_MULTITHREADED);

        // UIA nesnesini yarat
        let uia: Result<IUIAutomation, _> =
            CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER);
        let uia = match uia {
            Ok(u) => u,
            Err(_) => return None,
        };

        // Odaktaki elementi al (örn: Chrome içindeki şifre kutusu)
        let element = match uia.GetFocusedElement() {
            Ok(e) => e,
            Err(_) => return None,
        };

        // Elementin IsPassword özelliğini sor
        if let Ok(is_password) = element.CurrentIsPassword() {
            return Some(is_password.as_bool());
        }

        None
    }
}
