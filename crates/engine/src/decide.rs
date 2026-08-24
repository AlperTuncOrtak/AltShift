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
    /// Normal typing threshold.
    pub margin: f64,
    /// Bonus applied if the candidate layout matches the layout of recent words.
    pub context_bonus: f64,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self { margin: 1.0, context_bonus: 0.4 }
    }
}

/// A rewrite the engine is proposing.
#[derive(Clone, PartialEq, Debug)]
pub struct Correction {
    pub from: String,
    pub to: String,
    pub target_layout: LayoutId,
    pub backspaces: usize,
    pub margin: f64,
}

#[derive(Clone, PartialEq, Debug)]
pub enum Decision {
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

    pub fn reject(&mut self, word: &str) {
        self.guards.add_exception(word);
    }

    fn model(&self, id: LayoutId) -> Option<&LanguageModel> {
        self.models.iter().find(|(l, _)| *l == id).map(|(_, m)| m)
    }

    pub fn decide(
        &self,
        strokes: &[Stroke],
        current: LayoutId,
        available: &[LayoutId],
        ctx: &Context,
        recent: Option<LayoutId>,
    ) -> Decision {
        if strokes.is_empty() {
            return Decision::Leave(None);
        }

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
            if rendered == typed {
                continue;
            }

            let mut margin = model.score(&rendered) - typed_score;
            
            // Apply context bonus if this candidate matches the recent layout context
            if recent == Some(candidate_id) {
                margin += self.thresholds.context_bonus;
            }

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

        match best {
            Some(c) if c.margin > self.thresholds.margin => Decision::Correct(c),
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
        let en = LanguageModel::train(
            "en",
            ["hello", "there", "world", "message", "the", "and", "letter", "sender", "system"]
                .map(String::from),
        );
        let ru = LanguageModel::train(
            "ru",
            ["привет", "как", "дела", "хорошо", "спасибо", "сообщение", "письмо", "система"]
                .map(String::from),
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
        let d = engine().decide(&strokes("ghbdtn"), LayoutId::UsQwerty, &BOTH, &safe(), None);
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
            let d = engine().decide(&strokes(word), LayoutId::UsQwerty, &BOTH, &safe(), None);
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
            e.decide(&strokes("ghbdtn"), LayoutId::UsQwerty, &BOTH, &ctx, None),
            Decision::Leave(Some(Reason::SecureField))
        );
    }

    /// Without a Russian layout installed there is nothing to switch to.
    #[test]
    fn only_installed_layouts_are_candidates() {
        let only_us = [LayoutId::UsQwerty];
        let d = engine().decide(&strokes("ghbdtn"), LayoutId::UsQwerty, &only_us, &safe(), None);
        assert!(matches!(d, Decision::Leave(_)));
    }

    #[test]
    fn a_rejected_word_is_never_corrected_again() {
        let mut e = engine();
        assert!(matches!(
            e.decide(&strokes("ghbdtn"), LayoutId::UsQwerty, &BOTH, &safe(), None),
            Decision::Correct(_)
        ));
        e.reject("ghbdtn");
        assert_eq!(
            e.decide(&strokes("ghbdtn"), LayoutId::UsQwerty, &BOTH, &safe(), None),
            Decision::Leave(Some(Reason::UserException))
        );
    }

    #[test]
    fn the_swap_works_in_both_directions() {
        let cyrillic = keymap::RU_YCUKEN.strokes_for("hello").unwrap_or_default();
        assert!(cyrillic.is_empty(), "sanity: latin text is not typeable on the RU table");

        let strokes = keymap::RU_YCUKEN.strokes_for("руддщ").unwrap();
        match engine().decide(&strokes, LayoutId::RuYcuken, &BOTH, &safe(), None) {
            Decision::Correct(c) => {
                assert_eq!(c.to, "hello");
                assert_eq!(c.target_layout, LayoutId::UsQwerty);
            }
            other => panic!("expected a correction, got {other:?}"),
        }
    }
}
