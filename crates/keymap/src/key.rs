//! Platform-neutral identity for a physical key.
//!
//! The whole premise of a layout switcher is that we must remember *which key
//! the user pressed*, not *which character appeared*. The character is a
//! function of the key and the active layout; the key is the invariant.
//!
//! Names follow the W3C UI Events `code` values so that each platform backend
//! has an unambiguous target to normalise its native codes into (Windows scan
//! codes, macOS virtual keycodes and Linux evdev codes all number keys
//! differently).

/// Number of keys in the alphanumeric block.
///
/// Only these keys differ between layouts. Function keys, arrows, modifiers and
/// the numeric keypad produce the same result in every layout, so they are not
/// part of the table — they are handled as buffer-breaking events instead.
pub const KEY_COUNT: usize = 47;

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
#[repr(u8)]
pub enum Key {
    // Digit row
    Backquote = 0,
    Digit1,
    Digit2,
    Digit3,
    Digit4,
    Digit5,
    Digit6,
    Digit7,
    Digit8,
    Digit9,
    Digit0,
    Minus,
    Equal,
    // Upper letter row
    KeyQ,
    KeyW,
    KeyE,
    KeyR,
    KeyT,
    KeyY,
    KeyU,
    KeyI,
    KeyO,
    KeyP,
    BracketLeft,
    BracketRight,
    Backslash,
    // Home row
    KeyA,
    KeyS,
    KeyD,
    KeyF,
    KeyG,
    KeyH,
    KeyJ,
    KeyK,
    KeyL,
    Semicolon,
    Quote,
    // Lower letter row
    KeyZ,
    KeyX,
    KeyC,
    KeyV,
    KeyB,
    KeyN,
    KeyM,
    Comma,
    Period,
    Slash,
}

impl Key {
    /// Index into a layout table.
    #[inline]
    pub const fn index(self) -> usize {
        self as usize
    }

    /// Every key in the alphanumeric block, in physical order.
    pub const ALL: [Key; KEY_COUNT] = {
        use Key::*;
        [
            Backquote,
            Digit1,
            Digit2,
            Digit3,
            Digit4,
            Digit5,
            Digit6,
            Digit7,
            Digit8,
            Digit9,
            Digit0,
            Minus,
            Equal,
            KeyQ,
            KeyW,
            KeyE,
            KeyR,
            KeyT,
            KeyY,
            KeyU,
            KeyI,
            KeyO,
            KeyP,
            BracketLeft,
            BracketRight,
            Backslash,
            KeyA,
            KeyS,
            KeyD,
            KeyF,
            KeyG,
            KeyH,
            KeyJ,
            KeyK,
            KeyL,
            Semicolon,
            Quote,
            KeyZ,
            KeyX,
            KeyC,
            KeyV,
            KeyB,
            KeyN,
            KeyM,
            Comma,
            Period,
            Slash,
        ]
    };
}

/// A single keypress: a physical key plus whether Shift was held.
///
/// AltGr / Option levels are deliberately absent. Neither the US nor the
/// Russian layout needs them, and admitting a level we cannot faithfully
/// reproduce on every platform would let a wrong character reach the user.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Stroke {
    pub key: Key,
    pub shift: bool,
}

impl Stroke {
    #[inline]
    pub const fn new(key: Key, shift: bool) -> Self {
        Self { key, shift }
    }
}
