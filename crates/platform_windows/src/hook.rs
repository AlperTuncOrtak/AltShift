use crate::injector::ALTSHIFT_MAGIC_INFO;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{SendInput, INPUT, INPUT_KEYBOARD, KEYBDINPUT};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetMessageW, SetWindowsHookExW, UnhookWindowsHookEx, HHOOK, KBDLLHOOKSTRUCT,
    MSG, WH_KEYBOARD_LL,
};

/// Enjeksiyon sırasında (düzeltme yapılırken) basılan tuşların tutulduğu tampon.
static HELD_KEYS: Mutex<Vec<INPUT>> = Mutex::new(Vec::new());

/// Motorun metin enjekte ettiğini (ve dışarıdan gelen tuşların bekletilmesi gerektiğini)
/// belirten atomik bayrak.
static INJECTION_LOCK: AtomicBool = AtomicBool::new(false);

use engine::{Buffer, Decision, Engine};
use guards::Context;
use keymap::{LayoutId, Stroke};

/// Istatistik: Toplam yapilan duzeltme sayisi
pub static TOTAL_CORRECTIONS: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

lazy_static::lazy_static! {
    pub static ref ENGINE: Mutex<Engine> = Mutex::new(Engine::new());
    pub static ref BUFFER: Mutex<Buffer> = Mutex::new(Buffer::new());
}

pub fn init_engine() {
    // Embedded on purpose: the shipped app must carry its models, not look
    // for a data/ directory on the user's machine. Building therefore requires
    // ./fetch-wordlists.sh to have run -- see README.
    let en_words = lang::parse_frequency_list(include_str!("../../../data/en.txt"));
    let ru_words = lang::parse_frequency_list(include_str!("../../../data/ru.txt"));

    let mut engine = ENGINE.lock().unwrap();
    *engine = Engine::new()
        .with_model(
            LayoutId::UsQwerty,
            lang::LanguageModel::train("en", en_words),
        )
        .with_model(
            LayoutId::RuYcuken,
            lang::LanguageModel::train("ru", ru_words),
        );

    println!("AltShift Engine initialized with EN and RU models.");
}

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
                    wVk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY(
                        hook_struct.vkCode as u16,
                    ),
                    wScan: hook_struct.scanCode as u16,
                    dwFlags: windows::Win32::UI::Input::KeyboardAndMouse::KEYBD_EVENT_FLAGS(
                        hook_struct.flags.0,
                    ),
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
    if hook_struct.flags.0 & windows::Win32::UI::Input::KeyboardAndMouse::KEYEVENTF_KEYUP.0 == 0 {
        if let Some(key) = crate::vk_map::vk_to_key(hook_struct.vkCode as u16) {
            let mut buf = BUFFER.lock().unwrap();

            // Eğer boşluk tuşuna basıldıysa (kelime sonu)
            if hook_struct.vkCode as u16 == windows::Win32::UI::Input::KeyboardAndMouse::VK_SPACE.0
            {
                // Şimdilik default Context ve LayoutId ile karar verelim
                let ctx = Context::default(); // TODO: get from active window
                let current_layout = LayoutId::UsQwerty; // TODO: get from active window

                let decision = {
                    let mut engine = ENGINE.lock().unwrap();
                    engine.decide(
                        buf.strokes(),
                        current_layout,
                        &[LayoutId::UsQwerty, LayoutId::RuYcuken],
                        &ctx,
                        None,
                    )
                };

                if let Decision::Correct(correction) = decision {
                    TOTAL_CORRECTIONS.fetch_add(1, Ordering::SeqCst);
                    // Düzeltmeyi uygula (enjekte et)
                    // WUL-19: Yarış durumu (race condition) koruması
                    crate::hook::acquire_injection_lock();
                    let _ = crate::injector::replace_text(correction.backspaces, &correction.to);
                    crate::hook::release_injection_lock_and_flush();

                    buf.clear(engine::Break::Applied);
                    return LRESULT(1); // Boşluğu yut? Veya boşluğu yazması için bırakalım?
                                       // Replace text metni yazar. Space sonradan gelebilir.
                } else {
                    buf.clear(engine::Break::WordEnd);
                }
            } else {
                // Sadece Shift tuşunu takip etmek için (basitçe)
                let shift = false; // TODO: handle shift state
                buf.push(Stroke::new(key, shift));
            }
        }
    }

    CallNextHookEx(None, n_code, w_param, l_param)
}

/// Hook'u sisteme kaydeder ve Windows mesaj döngüsünü başlatır.
/// Bu fonksiyon çağrıldığında mevcut thread sonsuz bir döngüye girer (bloklar).
/// Bu nedenle ayrı bir thread içinde çalıştırılmalıdır.
pub fn run_hook_loop() -> Result<(), String> {
    unsafe {
        // WH_KEYBOARD_LL is a global hook — hInstance MUST be None (null).
        // Passing the exe module handle here causes SetWindowsHookExW to fail.
        let hook_id = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook_proc), None, 0)
            .map_err(|e| format!("SetWindowsHookExW failed: {}", e))?;

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
