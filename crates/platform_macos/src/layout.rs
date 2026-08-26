use keymap::LayoutId;

#[cfg(target_os = "macos")]
mod macos_ffi {
    use std::os::raw::c_void;
    pub type TISInputSourceRef = *mut c_void;
    pub type CFStringRef = *const c_void;
    
    #[link(name = "Carbon", kind = "framework")]
    extern "C" {
        pub fn TISCopyCurrentKeyboardInputSource() -> TISInputSourceRef;
        pub fn TISSelectInputSource(inputSource: TISInputSourceRef) -> i32;
        // pub fn TISCopyInputSourceForLanguage(language: CFStringRef) -> TISInputSourceRef;
        pub fn CFRelease(cf: *mut c_void);
    }
}

pub fn get_installed_layouts() -> Vec<LayoutId> {
    // Şimdilik sadece statik US/RU döndürüyoruz
    vec![LayoutId::UsQwerty, LayoutId::RuYcuken]
}

pub fn switch_layout(target: LayoutId) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        // TODO: Map LayoutId to macOS input source layout ID, find it via TISCreateInputSourceList,
        // and select it.
        // For now, this is a placeholder.
        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    Err("macOS layout switcher cannot run on this platform".into())
}
