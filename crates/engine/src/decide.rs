//! Deciding whether a finished word should be rewritten.

use guards::{Context, Guards, Reason, Verdict};
use keymap::{LayoutId, Stroke};
use lang::LanguageModel;

/// How much better the alternative reading must be before we touch anything.
///
/// Expressed in the log-probability units of [`LanguageModel::score`], so a
/// margin of 1.0 means "roughly e times more likely". Correcting is an
/// intrusion; the burden of proof sits on the correction.
#[derive(Copy, Clone, Debug)]
pub struct Thresholds {
    /// Normal typing.
    pub margin: f64,
    /// Extra margin demanded of short words, divided by word length.
    ///
    /// A score is a mean over trigrams, so a four-letter word averages three
    /// samples and a ten-letter word averages nine: the short word's estimate
    /// is simply noisier. Without this, nearly every surviving false positive
    /// was four letters long -- including real words like `verb` and `внук`,
    /// which a flat threshold cannot separate from corpus junk.
    pub short_word_penalty: f64,
    /// The first word after focus moved to a new window.
    ///
    /// Deliberately lower: this is exactly where the user has not yet checked
    /// which layout is active, so a wrong-layout word is far more likely here
    /// than mid-paragraph. We are encoding that prior rather than pretending
    /// every position is equally suspect.
    pub first_word_margin: f64,
}

impl Default for Thresholds {
    fn default() -> Self {
        // Set by sweeping the accuracy harness over held-out OpenSubtitles
        // lists (see tools/accuracy). We take the configuration with the
        // fewest misses that still fits the false-positive budget, since a
        // missed word costs a keystroke and a mangled one costs a user.
        //
        //   penalty  margin   false pos   false neg
        //   0        1.2        0.090%      7.36%
        //   3        0.6        0.080%      7.02%
        //   4        0.3        0.096%      6.39%   <- chosen
        //   5        0.1        0.096%      6.36%
        //   6        0.0        0.080%      6.88%
        //
        // `short_word_penalty` earns its place: with a flat threshold, four
        // letters accounted for nearly every surviving false positive --
        // `verb`, `kerb`, `внук`, `лгут` -- and no flat value separated those
        // from corpus junk. Trading base margin for length sensitivity cut
        // misses from 7.36% to 6.39% at the same budget.
        //
        // `first_word_margin` is NOT measured: the harness has no notion of
        // focus changes. Validate it or delete the field (WUL-16).
        Self { margin: 0.3, first_word_margin: 0.2, short_word_penalty: 4.0 }
    }
}

/// A rewrite the engine is proposing.
#[derive(Clone, PartialEq, Debug)]
pub struct Correction {
    /// What the user sees now.
    pub from: String,
    /// What we would put there instead.
    pub to: String,
    /// The layout to switch to afterwards, so the *next* word is already right.
    pub target_layout: LayoutId,
    /// How many characters the platform layer must delete first.
    pub backspaces: usize,
    /// How decisively the alternative won. Surfaced for the accuracy harness
    /// and for explaining a decision in the settings UI.
    pub margin: f64,
}

#[derive(Clone, PartialEq, Debug)]
pub enum Decision {
    /// Leave the word alone. Carries a reason when a guard was responsible.
    Leave(Option<Reason>),
    Correct(Correction),
}

#[derive(Default)]
pub struct Engine {
    models: Vec<(LayoutId, LanguageModel)>,
    guards: Guards,
    thresholds: Thresholds,
}

impl Engine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_model(mut self, id: LayoutId, model: LanguageModel) -> Self {
        self.models.retain(|(existing, _)| *existing != id);
        self.models.push((id, model));
        self
    }

    pub fn with_thresholds(mut self, thresholds: Thresholds) -> Self {
        self.thresholds = thresholds;
        self
    }

    pub fn guards(&self) -> &Guards {
        &self.guards
    }

    /// Record that the user undid a correction: never offer it again.
    pub fn reject(&mut self, word: &str) {
        self.guards.add_exception(word);
    }

    fn model(&self, id: LayoutId) -> Option<&LanguageModel> {
        self.models.iter().find(|(l, _)| *l == id).map(|(_, m)| m)
    }

    /// Decide what to do with a finished word.
    ///
    /// `available` is the set of layouts the user actually has installed. It is
    /// a parameter rather than a constant because it does double duty: we
    /// cannot switch to a layout the OS does not have, and restricting the
    /// candidate set to layouts the user genuinely uses removes a whole class
    /// of false positives for free.
    pub fn decide(
        &self,
        strokes: &[Stroke],
        current: LayoutId,
        available: &[LayoutId],
        ctx: &Context,
        first_word_after_focus: bool,
    ) -> Decision {
        if strokes.is_empty() {
            return Decision::Leave(None);
        }

        // What the user is looking at right now.
        let Some(typed) = current.layout().render(strokes) else {
            return Decision::Leave(None);
        };

        if let Verdict::Block(reason) = self.guards.check(&typed, ctx) {
            return Decision::Leave(Some(reason));
        }

        let Some(current_model) = self.model(current) else {
            return Decision::Leave(None);
        };
        let typed_score = current_model.score(&typed);

        let mut best: Option<Correction> = None;
        for &candidate_id in available {
            if candidate_id == current {
                continue;
            }
            let Some(model) = self.model(candidate_id) else { continue };
            let Some(rendered) = candidate_id.layout().render(strokes) else { continue };
            // Punctuation-only runs render identically in both layouts.
            if rendered == typed {
                continue;
            }

            let margin = model.score(&rendered) - typed_score;
            if best.as_ref().is_none_or(|b| margin > b.margin) {
                best = Some(Correction {
                    backspaces: typed.chars().count(),
                    from: typed.clone(),
                    to: rendered,
                    target_layout: candidate_id,
                    margin,
                });
            }
        }

        let base = if first_word_after_focus {
            self.thresholds.first_word_margin
        } else {
            self.thresholds.margin
        };
        // Demand a wider margin where the evidence is thinner.
        let required = base + self.thresholds.short_word_penalty / typed.chars().count() as f64;

        match best {
            Some(c) if c.margin > required => Decision::Correct(c),
            _ => Decision::Leave(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use keymap::{LayoutId, US_QWERTY};

    const BOTH: [LayoutId; 2] = [LayoutId::UsQwerty, LayoutId::RuYcuken];

    fn engine() -> Engine {
        // Counts stand in for corpus frequency; the model reads relative
        // magnitude, so only the ordering has to be plausible.
        let en = LanguageModel::train(
            "en",
            [
                ("the", 900_000), ("and", 500_000), ("hello", 90_000), ("there", 80_000),
                ("world", 40_000), ("message", 20_000), ("letter", 15_000),
                ("sender", 4_000), ("system", 3_000),
            ]
            .map(|(w, c)| (w.to_string(), c)),
        );
        let ru = LanguageModel::train(
            "ru",
            [
                ("как", 800_000), ("привет", 200_000), ("дела", 90_000), ("хорошо", 70_000),
                ("спасибо", 60_000), ("сообщение", 20_000), ("письмо", 15_000),
                ("система", 3_000),
            ]
            .map(|(w, c)| (w.to_string(), c)),
        );
        Engine::new().with_model(LayoutId::UsQwerty, en).with_model(LayoutId::RuYcuken, ru)
    }

    /// A context the platform has positively cleared as ordinary text.
    fn safe() -> Context {
        Context { is_password_field: Some(false), ..Context::default() }
    }

    fn strokes(text: &str) -> Vec<Stroke> {
        US_QWERTY.strokes_for(text).unwrap()
    }

    #[test]
    fn wrong_layout_russian_is_corrected() {
        let d = engine().decide(&strokes("ghbdtn"), LayoutId::UsQwerty, &BOTH, &safe(), false);
        match d {
            Decision::Correct(c) => {
                assert_eq!(c.from, "ghbdtn");
                assert_eq!(c.to, "привет");
                assert_eq!(c.target_layout, LayoutId::RuYcuken);
                assert_eq!(c.backspaces, 6);
            }
            other => panic!("expected a correction, got {other:?}"),
        }
    }

    /// The failure that loses users: rewriting text that was already fine.
    #[test]
    fn correct_english_is_left_alone() {
        for word in ["hello", "world", "message", "there"] {
            let d = engine().decide(&strokes(word), LayoutId::UsQwerty, &BOTH, &safe(), false);
            assert!(
                matches!(d, Decision::Leave(_)),
                "{word} was already correct but got {d:?}"
            );
        }
    }

    #[test]
    fn guards_win_over_the_score() {
        let e = engine();
        // Scores as strongly Cyrillic, but it is in a password field.
        let ctx = Context { is_password_field: Some(true), ..Context::default() };
        assert_eq!(
            e.decide(&strokes("ghbdtn"), LayoutId::UsQwerty, &BOTH, &ctx, false),
            Decision::Leave(Some(Reason::SecureField))
        );
    }

    /// Without a Russian layout installed there is nothing to switch to.
    #[test]
    fn only_installed_layouts_are_candidates() {
        let only_us = [LayoutId::UsQwerty];
        let d = engine().decide(&strokes("ghbdtn"), LayoutId::UsQwerty, &only_us, &safe(), false);
        assert!(matches!(d, Decision::Leave(_)));
    }

    #[test]
    fn a_rejected_word_is_never_corrected_again() {
        let mut e = engine();
        assert!(matches!(
            e.decide(&strokes("ghbdtn"), LayoutId::UsQwerty, &BOTH, &safe(), false),
            Decision::Correct(_)
        ));
        e.reject("ghbdtn");
        assert_eq!(
            e.decide(&strokes("ghbdtn"), LayoutId::UsQwerty, &BOTH, &safe(), false),
            Decision::Leave(Some(Reason::UserException))
        );
    }

    #[test]
    fn the_swap_works_in_both_directions() {
        let cyrillic = keymap::RU_YCUKEN.strokes_for("hello").unwrap_or_default();
        assert!(cyrillic.is_empty(), "sanity: latin text is not typeable on the RU table");

        let strokes = keymap::RU_YCUKEN.strokes_for("руддщ").unwrap();
        match engine().decide(&strokes, LayoutId::RuYcuken, &BOTH, &safe(), false) {
            Decision::Correct(c) => {
                assert_eq!(c.to, "hello");
                assert_eq!(c.target_layout, LayoutId::UsQwerty);
            }
            other => panic!("expected a correction, got {other:?}"),
        }
    }
}
