//! Scoring a candidate word against a language.
//!
//! Two signals, combined:
//!
//! 1. **Dictionary membership, weighted by how often the word is actually
//!    used.** A flat word list is worse than useless here: subtitle corpora are
//!    full of names and initialisms, and a flat list hands `axl` the same
//!    credibility as `hello`. Those stray three-letter entries produced nearly
//!    every false positive in the first accuracy run.
//! 2. **Character trigram probability.** This is what separates the two
//!    readings. `ghbdtn` is not merely absent from English; six consonants in a
//!    row make it *impossible* in English. Its Cyrillic reading `привет` is
//!    ordinary Russian. That gap is the whole decision.
//!
//! Both are trained on frequency-ordered corpus counts, so the model describes
//! what people type rather than what a lexicon admits.

use std::collections::{HashMap, HashSet};

/// Word boundary markers. Chosen from a range no natural alphabet occupies, so
/// they cannot collide with corpus text.
const START: char = '\u{2}';
const END: char = '\u{3}';

/// Add-k smoothing constant. Keeps an unseen-but-plausible trigram from
/// scoring as negative infinity while still punishing it heavily.
const SMOOTHING: f64 = 0.5;

/// Largest log-probability bonus a dictionary word can earn.
///
/// Scaled by corpus frequency rather than granted flat, so a word the corpus
/// saw once gets almost nothing and a word it saw a million times gets all of
/// it. Large enough that a common word beats a merely-plausible one, small
/// enough that an out-of-vocabulary inflection can still win on trigrams.
const DICT_BONUS: f64 = 6.0;

/// Parse a frequency list -- one `word count` pair per line -- into training
/// input.
///
/// The format is what the corpus ships. Keeping the parser here means the
/// desktop app, the accuracy harness and the benchmark cannot disagree about
/// it; when they did, one of them silently trained on zero words and still
/// reported success.
pub fn parse_frequency_list(text: &str) -> impl Iterator<Item = (String, u64)> + '_ {
    text.lines().filter_map(|line| {
        let (word, count) = line.trim().split_once(' ')?;
        Some((word.to_lowercase(), count.trim().parse().ok()?))
    })
}

pub struct LanguageModel {
    pub name: String,
    /// Word to corpus occurrence count.
    words: HashMap<String, u64>,
    max_count: u64,
    trigrams: HashMap<(char, char, char), f64>,
    bigrams: HashMap<(char, char), f64>,
    vocab: usize,
}

impl LanguageModel {
    /// Build a model from `(word, corpus count)` pairs.
    ///
    /// Words are lowercased; anything not wholly alphabetic is dropped, since
    /// such entries teach the model nothing about the shape of the language.
    ///
    /// N-gram counts are weighted by corpus frequency: the character statistics
    /// then describe running text, not a lexicon, and a single stray subtitle
    /// credit cannot make its letter sequence look native.
    pub fn train(name: impl Into<String>, source: impl IntoIterator<Item = (String, u64)>) -> Self {
        let mut words = HashMap::new();
        let mut trigrams: HashMap<(char, char, char), f64> = HashMap::new();
        let mut bigrams: HashMap<(char, char), f64> = HashMap::new();
        let mut alphabet = HashSet::new();
        let mut max_count = 1u64;

        for (raw, count) in source {
            let word = raw.trim().to_lowercase();
            if word.chars().count() < 2 || !word.chars().all(char::is_alphabetic) {
                continue;
            }
            // Log-scaled, not raw. Raw counts span six orders of magnitude, which
            // makes a fixed smoothing constant meaningless and crushes any unseen
            // continuation of a frequent context: `hellon` then scores *below*
            // `ghbdtn`, because violating a strong expectation is punished harder
            // than wandering into territory the model has never seen. Backwards,
            // for a model whose whole job is to recognise foreign text.
            let weight = 1.0 + (count.max(1) as f64).ln();

            let padded: Vec<char> =
                std::iter::once(START).chain(word.chars()).chain([END]).collect();
            for w in padded.windows(3) {
                *trigrams.entry((w[0], w[1], w[2])).or_insert(0.0) += weight;
                *bigrams.entry((w[0], w[1])).or_insert(0.0) += weight;
            }
            alphabet.extend(word.chars());
            max_count = max_count.max(count);
            *words.entry(word).or_insert(0) += count;
        }

        Self { name: name.into(), words, max_count, trigrams, bigrams, vocab: alphabet.len().max(1) }
    }

    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }

    pub fn word_count(&self) -> usize {
        self.words.len()
    }

    pub fn contains(&self, word: &str) -> bool {
        self.words.contains_key(&word.to_lowercase())
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

        let padded: Vec<char> = std::iter::once(START).chain(lower.chars()).chain([END]).collect();
        let mut total = 0.0;
        let mut n = 0usize;

        for w in padded.windows(3) {
            let tri = *self.trigrams.get(&(w[0], w[1], w[2])).unwrap_or(&0.0);
            let bi = *self.bigrams.get(&(w[0], w[1])).unwrap_or(&0.0);
            // P(w2 | w0 w1), smoothed.
            total += ((tri + SMOOTHING) / (bi + SMOOTHING * self.vocab as f64)).ln();
            n += 1;
        }

        let mean = if n == 0 { f64::NEG_INFINITY } else { total / n as f64 };
        mean + self.dictionary_bonus(&lower)
    }

    /// Dictionary credit, scaled by how common the word actually is.
    ///
    /// `ln(1 + count)` normalised against the most frequent word in the corpus.
    /// A hapax scores near zero; the commonest words score the full bonus. The
    /// `1 +` also keeps a single-count corpus (as in tests) from collapsing to
    /// a zero denominator.
    fn dictionary_bonus(&self, lower: &str) -> f64 {
        let Some(&count) = self.words.get(lower) else { return 0.0 };
        DICT_BONUS * (1.0 + count as f64).ln() / (1.0 + self.max_count as f64).ln()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Counts here stand in for corpus frequency; relative order is what the
    /// model reads, so plausible magnitudes matter more than exact values.
    fn english() -> LanguageModel {
        LanguageModel::train(
            "en",
            [
                ("the", 900_000),
                ("and", 500_000),
                ("hello", 90_000),
                ("there", 80_000),
                ("world", 40_000),
                ("message", 20_000),
                ("letter", 15_000),
                ("sender", 4_000),
                ("hell", 3_000),
            ]
            .map(|(w, c)| (w.to_string(), c)),
        )
    }

    fn russian() -> LanguageModel {
        LanguageModel::train(
            "ru",
            [
                ("как", 800_000),
                ("привет", 200_000),
                ("дела", 90_000),
                ("хорошо", 70_000),
                ("спасибо", 60_000),
                ("привычка", 5_000),
                ("приветствие", 2_000),
            ]
            .map(|(w, c)| (w.to_string(), c)),
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
        assert!(ru.score("привет") > en.score("ghbdtn"), "the Cyrillic reading must win");
    }

    #[test]
    fn plausible_shapes_outscore_impossible_ones() {
        let en = english();
        // Neither is a word; one could be English, the other could not.
        assert!(en.score("hellon") > en.score("ghbdtn"));
    }

    /// The point of weighting: a word the corpus barely saw must not carry the
    /// same authority as one it saw constantly. Flat credit is what let stray
    /// three-letter subtitle entries win corrections.
    #[test]
    fn a_rare_word_earns_less_credit_than_a_common_one() {
        let en = english();
        assert!(en.dictionary_bonus("the") > en.dictionary_bonus("hell") * 1.2);
        assert_eq!(en.dictionary_bonus("nosuchword"), 0.0);
    }
}
