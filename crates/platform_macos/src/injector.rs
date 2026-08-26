#[cfg(target_os = "macos")]
mod macos_ffi {
    use std::os::raw::c_void;
    pub type CGEventSourceRef = *mut c_void;
    pub type CGEventRef = *mut c_void;
    pub type UniChar = u16;

    pub const kCGEventSourceStateHIDSystemState: u32 = 1;
    
    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        pub fn CGEventSourceCreate(stateID: u32) -> CGEventSourceRef;
        pub fn CGEventCreateKeyboardEvent(
            source: CGEventSourceRef,
            keycode: u16,
            keydown: bool,
        ) -> CGEventRef;
        pub fn CGEventPost(tapLocation: u32, event: CGEventRef);
        pub fn CGEventKeyboardSetUnicodeString(
            event: CGEventRef,
            stringLength: usize,
            unicodeString: *const UniChar,
        );
        pub fn CFRelease(cf: *mut c_void);
    }
}

pub fn replace_text(backspace_count: usize, replacement: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        use macos_ffi::*;
        unsafe {
            let source = CGEventSourceCreate(kCGEventSourceStateHIDSystemState);
            if source.is_null() {
                return Err("Failed to create event source".into());
            }

            let kCGHIDEventTap = 0; // Post to HID system
            let delete_keycode = 51; // macOS Delete key

            // 1. Backspace tuşlarını bas (Yanlış kelimeyi sil)
            for _ in 0..backspace_count {
                let down = CGEventCreateKeyboardEvent(source, delete_keycode, true);
                let up = CGEventCreateKeyboardEvent(source, delete_keycode, false);
                
                CGEventPost(kCGHIDEventTap, down);
                CGEventPost(kCGHIDEventTap, up);
                
                CFRelease(down);
                CFRelease(up);
            }

            // 2. Unicode string olarak yeni kelimeyi bas
            let utf16: Vec<u16> = replacement.encode_utf16().collect();
            if !utf16.is_empty() {
                let down = CGEventCreateKeyboardEvent(source, 0, true);
                CGEventKeyboardSetUnicodeString(down, utf16.len(), utf16.as_ptr());
                CGEventPost(kCGHIDEventTap, down);
                CFRelease(down);

                let up = CGEventCreateKeyboardEvent(source, 0, false);
                CGEventKeyboardSetUnicodeString(up, utf16.len(), utf16.as_ptr());
                CGEventPost(kCGHIDEventTap, up);
                CFRelease(up);
            }

            CFRelease(source);
        }
        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    Err("macOS injector cannot run on this platform".into())
}
