//! The contract every platform backend implements.
//!
//! This crate holds no code that does anything -- only the shape the operating
//! system layer must take. It exists so that adding a third platform does not
//! mean revisiting every call site in the engine, and so that two people can
//! work on opposite sides of the boundary without meeting in the middle of a
//! function.
//!
//! Nothing here mentions a window handle, a process id, or an event tap. Each
//! method acts on *whatever the user is currently typing into*, and the backend
//! resolves what that means. A handle in a signature would be a Windows detail
//! leaking into a trait that macOS and Linux also have to satisfy.

use keymap::LayoutId;

/// Backends report failures as text. The engine cannot act differently on a
/// failed injection than on a failed layout switch -- it abandons the
/// correction either way -- so a richer error type would carry information
/// nothing consumes.
pub type Result<T> = std::result::Result<T, String>;

/// Rewrites text in the focused field.
pub trait TextInjector {
    /// Delete `backspaces` characters, then type `replacement`.
    ///
    /// Implementations must send `replacement` as literal Unicode rather than
    /// as key presses, so the text does not depend on which layout happens to
    /// be active while it is being written.
    fn replace_text(&self, backspaces: usize, replacement: &str) -> Result<()>;
}

/// Reads and changes the active keyboard layout.
pub trait LayoutController {
    /// Layouts the user actually has installed.
    ///
    /// This does double duty: we cannot switch to a layout the system does not
    /// have, and restricting candidates to layouts someone genuinely uses
    /// removes a whole class of false corrections for free.
    fn installed_layouts(&self) -> Vec<LayoutId>;

    /// Switch the focused window to `target`.
    fn switch_to(&self, target: LayoutId) -> Result<()>;
}

/// What the backend managed to learn about where the user is typing.
pub trait FocusContext {
    /// Executable name of the foreground application, when it can be read.
    fn active_process_name(&self) -> Option<String>;

    /// Whether the focused field is a password field.
    ///
    /// `Some(false)` **only** when the backend positively established the field
    /// is ordinary text. `None` means it could not tell, and callers must treat
    /// that exactly like `Some(true)`.
    ///
    /// The `Option` is the single most important thing in this file. macOS and
    /// Linux cannot always answer this question; a plain `bool` would make
    /// "unknown" silently mean "safe to type into", and the program would start
    /// rewriting passwords on those platforms with nothing to signal it.
    fn is_password_field(&self) -> Option<bool>;

    /// Whether the foreground application is on the user's block list.
    fn is_blocked(&self) -> bool;
}

/// Watches the keyboard and drives the correction loop.
pub trait KeyHook {
    /// Run until the process exits.
    ///
    /// The implementation must not do the deciding on whatever thread the OS
    /// calls it back on: Windows silently removes a low-level hook whose
    /// callback does not return promptly, leaving a program that is running but
    /// has quietly stopped working.
    fn run(&self) -> Result<()>;
}
