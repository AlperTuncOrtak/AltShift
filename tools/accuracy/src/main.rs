//! Measures the correction engine against word lists, with no keyboard hook and
//! no OS permissions.
//!
//! Two error types matter, and they are not symmetric:
//!
//! * A **false negative** is a wrong-layout word we failed to fix. The user
//!   fixes it by hand, exactly as they did before installing anything. Mildly
//!   disappointing.
//! * A **false positive** is a correct word we mangled. The user watches the
//!   program corrupt their writing, and uninstalls it.
//!
//! So the false-positive rate is the number that decides whether this program
//! is usable, and it is budgeted an order of magnitude tighter.

use engine::{Decision, Engine, Thresholds};
use guards::Context;
use keymap::{LayoutId, Script, RU_YCUKEN, US_QWERTY};
use lang::LanguageModel;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;

/// Ceiling for mangled-correct-word rate. Above this, the program is a liability.
const FALSE_POSITIVE_BUDGET: f64 = 0.001; // 0.1%

const AVAILABLE: [LayoutId; 2] = [LayoutId::UsQwerty, LayoutId::RuYcuken];

fn main() {
    let en = match load("data/en.txt", Script::Latin) {
        Ok(w) => w,
        Err(e) => return eprintln!("data/en.txt: {e}\nRun ./fetch-wordlists.sh first."),
    };
    let ru = match load("data/ru.txt", Script::Cyrillic) {
        Ok(w) => w,
        Err(e) => return eprintln!("data/ru.txt: {e}\nRun ./fetch-wordlists.sh first."),
    };

    // Held-out split. Training and testing on the same words would let the
    // dictionary bonus answer every question and report a perfect score that
    // says nothing about unseen text.
    let (en_train, en_test) = split(&en);
    let (ru_train, ru_test) = split(&ru);

    println!("English : {} train / {} test", en_train.len(), en_test.len());
    println!("Russian : {} train / {} test", ru_train.len(), ru_test.len());

    let e = Engine::new()
        .with_model(LayoutId::UsQwerty, LanguageModel::train("en", en_train))
        .with_model(LayoutId::RuYcuken, LanguageModel::train("ru", ru_train))
        .with_thresholds(Thresholds {
            // Overridable so a shell loop can sweep the threshold without a
            // rebuild; the sweep is how these numbers get chosen (WUL-16).
            margin: std::env::var("ALTSHIFT_MARGIN")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(Thresholds::default().margin),
            ..Thresholds::default()
        });

    println!("\n{:-<64}", "");

    // --- False positives: correctly typed words that must be left alone -----
    let en_fp = run(&e, &en_test, LayoutId::UsQwerty, LayoutId::UsQwerty, Expect::Leave);
    let ru_fp = run(&e, &ru_test, LayoutId::RuYcuken, LayoutId::RuYcuken, Expect::Leave);

    // --- False negatives: wrong-layout words that should be rescued --------
    let ru_fn = run(&e, &ru_test, LayoutId::RuYcuken, LayoutId::UsQwerty, Expect::Correct);
    let en_fn = run(&e, &en_test, LayoutId::UsQwerty, LayoutId::RuYcuken, Expect::Correct);

    report("FALSE POSITIVE  English typed on US ", &en_fp);
    report("FALSE POSITIVE  Russian typed on RU ", &ru_fp);
    report("FALSE NEGATIVE  Russian typed on US ", &ru_fn);
    report("FALSE NEGATIVE  English typed on RU ", &en_fn);

    let fp_rate = (en_fp.wrong + ru_fp.wrong) as f64 / (en_fp.total + ru_fp.total).max(1) as f64;
    let fn_rate = (en_fn.wrong + ru_fn.wrong) as f64 / (en_fn.total + ru_fn.total).max(1) as f64;

    println!("\n{:-<64}", "");
    println!("false positive rate  {:>7.3}%   (budget {:.3}%)", fp_rate * 100.0, FALSE_POSITIVE_BUDGET * 100.0);
    println!("false negative rate  {:>7.3}%", fn_rate * 100.0);

    if fp_rate <= FALSE_POSITIVE_BUDGET {
        println!("\nPASS - within the false-positive budget.");
    } else {
        println!("\nFAIL - mangles too much correct text. Raise Thresholds::margin.");
        std::process::exit(1);
    }
}

#[derive(Copy, Clone, PartialEq)]
enum Expect {
    Leave,
    Correct,
}

#[derive(Default)]
struct Outcome {
    total: usize,
    wrong: usize,
    /// A few concrete failures beat any summary statistic when tuning.
    samples: Vec<String>,
}

/// Type `words` (which are valid in `source`) while `active` is the live layout,
/// and check the engine does what `expect` says.
fn run(e: &Engine, words: &[String], source: LayoutId, active: LayoutId, expect: Expect) -> Outcome {
    let ctx = Context { is_password_field: Some(false), ..Context::default() };
    let source_layout = source.layout();
    let mut out = Outcome::default();

    for word in words {
        // The keys a user presses to produce this word on its own layout. Those
        // same keys are what reach us when the wrong layout is active.
        let Some(strokes) = source_layout.strokes_for(word) else { continue };

        let decision = e.decide(&strokes, active, &AVAILABLE, &ctx, false);
        out.total += 1;

        let ok = match (expect, &decision) {
            (Expect::Leave, Decision::Leave(_)) => true,
            (Expect::Correct, Decision::Correct(c)) => c.to == *word,
            _ => false,
        };
        if !ok {
            out.wrong += 1;
            if out.samples.len() < 8 {
                out.samples.push(match &decision {
                    Decision::Correct(c) => format!("{} -> {}", c.from, c.to),
                    Decision::Leave(r) => format!("{word} left alone ({r:?})"),
                });
            }
        }
    }
    out
}

fn report(label: &str, o: &Outcome) {
    let rate = o.wrong as f64 / o.total.max(1) as f64 * 100.0;
    println!("{label} {:>6}/{:<6} {:>7.3}%", o.wrong, o.total, rate);
    for s in &o.samples {
        println!("      {s}");
    }
}

fn load(path: impl AsRef<Path>, script: Script) -> std::io::Result<Vec<String>> {
    // Keep only words written wholly in the target script. Subtitle corpora
    // leak the other alphabet freely (brand names, song titles), and such
    // entries would teach each model to accept the very script it exists to
    // reject.
    //
    // This filter lives here rather than in the fetch script because BSD grep
    // matches a Cyrillic character class byte-wise: `[а-яё]{3,}` counts bytes,
    // not characters, and quietly admits two-letter words.
    let in_script = |c: char| match script {
        Script::Latin => c.is_ascii_lowercase(),
        Script::Cyrillic => matches!(c, 'а'..='я' | 'ё'),
    };
    Ok(std::fs::read_to_string(path)?
        .lines()
        .map(|l| l.trim().to_lowercase())
        .filter(|l| l.chars().count() >= 3 && l.chars().all(in_script))
        .collect())
}

/// Deterministic 80/20 split, hashed on the word so the same word always lands
/// on the same side across runs and threshold changes stay comparable.
fn split(words: &[String]) -> (Vec<String>, Vec<String>) {
    words.iter().cloned().partition(|w| {
        let mut h = DefaultHasher::new();
        w.hash(&mut h);
        h.finish() % 5 != 0
    })
}

/// Keeps the layout statics referenced from this binary honest.
#[allow(dead_code)]
fn _layouts() -> [&'static str; 2] {
    [US_QWERTY.name, RU_YCUKEN.name]
}
