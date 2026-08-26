pub struct ActiveWindowInfo {
    pub process_id: u32,
    pub exe_name: String,
}

pub fn get_active_window_info() -> Option<ActiveWindowInfo> {
    #[cfg(target_os = "macos")]
    {
        // TODO: macOS API (e.g. NSWorkspace::sharedWorkspace().frontmostApplication())
        None
    }
    
    #[cfg(not(target_os = "macos"))]
    None
}

pub fn update_blacklist(blacklist: Vec<String>) {
    // TODO: cache blacklist globally
}
