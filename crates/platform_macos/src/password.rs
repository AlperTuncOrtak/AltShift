pub fn is_focused_field_password() -> Option<bool> {
    #[cfg(target_os = "macos")]
    {
        // TODO: Use AXUIElementCreateSystemWide() -> AXUIElementCopyAttributeValue(kAXFocusedUIElementAttribute) -> kAXValueIsSecure
        // and return Some(true/false).
        None
    }

    #[cfg(not(target_os = "macos"))]
    None
}
