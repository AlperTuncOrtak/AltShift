//! Physical keys and keyboard layout tables.
//!
//! This crate is deliberately free of any OS dependency: it is the piece that
//! lets the correction engine be developed and measured on one machine and
//! shipped to every platform unchanged.

pub mod key;
pub mod layout;

pub use key::{Key, Stroke, KEY_COUNT};
pub use layout::{Layout, LayoutId, Script, RU_YCUKEN, US_QWERTY};
