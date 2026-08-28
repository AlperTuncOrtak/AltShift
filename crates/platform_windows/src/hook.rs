use crate::injector::ALTSHIFT_MAGIC_INFO;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::GetKeyState;
use windows::Win32::UI::Input::KeyboardAndMouse::{SendInput, INPUT, INPUT_KEYBOARD, KEYBDINPUT};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetForegroundWindow, GetMessageW, SetWindowsHookExW, UnhookWindowsHookEx,
    HHOOK, KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL,
};

/// Enjeksiyon sırasında (düzeltme yapılırken) basılan tuşların tutulduğu tampon.
static HELD_KEYS: Mutex<Vec<INPUT>> = Mutex::new(Vec::new());

/// Motorun metin enjekte ettiğini (ve dışarıdan gelen tuşların bekletilmesi gerektiğini)
/// belirten atomik bayrak.
static INJECTION_LOCK: AtomicBool = AtomicBool::new(false);

/// Hook'un sisteme gerçekten kurulup kurulmadığı.
///
/// Kurulamazsa program açık görünür ama hiçbir tuşu görmez -- dışarıdan
/// "çalışmıyor" ile ayırt edilemeyen bir durum.
pub static HOOK_INSTALLED: AtomicBool = AtomicBool::new(false);

use engine::{Buffer, Decision, Engine};
use guards::Context;
use keymap::{LayoutId, Stroke};

/// Istatistik: Toplam yapilan duzeltme sayisi
pub static TOTAL_CORRECTIONS: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

lazy_static::lazy_static! {
    /// Son kelimelerin oturduğu düzen. Karakter kanıtı kısa kelimelerde
    /// yetersiz kalıyor; "önceki kelimeler buradaydı" başka türden bir kanıt.
    pub static ref RECENT_LAYOUT: Mutex<Option<LayoutId>> = Mutex::new(None);
    /// Odak değişimini yakalamak için son görülen pencere.
    static ref LAST_HWND: Mutex<isize> = Mutex::new(0);
    /// Son kararın insan tarafından okunabilir özeti.
    ///
    /// Arayüzde gösterilmek için: kullanıcı "çalışmıyor" dediğinde log
    /// dosyasını bulmak zorunda kalmasın. Kelimenin kendisi değil, sadece
    /// gerekçesi tutuluyor.
    pub static ref LAST_DECISION: Mutex<String> = Mutex::new("henüz karar verilmedi".to_string());
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

    // Bu satırlar tanı için: log dosyasının boş olması ile uygulamanın hiç
    // çalışmaması aynı şeye benziyordu ve aradaki farkı anlamak bir derleme,
    // indirme ve kurulum turuna mal oldu. Artık program açıldıysa log konuşur.
    log::info!(
        "engine ready: installed layouts = {:?}",
        crate::layout::get_installed_layouts()
    );
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

    // Kelime sınırı tuşları. Bunlar `Key` enum'unda YOK ve olmamalı: enum
    // yalnızca düzenler arasında farklılaşan tuşları taşıyor, boşluk her
    // düzende boşluk.
    //
    // Karar bloğu daha önce `vk_to_key(...)` başarılı olursa çalışacak şekilde
    // yazılmıştı. Boşluk hiçbir zaman bir `Key`e çevrilemediği için o blok
    // hiç çalışmadı -- uygulama kurulup açılıyor, hook takılıyor, tuşlar
    // geliyordu ve motor tek bir kelimeye bile bakmıyordu.
    const VK_SPACE: u16 = 0x20;
    const VK_RETURN: u16 = 0x0D;
    const VK_TAB: u16 = 0x09;
    const VK_BACK: u16 = 0x08;

    /// Basılması tampona dokunmaması gereken tuşlar.
    ///
    /// Büyük harf yazmak için Shift'e basmak tamponu düşürseydi, büyük harfle
    /// başlayan hiçbir kelime düzeltilemezdi.
    fn is_modifier(vk: u16) -> bool {
        matches!(vk, 0x10..=0x12 | 0x14 | 0x5B..=0x5C | 0xA0..=0xA5)
    }

    if hook_struct.flags.0 & windows::Win32::UI::Input::KeyboardAndMouse::KEYEVENTF_KEYUP.0 == 0 {
        let vk = hook_struct.vkCode as u16;
        let mut buf = BUFFER.lock().unwrap();

        match vk {
            VK_SPACE | VK_RETURN | VK_TAB => {
                let hwnd = unsafe { GetForegroundWindow() };

                // Odak değiştiyse önceki kelimeler artık bu bağlama ait değil.
                // Hem tamponu hem de "son kelimeler hangi düzendeydi" geçmişini
                // düşürmek gerekiyor; taşımak, yanlış bağlamla karar vermek olur.
                let focus_changed = {
                    let mut last = LAST_HWND.lock().unwrap();
                    let changed = *last != hwnd.0;
                    *last = hwnd.0;
                    changed
                };
                if focus_changed {
                    buf.clear(engine::Break::FocusChange);
                    *RECENT_LAYOUT.lock().unwrap() = None;
                }

                // Gerçek bağlam. Buranın `Context::default()` olması, guards'ın
                // her kelimeyi şifre alanı sayıp engellemesi demekti -- motor
                // skorlamaya hiç ulaşmıyordu.
                // ponytail: UI Automation sorgusu her kelimede yapılıyor ve bu
                // kod hook callback'inin içinde. Yavaşsa iki sorun birden:
                // yazarken takılma, ve callback ~300 ms'de dönmezse Windows'un
                // hook'u sessizce kaldırması.
                //
                // Odak başına önbelleklemek cazip ama güvenli değil:
                // GetForegroundWindow en üstteki pencereyi verir, aynı pencere
                // içinde kullanıcı adından şifre alanına geçmek onu değiştirmez
                // -- yani önbellek tam korumamız gereken anda bayatlar.
                //
                // Doğru çözüm önbellek değil, kararı hook thread'inden çıkarmak.
                // Önce Windows'ta ölçelim: gerçekten yavaş mı?
                let ctx = Context {
                    is_password_field: crate::password::is_password_field(hwnd),
                    app_blocked: crate::active_window::is_active_app_blocked(),
                    sentence_initial: false,
                };

                // Kullanıcının o an hangi düzende yazdığı. Desteklemediğimiz bir
                // düzense (Türkçe, Almanca...) hiç karışmıyoruz.
                let Some(current_layout) = crate::layout::get_current_layout(hwnd) else {
                    buf.clear(engine::Break::WordEnd);
                    return CallNextHookEx(None, n_code, w_param, l_param);
                };

                // Sadece gerçekten kurulu düzenler aday olabilir: olmayan bir
                // düzene geçemeyiz, ve aday kümesini daraltmak yanlış
                // düzeltmeleri de azaltıyor.
                let available = crate::layout::get_installed_layouts();
                let recent = *RECENT_LAYOUT.lock().unwrap();

                let decision = {
                    let engine = ENGINE.lock().unwrap();
                    engine.decide(buf.strokes(), current_layout, &available, &ctx, recent)
                };

                // Tanı için: bir kelimeye neden dokunulmadığı, kullanıcı
                // "çalışmıyor" dediğinde tek tek koda bakmadan görülebilsin.
                //
                // Kelimenin KENDİSİ asla loglanmıyor -- sadece uzunluğu ve
                // kararın gerekçesi. Tuş dinleyen bir programın kullanıcı
                // metnini diske yazması, yapmayacağız dediğimiz şeyin ta kendisi.
                let outcome = match &decision {
                    Decision::Correct(c) => format!("düzeltildi → {:?}", c.target_layout),
                    Decision::Leave(Some(reason)) => format!("dokunulmadı: {reason:?}"),
                    Decision::Leave(None) => "dokunulmadı: eşiğin altında".to_string(),
                };
                log::info!(
                    "word len={} layout={:?} available={:?} pwd={:?} blocked={} -> {}",
                    buf.len(),
                    current_layout,
                    available,
                    ctx.is_password_field,
                    ctx.app_blocked,
                    outcome
                );
                *LAST_DECISION.lock().unwrap() = format!(
                    "{} harf, {:?} düzeninde, şifre alanı={:?} → {}",
                    buf.len(),
                    current_layout,
                    ctx.is_password_field,
                    outcome
                );

                match decision {
                    Decision::Correct(correction) => {
                        TOTAL_CORRECTIONS.fetch_add(1, Ordering::SeqCst);

                        // WUL-19: enjeksiyon sürerken gelen tuşlar tutulur.
                        crate::hook::acquire_injection_lock();
                        // Boşluk da düzeltmenin parçası olarak yazılıyor: orijinal
                        // boşluğu yutup buraya eklemek, sıranın karışmasını
                        // imkânsız kılar.
                        let text = format!("{} ", correction.to);
                        let injected = crate::injector::replace_text(correction.backspaces, &text);
                        crate::hook::release_injection_lock_and_flush();

                        if injected.is_ok() {
                            // Düzeltmenin asıl faydası burada: bundan sonrası
                            // zaten doğru düzende yazılır.
                            let _ = crate::layout::switch_layout(hwnd, correction.target_layout);
                            *RECENT_LAYOUT.lock().unwrap() = Some(correction.target_layout);
                        }

                        buf.clear(engine::Break::Applied);
                        return LRESULT(1);
                    }
                    Decision::Leave(_) => {
                        // Dokunmadığımız kelime, o düzenin doğru olduğuna dair
                        // kanıt: bir sonraki kelimede bağlam olarak kullanılıyor.
                        *RECENT_LAYOUT.lock().unwrap() = Some(current_layout);
                        buf.clear(engine::Break::WordEnd);
                    }
                }
            }
            VK_BACK => {
                // Kullanıcının sildiği karakter tampondan da düşmeli, yoksa
                // tampon ekranda olmayan tuşları hatırlar.
                buf.pop();
            }
            _ => {
                if let Some(key) = crate::vk_map::vk_to_key(vk) {
                    // Shift durumu gerçekten okunuyor: sabit `false` bırakmak
                    // büyük harfle başlayan her kelimeyi yanlış render etmek
                    // demekti.
                    let shift = unsafe {
                        GetKeyState(windows::Win32::UI::Input::KeyboardAndMouse::VK_SHIFT.0 as i32)
                            as u16
                            & 0x8000
                            != 0
                    };
                    buf.push(Stroke::new(key, shift));
                } else if !is_modifier(vk) {
                    // Ne karakter ne sınır: ok tuşları, Home, Delete, F1...
                    // İmleç yer değiştirmiş olabilir, yani tampon artık
                    // imlecin önündeki metne karşılık gelmiyor.
                    buf.clear(engine::Break::CaretMoved);
                }
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

        HOOK_INSTALLED.store(true, Ordering::SeqCst);
        log::info!("keyboard hook installed");

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
