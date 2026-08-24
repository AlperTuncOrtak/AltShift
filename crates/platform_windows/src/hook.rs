use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_KEYBOARD, KEYBDINPUT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, SetWindowsHookExW, UnhookWindowsHookEx, GetMessageW, 
    WH_KEYBOARD_LL, HHOOK, KBDLLHOOKSTRUCT, MSG,
};
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use crate::injector::ALTSHIFT_MAGIC_INFO;

/// Enjeksiyon sırasında (düzeltme yapılırken) basılan tuşların tutulduğu tampon.
static HELD_KEYS: Mutex<Vec<INPUT>> = Mutex::new(Vec::new());

/// Motorun metin enjekte ettiğini (ve dışarıdan gelen tuşların bekletilmesi gerektiğini) 
/// belirten atomik bayrak.
static INJECTION_LOCK: AtomicBool = AtomicBool::new(false);

/// Enjeksiyon işlemine başlamadan önce çağrılır. 
/// Bu andan itibaren kullanıcının bastığı tuşlar uygulamaya gitmez, tamponda birikir.
pub fn acquire_injection_lock() {
    INJECTION_LOCK.store(true, Ordering::SeqCst);
}

/// Enjeksiyon bittikten sonra çağrılır. Tamponda biriken tuşları sırasıyla 
/// işletim sistemine (uygulamaya) gönderir ve kilidi açar.
pub fn release_injection_lock_and_flush() {
    let mut keys = HELD_KEYS.lock().unwrap();
    
    if !keys.is_empty() {
        unsafe {
            SendInput(&keys, std::mem::size_of::<INPUT>() as i32);
        }
        keys.clear();
    }
    
    INJECTION_LOCK.store(false, Ordering::SeqCst);
}

/// Düşük seviyeli klavye kancası (WH_KEYBOARD_LL) işleyicisi (Callback).
/// WUL-19 gereği: Eğer enjeksiyon kilidi aktifse, tuşu yut (1 dön) ve tampona al.
/// Sihirli kendi tuşlarımızı ise doğrudan serbest bırak.
#[no_mangle]
pub unsafe extern "system" fn keyboard_hook_proc(
    n_code: i32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    if n_code < 0 {
        return CallNextHookEx(None, n_code, w_param, l_param);
    }

    let hook_struct = &*(l_param.0 as *const KBDLLHOOKSTRUCT);

    // Kural 1: Bizim enjekte ettiğimiz tuşlar (Sonsuz döngüyü önleme)
    if hook_struct.dwExtraInfo == ALTSHIFT_MAGIC_INFO {
        return CallNextHookEx(None, n_code, w_param, l_param);
    }

    // Kural 2: Eğer motor şu an metin siliyor/yazıyorsa (Yarış durumu koruması)
    if INJECTION_LOCK.load(Ordering::SeqCst) {
        let mut keys = HELD_KEYS.lock().unwrap();
        
        let input = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: windows::Win32::UI::Input::KeyboardAndMouse::INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY(hook_struct.vkCode as u16),
                    wScan: hook_struct.scanCode as u16,
                    dwFlags: windows::Win32::UI::Input::KeyboardAndMouse::KEYBD_EVENT_FLAGS(hook_struct.flags.0),
                    time: hook_struct.time,
                    dwExtraInfo: 0, // Orijinal tuş, sihirli numara yok
                },
            },
        };

        keys.push(input);
        
        // Tuşu yut (Uygulamaya gitmesin)
        return LRESULT(1);
    }

    // Normal işleyiş: Burada motor (engine) çağrılacak ve gerekirse engellenecek.
    // Şimdilik pas geçiyoruz.
    CallNextHookEx(None, n_code, w_param, l_param)
}

/// Hook'u sisteme kaydeder ve Windows mesaj döngüsünü başlatır.
/// Bu fonksiyon çağrıldığında mevcut thread sonsuz bir döngüye girer (bloklar).
/// Bu nedenle ayrı bir thread içinde çalıştırılmalıdır.
pub fn run_hook_loop() -> Result<(), String> {
    unsafe {
        let h_instance = GetModuleHandleW(None).map_err(|e| e.to_string())?;

        let hook_id = SetWindowsHookExW(
            WH_KEYBOARD_LL,
            Some(keyboard_hook_proc),
            h_instance,
            0,
        ).map_err(|e| e.to_string())?;

        // Windows Mesaj Döngüsü (GetMessage). Hook'un hayatta kalması için şart.
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).into() {
            // WH_KEYBOARD_LL hook'ları için TranslateMessage / DispatchMessage zorunlu değildir,
            // sadece mesaj kuyruğunu boşaltmak (GetMessage) hook'un canlı kalmasını sağlar.
        }

        // Döngü kırılırsa hook'u kaldır
        let _ = UnhookWindowsHookEx(hook_id);
    }
    
    Ok(())
}
