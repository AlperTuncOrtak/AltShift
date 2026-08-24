//! Everything the correction engine is forbidden to touch.
//!
//! This module exists as its own crate so that the answer to "what does this
//! keylogger-shaped program leave alone?" is one auditable file rather than a
//! behaviour scattered across the engine.
//!
//! The governing rule is **fail closed**: every uncertainty resolves to
//! [`Verdict::Block`]. Declining to correct a word costs the user one
//! keystroke; corrupting a password costs them an account.

use std::collections::HashSet;

/// Why a token was left alone. Carried rather than discarded so the settings
/// UI can explain a non-correction instead of looking broken.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Reason {
    /// The focused field is a password field, or we could not prove it is not.
    SecureField,
    /// The foreground application is on the block list.
    BlockedApp,
    /// The user rejected this correction before.
    UserException,
    /// Too short to carry a reliable signal.
    TooShort,
    /// Long enough to be a secret rather than a word.
    TooLong,
    /// Contains `@` — email address or handle.
    Address,
    /// Contains a path or URL separator.
    UrlOrPath,
    /// Contains a digit — identifier, version, or credential.
    HasDigit,
    /// Contains `_` — code identifier.
    Identifier,
    /// Internal capital letters — camelCase or a random secret.
    InternalCapital,
    /// Capitalised mid-sentence — most likely a name.
    ProperNoun,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Verdict {
    Allow,
    Block(Reason),
}

impl Verdict {
    pub fn is_allowed(self) -> bool {
        matches!(self, Verdict::Allow)
    }
}

/// What the platform layer managed to learn about where the user is typing.
#[derive(Copy, Clone, Debug)]
pub struct Context {
    /// `Some(false)` only when the platform positively determined the focused
    /// field is *not* a password field. `None` means it could not tell — which
    /// is treated exactly like `Some(true)`.
    ///
    /// The `Option` is in the signature on purpose: macOS and Linux cannot
    /// always answer this, and a `bool` would have quietly defaulted them to
    /// "safe to type into".
    pub is_password_field: Option<bool>,
    /// Foreground app is on the user's block list.
    pub app_blocked: bool,
    /// This token begins a sentence, so a leading capital is unremarkable.
    pub sentence_initial: bool,
}

impl Default for Context {
    /// The safe context: assume a password field until proven otherwise.
    fn default() -> Self {
        Self {
            is_password_field: None,
            app_blocked: false,
            sentence_initial: false,
        }
    }
}

pub struct Guards {
    /// Words the user has rejected. This is the only user-typed text the
    /// program ever writes to disk, and the settings UI must be able to show
    /// and clear it.
    exceptions: HashSet<String>,
    min_len: usize,
    max_len: usize,
}

impl Default for Guards {
    fn default() -> Self {
        Self {
            exceptions: HashSet::new(),
            // Under three characters there is not enough signal to beat the
            // false-positive budget, whichever way the word is rendered.
            min_len: 4,
            // Ordinary Russian and English words are comfortably under this.
            // Past it, a token is far more likely to be a token.
            max_len: 24,
        }
    }
}

impl Guards {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a correction the user undid.
    pub fn add_exception(&mut self, word: &str) {
        self.exceptions.insert(word.to_lowercase());
    }

    pub fn exceptions(&self) -> impl Iterator<Item = &str> {
        self.exceptions.iter().map(String::as_str)
    }

    pub fn clear_exceptions(&mut self) {
        self.exceptions.clear();
    }

    /// Decide whether `token`, as the user typed it, may be corrected at all.
    ///
    /// Ordered cheapest-and-most-serious first, so the common rejection paths
    /// cost almost nothing on the typing hot path.
    pub fn check(&self, token: &str, ctx: &Context) -> Verdict {
        if ctx.app_blocked {
            return Verdict::Block(Reason::BlockedApp);
        }
        // Anything other than a positive "not a password" blocks.
        if ctx.is_password_field != Some(false) {
            return Verdict::Block(Reason::SecureField);
        }
        if self.exceptions.contains(&token.to_lowercase()) {
            return Verdict::Block(Reason::UserException);
        }

        let len = token.chars().count();
        if len < self.min_len {
            return Verdict::Block(Reason::TooShort);
        }
        if len > self.max_len {
            return Verdict::Block(Reason::TooLong);
        }

        if token.contains('@') {
            return Verdict::Block(Reason::Address);
        }
        if token.contains('/') || token.contains('\\') || token.contains(':') {
            return Verdict::Block(Reason::UrlOrPath);
        }
        if token.chars().any(|c| c.is_ascii_digit()) {
            return Verdict::Block(Reason::HasDigit);
        }
        if token.contains('_') {
            return Verdict::Block(Reason::Identifier);
        }

        self.check_capitalisation(token, ctx)
    }

    /// Capitalisation carries most of our "this belongs to the user" signal:
    /// names, camelCase identifiers and random secrets all show up here.
    fn check_capitalisation(&self, token: &str, ctx: &Context) -> Verdict {
        let mut chars = token.chars();
        let Some(first) = chars.next() else {
            return Verdict::Block(Reason::TooShort);
        };

        // All-caps is shouting, not a secret. Leave it correctable.
        if token
            .chars()
            .filter(|c| c.is_alphabetic())
            .all(char::is_uppercase)
        {
            return Verdict::Allow;
        }
        // A capital anywhere but the front means camelCase or a generated
        // token. Never a word we should be second-guessing.
        if chars.any(char::is_uppercase) {
            return Verdict::Block(Reason::InternalCapital);
        }
        // A leading capital away from a sentence boundary reads as a name.
        // The user's own surname is exactly the word we must never "fix".
        if first.is_uppercase() && !ctx.sentence_initial {
            return Verdict::Block(Reason::ProperNoun);
        }
        Verdict::Allow
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A context where the platform proved the field is ordinary text.
    fn safe() -> Context {
        Context {
            is_password_field: Some(false),
            ..Context::default()
        }
    }

    #[test]
    fn an_ordinary_word_is_allowed() {
        assert_eq!(Guards::new().check("ghbdtn", &safe()), Verdict::Allow);
    }

    #[test]
    fn a_password_field_blocks_everything() {
        let ctx = Context {
            is_password_field: Some(true),
            ..Context::default()
        };
        assert_eq!(
            Guards::new().check("ghbdtn", &ctx),
            Verdict::Block(Reason::SecureField)
        );
    }

    /// The whole reason `is_password_field` is an `Option`.
    #[test]
    fn an_undetermined_field_blocks_too() {
        let ctx = Context {
            is_password_field: None,
            ..Context::default()
        };
        assert_eq!(
            Guards::new().check("ghbdtn", &ctx),
            Verdict::Block(Reason::SecureField)
        );
    }

    #[test]
    fn personal_data_is_left_alone() {
        let g = Guards::new();
        let ctx = safe();
        for (token, reason) in [
            ("alper@gmail.com", Reason::Address),
            ("https://example.com", Reason::UrlOrPath),
            ("C:\\Users\\alpertunc", Reason::UrlOrPath),
            ("sk-proj-a1b2c3", Reason::HasDigit),
            ("api_key", Reason::Identifier),
            ("hunter2", Reason::HasDigit),
            ("xK9mQ2vLp8Wz", Reason::HasDigit),
            ("getUserName", Reason::InternalCapital),
        ] {
            assert_eq!(g.check(token, &ctx), Verdict::Block(reason), "{token}");
        }
    }

    #[test]
    fn a_name_mid_sentence_is_left_alone() {
        let g = Guards::new();
        assert_eq!(
            g.check("Cagri", &safe()),
            Verdict::Block(Reason::ProperNoun)
        );
        // ...but the same word opening a sentence is fair game.
        let ctx = Context {
            sentence_initial: true,
            ..safe()
        };
        assert_eq!(g.check("Cagri", &ctx), Verdict::Allow);
    }

    #[test]
    fn shouting_is_still_correctable() {
        assert_eq!(Guards::new().check("GHBDTN", &safe()), Verdict::Allow);
    }

    #[test]
    fn a_long_secret_is_left_alone() {
        let g = Guards::new();
        assert_eq!(
            g.check("correcthorsebatterystaple", &safe()),
            Verdict::Block(Reason::TooLong)
        );
    }

    #[test]
    fn short_words_carry_too_little_signal() {
        assert_eq!(
            Guards::new().check("ab", &safe()),
            Verdict::Block(Reason::TooShort)
        );
    }

    #[test]
    fn a_rejected_word_is_never_offered_again() {
        let mut g = Guards::new();
        assert!(g.check("ghbdtn", &safe()).is_allowed());
        g.add_exception("ghbdtn");
        assert_eq!(
            g.check("GhBdTn".to_lowercase().as_str(), &safe()),
            Verdict::Block(Reason::UserException)
        );
    }
}
