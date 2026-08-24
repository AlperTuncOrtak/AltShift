//! The keystroke buffer: the only place user text lives, and the shortest-lived
//! one we can get away with.

use keymap::Stroke;

/// Why the buffer was discarded.
///
/// Each variant is a moment where we can no longer be sure the strokes we
/// remember still correspond to the text in front of the caret. Correcting on
/// stale strokes would delete characters the user did not type.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Break {
    /// Space or punctuation: the word is finished. The only break that asks for
    /// a decision first.
    WordEnd,
    /// Focus moved to another window or field.
    FocusChange,
    /// The caret was moved by mouse or arrow keys.
    CaretMoved,
    /// A password field became active.
    SecureField,
    /// The user idled long enough that we should not assume continuity.
    Timeout,
    /// The user changed layout by hand; earlier strokes were meant for the
    /// previous one.
    LayoutChanged,
    /// A correction was just applied or undone.
    Applied,
}

/// Fixed cap on remembered keystrokes.
///
/// This is a privacy boundary, not a performance one. The program is incapable
/// of holding a sentence, so there is no buffer for anyone to recover.
pub const MAX_STROKES: usize = 32;

#[derive(Default)]
pub struct Buffer {
    strokes: Vec<Stroke>,
    /// Set when the buffer overflows. An over-long run is dropped rather than
    /// truncated: half a password is still a password fragment.
    poisoned: bool,
}

impl Buffer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, stroke: Stroke) {
        if self.poisoned {
            return;
        }
        if self.strokes.len() >= MAX_STROKES {
            self.clear(Break::Timeout);
            self.poisoned = true;
            return;
        }
        self.strokes.push(stroke);
    }

    /// Remove the most recent stroke, mirroring a Backspace the user typed.
    pub fn pop(&mut self) {
        self.strokes.pop();
    }

    /// Forget everything. `_why` is taken to make every call site state its
    /// reason, so a future reader can audit when text is retained.
    pub fn clear(&mut self, _why: Break) {
        self.strokes.clear();
        self.poisoned = false;
    }

    pub fn strokes(&self) -> &[Stroke] {
        if self.poisoned {
            &[]
        } else {
            &self.strokes
        }
    }

    pub fn len(&self) -> usize {
        self.strokes().len()
    }

    pub fn is_empty(&self) -> bool {
        self.strokes().is_empty()
    }
}

impl Drop for Buffer {
    /// Overwrite on the way out rather than leaving keystrokes in freed memory.
    fn drop(&mut self) {
        self.strokes.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use keymap::Key;

    fn s(key: Key) -> Stroke {
        Stroke::new(key, false)
    }

    #[test]
    fn strokes_accumulate_and_clear() {
        let mut b = Buffer::new();
        b.push(s(Key::KeyG));
        b.push(s(Key::KeyH));
        assert_eq!(b.len(), 2);
        b.clear(Break::WordEnd);
        assert!(b.is_empty());
    }

    #[test]
    fn backspace_shortens_the_buffer() {
        let mut b = Buffer::new();
        b.push(s(Key::KeyG));
        b.push(s(Key::KeyH));
        b.pop();
        assert_eq!(b.len(), 1);
    }

    /// An over-long run must vanish entirely, not survive as a fragment.
    #[test]
    fn overflow_discards_rather_than_truncates() {
        let mut b = Buffer::new();
        for _ in 0..MAX_STROKES + 5 {
            b.push(s(Key::KeyA));
        }
        assert!(b.is_empty(), "an overflowing run must leave nothing behind");
    }

    #[test]
    fn a_poisoned_buffer_stays_empty_until_cleared() {
        let mut b = Buffer::new();
        for _ in 0..MAX_STROKES + 1 {
            b.push(s(Key::KeyA));
        }
        b.push(s(Key::KeyB));
        assert!(b.is_empty());
        b.clear(Break::WordEnd);
        b.push(s(Key::KeyB));
        assert_eq!(b.len(), 1);
    }
}
