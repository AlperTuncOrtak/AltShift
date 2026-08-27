//! Windows implementation of the shared [`platform`] contract.
//!
//! Thin delegation on purpose: the working code lives in the sibling modules,
//! and this file exists only so the engine can talk to Windows through the same
//! interface it will use for macOS and Linux.

use crate::{active_window, injector, layout, password};
use keymap::LayoutId;
use platform::{FocusContext, KeyHook, LayoutController, Result, TextInjector};
use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

pub struct Windows;

impl TextInjector for Windows {
    fn replace_text(&self, backspaces: usize, replacement: &str) -> Result<()> {
        injector::replace_text(backspaces, replacement)
    }
}

impl LayoutController for Windows {
    fn installed_layouts(&self) -> Vec<LayoutId> {
        layout::get_installed_layouts()
    }

    fn switch_to(&self, target: LayoutId) -> Result<()> {
        // The handle is resolved here rather than taken as an argument: a
        // window handle in the trait would be a Windows detail every other
        // platform then has to pretend to have.
        layout::switch_layout(unsafe { GetForegroundWindow() }, target)
    }
}

impl FocusContext for Windows {
    fn active_process_name(&self) -> Option<String> {
        active_window::get_active_process_name()
    }

    fn is_password_field(&self) -> Option<bool> {
        password::is_password_field(unsafe { GetForegroundWindow() })
    }

    fn is_blocked(&self) -> bool {
        active_window::is_active_app_blocked()
    }
}

impl KeyHook for Windows {
    fn run(&self) -> Result<()> {
        crate::hook::run_hook_loop()
    }
}
