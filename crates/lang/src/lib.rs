//! Scoring a candidate word against a language.
//!
//! Two signals, combined:
//!
//! 1. **Dictionary membership.** Dominant when it fires: a real word is a real
//!    word. But a dictionary alone is useless here, because the *wrong* reading
//!    of a stroke run is almost never in either dictionary — both candidates
//!    miss and we learn nothing.
//! 2. **Character trigram probability.** This is what actually separates the
//!    two readings. `ghbdtn` is not merely absent from English; six consonants
//!    in a row make it *impossible* in English. Its Cyrillic reading `привет`
//!    is ordinary Russian. That gap is the whole decision.

use std::collections::{HashMap, HashSet};

/// Word boundary markers. Chosen from a range no natural alphabet occupies, so
/// they cannot collide with corpus text.
const START: char = '\u{2}';
const END: char = '\u{3}';

/// Add-k smoothing constant. Keeps an unseen-but-plausible trigram from
/// scoring as negative infinity while still punishing it heavily.
const SMOOTHING: f64 = 0.5;

/// Log-probability bonus for a word present in the dictionary.
///
/// Large enough that a real word beats a merely-plausible one, small enough
/// that an out-of-vocabulary word which is obviously native (an inflection, a
/// name, a loanword) can still win on trigrams alone.
const DICT_BONUS: f64 = 5.0;

pub struct LanguageModel {
    pub name: String,
    words: HashSet<String>,
    trigrams: HashMap<(char, char, char), u32>,
    bigrams: HashMap<(char, char), u32>,
    vocab: usize,
}

impl LanguageModel {
    /// Build a model from a word list.
    ///
    /// Words are lowercased; anything containing a digit or separator is
    /// dropped, since such entries teach the model nothing about the shape of
    /// the language.
    pub fn train(name: impl Into<String>, source: impl IntoIterator<Item = String>) -> Self {
        let mut words = HashSet::new();
        let mut trigrams: HashMap<(char, char, char), u32> = HashMap::new();
        let mut bigrams: HashMap<(char, char), u32> = HashMap::new();
        let mut alphabet = HashSet::new();

        for raw in source {
            let word = raw.trim().to_lowercase();
            if word.len() < 2 || !word.chars().all(char::is_alphabetic) {
                continue;
            }

            let padded: Vec<char> = std::iter::once(START)
                .chain(word.chars())
                .chain([END])
                .collect();
            for w in padded.windows(3) {
                *trigrams.entry((w[0], w[1], w[2])).or_insert(0) += 1;
                *bigrams.entry((w[0], w[1])).or_insert(0) += 1;
            }
            alphabet.extend(word.chars());
            words.insert(word);
        }

        Self {
            name: name.into(),
            words,
            trigrams,
            bigrams,
            vocab: alphabet.len().max(1),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }

    pub fn word_count(&self) -> usize {
        self.words.len()
    }

    pub fn contains(&self, word: &str) -> bool {
        self.words.contains(&word.to_lowercase())
    }

    /// How much this language likes `word`, in log space.
    ///
    /// Mean-per-trigram rather than a sum, so the number means the same thing
    /// regardless of word length and a single threshold can be reasoned about.
    pub fn score(&self, word: &str) -> f64 {
        let lower = word.to_lowercase();
        if lower.is_empty() {
            return f64::NEG_INFINITY;
        }

        let padded: Vec<char> = std::iter::once(START)
            .chain(lower.chars())
            .chain([END])
            .collect();
        let mut total = 0.0;
        let mut n = 0usize;

        for w in padded.windows(3) {
            let tri = *self.trigrams.get(&(w[0], w[1], w[2])).unwrap_or(&0) as f64;
            let bi = *self.bigrams.get(&(w[0], w[1])).unwrap_or(&0) as f64;
            // P(w2 | w0 w1), smoothed.
            total += ((tri + SMOOTHING) / (bi + SMOOTHING * self.vocab as f64)).ln();
            n += 1;
        }

        let mean = if n == 0 {
            f64::NEG_INFINITY
        } else {
            total / n as f64
        };
        if self.words.contains(&lower) {
            mean + DICT_BONUS
        } else {
            mean
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn english() -> LanguageModel {
        LanguageModel::train(
            "en",
            [
                "hello", "there", "the", "and", "world", "message", "letter", "sender", "hell",
            ]
            .map(String::from),
        )
    }

    fn russian() -> LanguageModel {
        LanguageModel::train(
            "ru",
            [
                "привет",
                "как",
                "дела",
                "хорошо",
                "спасибо",
                "привычка",
                "приветствие",
            ]
            .map(String::from),
        )
    }

    #[test]
    fn a_known_word_beats_an_unknown_one() {
        let en = english();
        assert!(en.score("hello") > en.score("hellp"));
    }

    /// The load-bearing property: each language must reject the other's script
    /// decisively, even for words neither dictionary contains.
    #[test]
    fn each_language_rejects_the_other_script() {
        let (en, ru) = (english(), russian());
        assert!(ru.score("привет") > en.score("привет"));
        assert!(en.score("hello") > ru.score("hello"));
    }

    /// The real decision, made without either candidate being in a dictionary.
    #[test]
    fn gibberish_loses_to_its_other_reading() {
        let (en, ru) = (english(), russian());
        assert!(
            ru.score("привет") > en.score("ghbdtn"),
            "the Cyrillic reading must win outright"
        );
    }

    #[test]
    fn plausible_shapes_outscore_impossible_ones() {
        let en = english();
        // Neither is a word; one could be English, the other could not.
        assert!(en.score("hellon") > en.score("ghbdtn"));
    }
}
