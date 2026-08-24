use std::mem;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, VK_BACK,
};

/// Bize ait olan sahte tuş basımlarını ayırdetmek için sihirli sayı (dwExtraInfo).
/// WUL-19 (Kendi tuşlarımızı hook'ta tekrar dinlememek) için hayati önem taşır.
pub const ALTSHIFT_MAGIC_INFO: usize = 0x1234_5678;

/// N adet geri silme gönderip, üzerine verilen metni Unicode olarak (dilden bağımsız) yazar.
pub fn replace_text(backspace_count: usize, replacement: &str) -> Result<(), String> {
    let mut inputs = Vec::with_capacity(backspace_count * 2 + replacement.encode_utf16().count() * 2);

    // 1. Geri silme vuruşlarını hazırla (Bas ve Bırak)
    for _ in 0..backspace_count {
        // Backspace Down
        inputs.push(INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: windows::Win32::UI::Input::KeyboardAndMouse::INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_BACK,
                    wScan: 0,
                    dwFlags: Default::default(),
                    time: 0,
                    dwExtraInfo: ALTSHIFT_MAGIC_INFO,
                },
            },
        });
        // Backspace Up
        inputs.push(INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: windows::Win32::UI::Input::KeyboardAndMouse::INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_BACK,
                    wScan: 0,
                    dwFlags: KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: ALTSHIFT_MAGIC_INFO,
                },
            },
        });
    }

    // 2. Yeni metni Unicode olarak hazırla
    for code_unit in replacement.encode_utf16() {
        // Unicode Down
        inputs.push(INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: windows::Win32::UI::Input::KeyboardAndMouse::INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY(0),
                    wScan: code_unit,
                    dwFlags: KEYEVENTF_UNICODE,
                    time: 0,
                    dwExtraInfo: ALTSHIFT_MAGIC_INFO,
                },
            },
        });
        // Unicode Up
        inputs.push(INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: windows::Win32::UI::Input::KeyboardAndMouse::INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY(0),
                    wScan: code_unit,
                    dwFlags: KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: ALTSHIFT_MAGIC_INFO,
                },
            },
        });
    }

    // 3. Hepsini tek bir seferde işletim sistemine gönder (Maksimum hız)
    unsafe {
        let sent = SendInput(
            &inputs,
            mem::size_of::<INPUT>() as i32,
        );

        if sent == inputs.len() as u32 {
            Ok(())
        } else {
            Err(format!("SendInput failed. Expected to send {}, actually sent {}", inputs.len(), sent))
        }
    }
}
