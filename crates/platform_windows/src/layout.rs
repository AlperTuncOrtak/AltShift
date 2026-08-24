use std::ptr;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::GetKeyboardLayoutList;
use windows::Win32::UI::TextServices::HKL;
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowThreadProcessId, PostMessageW, WM_INPUTLANGCHANGEREQUEST,
};
use keymap::LayoutId;

/// Sistemde kurulu olan klavye düzenlerini HKL listesi olarak çeker
/// ve bizim bildiğimiz LayoutId'lere eşler.
pub fn get_installed_layouts() -> Vec<LayoutId> {
    unsafe {
        // Önce kaç tane layout kurulu olduğunu öğren
        let count = GetKeyboardLayoutList(None);
        if count <= 0 {
            return Vec::new();
        }

        let mut hkl_list = vec![HKL(0); count as usize];
        let fetched = GetKeyboardLayoutList(Some(&mut hkl_list));
        
        let mut layouts = Vec::new();
        for i in 0..fetched {
            let hkl_val = hkl_list[i as usize].0 as usize;
            
            // HKL'nin alt 16 biti dil (Language ID) kodudur.
            let lang_id = hkl_val & 0xFFFF;
            
            match lang_id {
                0x0409 => {
                    if !layouts.contains(&LayoutId::UsQwerty) {
                        layouts.push(LayoutId::UsQwerty);
                    }
                }
                0x0419 => {
                    if !layouts.contains(&LayoutId::RuYcuken) {
                        layouts.push(LayoutId::RuYcuken);
                    }
                }
                _ => {} // Desteklemediğimiz bir dil (Örn: Türkçe 0x041F)
            }
        }
        
        layouts
    }
}

/// Verilen pencerenin klavye düzenini istenen LayoutId'ye dönüştürme isteği (post message) yollar.
pub fn switch_layout(hwnd: HWND, target: LayoutId) -> Result<(), String> {
    if hwnd.0 == 0 {
        return Err("Geçersiz HWND".to_string());
    }

    // İstenen LayoutId'yi HKL değerine (Language ID) geri çevir
    let lang_id: usize = match target {
        LayoutId::UsQwerty => 0x0409, // English (US)
        LayoutId::RuYcuken => 0x0419, // Russian
    };

    // Sistemin kurulu HKL'lerini çek ve eşleşen tam HKL pointer'ını bul
    unsafe {
        let count = GetKeyboardLayoutList(None);
        if count <= 0 {
            return Err("Kurulu klavye düzeni bulunamadı".to_string());
        }

        let mut hkl_list = vec![HKL(0); count as usize];
        let fetched = GetKeyboardLayoutList(Some(&mut hkl_list));

        let mut target_hkl = None;
        for i in 0..fetched {
            let hkl_val = hkl_list[i as usize].0 as usize;
            if (hkl_val & 0xFFFF) == lang_id {
                target_hkl = Some(hkl_val);
                break;
            }
        }

        if let Some(hkl_val) = target_hkl {
            // Hedef pencereye klavye dili değiştirme emri gönder
            // WM_INPUTLANGCHANGEREQUEST: wParam=0, lParam=Hedef HKL
            let posted = PostMessageW(
                hwnd,
                WM_INPUTLANGCHANGEREQUEST,
                WPARAM(0),
                LPARAM(hkl_val as isize),
            );

            if posted.is_ok() {
                Ok(())
            } else {
                Err("WM_INPUTLANGCHANGEREQUEST gönderilemedi".to_string())
            }
        } else {
            Err("İstenen klavye düzeni sistemde kurulu değil".to_string())
        }
    }
}
