use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use lazy_static::lazy_static;

use engine::{Engine, Buffer, Decision, Break};
use keymap::{LayoutId, Stroke};
use guards::Context;

pub static TOTAL_CORRECTIONS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

static INJECTION_LOCK: AtomicBool = AtomicBool::new(false);

lazy_static! {
    pub static ref ENGINE: Mutex<Engine> = Mutex::new(Engine::new());
    pub static ref BUFFER: Mutex<Buffer> = Mutex::new(Buffer::new());
}

pub fn init_engine() {
    let en_words = include_str!("../../../data/en.txt").lines().filter_map(|line| {
            let (word, count) = line.trim().split_once(' ')?;
            Some((word.to_string(), count.parse().unwrap_or(1)))
        });
    let ru_words = include_str!("../../../data/ru.txt").lines().filter_map(|line| {
            let (word, count) = line.trim().split_once(' ')?;
            Some((word.to_string(), count.parse().unwrap_or(1)))
        });
    let tr_words = include_str!("../../../data/tr.txt").lines().filter_map(|line| {
            let (word, count) = line.trim().split_once(' ')?;
            Some((word.to_string(), count.parse().unwrap_or(1)))
        });
    
    let mut engine = ENGINE.lock().unwrap();
    *engine = Engine::new()
        .with_model(LayoutId::UsQwerty, lang::LanguageModel::train("en", en_words))
        .with_model(LayoutId::RuYcuken, lang::LanguageModel::train("ru", ru_words))
        .with_model(LayoutId::TrQwerty, lang::LanguageModel::train("tr", tr_words));
    
    println!("AltShift Engine initialized with EN and RU models.");
}

pub fn acquire_injection_lock() {
    INJECTION_LOCK.store(true, Ordering::SeqCst);
}

pub fn release_injection_lock_and_flush() {
    INJECTION_LOCK.store(false, Ordering::SeqCst);
}

#[cfg(target_os = "macos")]
mod macos_ffi {
    use std::os::raw::c_void;

    pub type CGEventRef = *mut c_void;
    pub type CFMachPortRef = *mut c_void;
    pub type CFRunLoopSourceRef = *mut c_void;
    pub type CFRunLoopRef = *mut c_void;
    pub type CGEventTapProxy = *mut c_void;
    pub type CGEventType = u32;

    pub const kCGSessionEventTap: u32 = 1;
    pub const kCGHeadInsertEventTap: u32 = 0;
    pub const kCGEventTapOptionDefault: u32 = 0;
    
    pub const kCGEventKeyDown: u32 = 10;
    pub const kCGEventFlagsChanged: u32 = 12;
    pub const kCGKeyboardEventKeycode: u32 = 9;

    pub type CGEventTapCallBack = extern "C" fn(
        proxy: CGEventTapProxy,
        type_: CGEventType,
        event: CGEventRef,
        user_info: *mut c_void,
    ) -> CGEventRef;

    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        pub fn CGEventTapCreate(
            tap: u32,
            place: u32,
            options: u32,
            eventsOfInterest: u64,
            callback: CGEventTapCallBack,
            userInfo: *mut c_void,
        ) -> CFMachPortRef;

        pub fn CGEventTapEnable(tap: CFMachPortRef, enable: bool);
        pub fn CGEventGetIntegerValueField(event: CGEventRef, field: u32) -> i64;
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        pub fn CFMachPortCreateRunLoopSource(
            allocator: *mut c_void,
            port: CFMachPortRef,
            order: isize,
        ) -> CFRunLoopSourceRef;

        pub fn CFRunLoopGetCurrent() -> CFRunLoopRef;
        pub fn CFRunLoopAddSource(rl: CFRunLoopRef, source: CFRunLoopSourceRef, mode: *const c_void);
        pub fn CFRunLoopRun();
        
        pub static kCFRunLoopCommonModes: *const c_void;
    }
}

#[cfg(target_os = "macos")]
extern "C" fn cg_event_callback(
    _proxy: macos_ffi::CGEventTapProxy,
    type_: macos_ffi::CGEventType,
    event: macos_ffi::CGEventRef,
    _user_info: *mut std::os::raw::c_void,
) -> macos_ffi::CGEventRef {
    // 1. Eğer injection kilidi aktifse, tuşları yutmadan geçir (veya yut, yarış durumuna göre değişir)
    // Şimdilik pas geçiyoruz.
    if INJECTION_LOCK.load(Ordering::SeqCst) {
        return event;
    }

    if type_ == macos_ffi::kCGEventKeyDown {
        let keycode = unsafe { macos_ffi::CGEventGetIntegerValueField(event, macos_ffi::kCGKeyboardEventKeycode) } as u16;
        
        if let Some(key) = crate::key_map::mac_keycode_to_key(keycode) {
            let mut buf = BUFFER.lock().unwrap();
            
            // Space (kelime sonu) - macOS keycode for space is 49
            if keycode == 49 {
                let ctx = Context::default(); // TODO: get from active window
                let current_layout = LayoutId::UsQwerty; // TODO: get from active window
                
                let decision = {
                    let mut engine = ENGINE.lock().unwrap();
                    engine.decide(buf.strokes(), current_layout, &[LayoutId::UsQwerty, LayoutId::RuYcuken], &ctx, None)
                };
                
                if let Decision::Correct(correction) = decision {
                    TOTAL_CORRECTIONS.fetch_add(1, Ordering::SeqCst);
                    
                    acquire_injection_lock();
                    let _ = crate::injector::replace_text(correction.backspaces, &correction.to);
                    release_injection_lock_and_flush();
                    
                    buf.clear(Break::Applied);
                    return std::ptr::null_mut(); // Tuşu yut
                } else {
                    buf.clear(Break::WordEnd);
                }
            } else {
                let shift = false; // TODO: handle shift state (via CGEventFlagsChanged tracking)
                buf.push(Stroke::new(key, shift));
            }
        }
    }
    
    event // Tuşu sisteme geri bırak
}

#[cfg(target_os = "macos")]
pub fn run_hook_loop() -> Result<(), String> {
    use macos_ffi::*;
    
    unsafe {
        let event_mask = (1 << kCGEventKeyDown) | (1 << kCGEventFlagsChanged);
        
        let tap = CGEventTapCreate(
            kCGSessionEventTap,
            kCGHeadInsertEventTap,
            kCGEventTapOptionDefault,
            event_mask,
            cg_event_callback,
            std::ptr::null_mut(),
        );
        
        if tap.is_null() {
            return Err("CGEventTapCreate failed. Do you have Accessibility permissions?".to_string());
        }
        
        let run_loop_source = CFMachPortCreateRunLoopSource(std::ptr::null_mut(), tap, 0);
        let run_loop = CFRunLoopGetCurrent();
        
        CFRunLoopAddSource(run_loop, run_loop_source, kCFRunLoopCommonModes);
        CGEventTapEnable(tap, true);
        
        println!("Starting macOS CGEventTap loop...");
        CFRunLoopRun();
    }
    
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn run_hook_loop() -> Result<(), String> {
    // Windows'tayken derleme hatası vermemesi için boş stub
    Err("macOS hook cannot run on this platform".into())
}
