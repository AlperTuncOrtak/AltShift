//! macOS implementation of the shared [`platform`] contract.
//!
//! The modules behind it are still boilerplate, so this backend is deliberately
//! inert rather than optimistic -- see [`FocusContext::is_blocked`].

use crate::{active_window, injector, layout, password};
use keymap::LayoutId;
use platform::{FocusContext, KeyHook, LayoutController, Result, TextInjector};

pub struct MacOs;

impl TextInjector for MacOs {
    fn replace_text(&self, backspaces: usize, replacement: &str) -> Result<()> {
        injector::replace_text(backspaces, replacement)
    }
}

impl LayoutController for MacOs {
    fn installed_layouts(&self) -> Vec<LayoutId> {
        layout::get_installed_layouts()
    }

    fn switch_to(&self, target: LayoutId) -> Result<()> {
        layout::switch_layout(target)
    }
}

impl FocusContext for MacOs {
    fn active_process_name(&self) -> Option<String> {
        active_window::get_active_window_info().map(|w| w.exe_name)
    }

    fn is_password_field(&self) -> Option<bool> {
        password::is_focused_field_password()
    }

    /// Always blocked, for now.
    ///
    /// The macOS block list is not implemented yet. Returning `false` here
    /// would read as "this application is fine to correct in", which would let
    /// the backend type into terminals and password managers the moment it is
    /// wired up -- and nothing would signal that the list was never consulted.
    ///
    /// Blocking everything makes macOS do nothing instead of doing something
    /// unsafe. Flip this the same commit the block list lands, not before.
    fn is_blocked(&self) -> bool {
        true
    }
}

impl KeyHook for MacOs {
    fn run(&self) -> Result<()> {
        crate::hook::run_hook_loop()
    }
}
