#![cfg(windows)]
//! Windows platform backend.
//!
//! Compiles to nothing off Windows so that `cargo test --workspace` keeps
//! working on every developer machine -- the engine is platform-free by
//! design, and it must stay testable without a Windows box.

pub mod active_window;
pub mod app_memory;
pub mod backend;
pub mod hook;
pub mod injector;
pub mod layout;
pub mod password;
pub mod undo;
pub mod vk_map;
