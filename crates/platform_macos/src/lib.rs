#![cfg(target_os = "macos")]
//! macOS platform backend.
//!
//! Compiles to nothing off macOS, for the same reason as the Windows backend.

pub mod active_window;
pub mod backend;
pub mod hook;
pub mod injector;
pub mod key_map;
pub mod layout;
pub mod password;
