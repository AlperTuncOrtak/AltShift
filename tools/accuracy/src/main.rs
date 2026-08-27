//! Measures the correction engine against word lists, with no keyboard hook and
//! no OS permissions.
//!
//! The simulation replays *runs* of words rather than isolated ones, and lets a
//! correction switch the layout the way the real program does. That matters:
//! someone who starts typing Russian in a US layout gets one mangled word, and
//! then the program has switched and the rest of the sentence arrives clean.
//! Scoring every word as though the layout were never fixed measures a
//! situation that does not happen.
//!
//! So the two questions are:
//!
//! * **How fast does it recover?** How many words does a user lose before the
//!   layout is right. One is the best case; never is the failure.
//! * **How often does it break working text?** A correction applied while the
//!   layout was already correct. This is the number that decides whether the
//!   program is usable: a word we fail to fix costs a keystroke, a word we
//!   wrongly "fix" makes someone watch software corrupt their writing.

use engine::{Decision, Engine, Thresholds};
use guards::Context;
use keymap::{LayoutId, Script};
use lang::LanguageModel;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;

/// Ceiling for the mangled-correct-word rate. Above this, the program is a
/// liability regardless of how well it recovers.
const FALSE_POSITIVE_BUDGET: f64 = 0.001; // 0.1%

/// Words per simulated run. Roughly a sentence: long enough for context to
/// accumulate, short enough that plenty of runs start cold.
const RUN_LEN: usize = 8;

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
    println!("English {:>6} train {:>6} test", en_train.len(), en_test.len());
    println!("Russian {:>6} train {:>6} test", ru_train.len(), ru_test.len());

    let t = Thresholds {
        margin: env("ALTSHIFT_MARGIN", Thresholds::default().margin),
        short_word_penalty: env("ALTSHIFT_SHORT", Thresholds::default().short_word_penalty),
        stickiness: env("ALTSHIFT_STICKY", Thresholds::default().stickiness),
    };
    let e = Engine::new()
        .with_model(LayoutId::UsQwerty, LanguageModel::train("en", en_train))
        .with_model(LayoutId::RuYcuken, LanguageModel::train("ru", ru_train))
        .with_thresholds(t);

    // Typing in the layout you meant to use: nothing should ever be touched.
    let en_ok = simulate(&e, &en_test, LayoutId::UsQwerty, LayoutId::UsQwerty);
    let ru_ok = simulate(&e, &ru_test, LayoutId::RuYcuken, LayoutId::RuYcuken);
    // Starting in the wrong layout: recover, then leave the rest alone.
    let en_bad = simulate(&e, &en_test, LayoutId::UsQwerty, LayoutId::RuYcuken);
    let ru_bad = simulate(&e, &ru_test, LayoutId::RuYcuken, LayoutId::UsQwerty);

    println!("\n{:-<70}", "");
    println!("{:<34}{:>10}{:>12}{:>12}", "", "runs", "recovered", "mangled");
    for (label, o) in [
        ("already correct  English / US", &en_ok),
        ("already correct  Russian / RU", &ru_ok),
        ("wrong layout     English on RU", &en_bad),
        ("wrong layout     Russian on US", &ru_bad),
    ] {
        println!(
            "{label:<34}{:>10}{:>11.1}%{:>11}",
            o.runs,
            o.recovered as f64 / o.runs.max(1) as f64 * 100.0,
            o.mangled
        );
    }

    let mangled: usize = [&en_ok, &ru_ok, &en_bad, &ru_bad].iter().map(|o| o.mangled).sum();
    let safe_words: usize = [&en_ok, &ru_ok, &en_bad, &ru_bad].iter().map(|o| o.safe_words).sum();
    let bad_runs = en_bad.runs + ru_bad.runs;
    let recovered = en_bad.recovered + ru_bad.recovered;
    let lost: usize = en_bad.words_lost + ru_bad.words_lost;

    let fp_rate = mangled as f64 / safe_words.max(1) as f64;

    println!("\n{:-<70}", "");
    println!("recovered from a wrong layout   {:>7.2}%", recovered as f64 / bad_runs.max(1) as f64 * 100.0);
    println!("words lost before recovery      {:>7.2}   (1.00 is the floor)", lost as f64 / recovered.max(1) as f64);
    println!("correct words mangled           {:>7.3}%  (budget {:.3}%)", fp_rate * 100.0, FALSE_POSITIVE_BUDGET * 100.0);

    for o in [&en_ok, &ru_ok, &en_bad, &ru_bad] {
        for s in &o.samples {
            println!("      {s}");
        }
    }

    if fp_rate <= FALSE_POSITIVE_BUDGET {
        println!("\nPASS - within the false-positive budget.");
    } else {
        println!("\nFAIL - mangles too much correct text.");
        std::process::exit(1);
    }
}

#[derive(Default)]
struct Outcome {
    runs: usize,
    /// Runs that reached the layout the user meant to type in.
    recovered: usize,
    /// Words typed before recovery, counted only for runs that recovered.
    words_lost: usize,
    /// Corrections applied while the layout was already right.
    mangled: usize,
    /// Words typed while the layout was already right -- the denominator that
    /// `mangled` is judged against.
    safe_words: usize,
    samples: Vec<String>,
}

/// Replay runs of `words` (valid in `intended`) starting from layout `active`,
/// letting corrections switch the layout as the real program would.
fn simulate(e: &Engine, words: &[(String, u64)], intended: LayoutId, start: LayoutId) -> Outcome {
    let ctx = Context { is_password_field: Some(false), ..Context::default() };
    let mut o = Outcome::default();

    for chunk in words.chunks(RUN_LEN) {
        o.runs += 1;
        let mut active = start;
        let mut recent = None;
        let mut lost = 0usize;
        let mut recovered_at = None;

        for (i, (word, _)) in chunk.iter().enumerate() {
            // The keys a user presses to produce this word on the layout they
            // meant to be using. Those same keys are what reach us whichever
            // layout is actually active.
            let Some(strokes) = intended.layout().strokes_for(word) else { continue };
            let decision = e.decide(&strokes, active, &AVAILABLE, &ctx, recent);

            if active == intended {
                o.safe_words += 1;
                if let Decision::Correct(c) = &decision {
                    o.mangled += 1;
                    if o.samples.len() < 6 {
                        o.samples.push(format!("mangled: {} -> {}", c.from, c.to));
                    }
                }
            } else if recovered_at.is_none() {
                lost += 1;
            }

            recent = Some(match &decision {
                Decision::Correct(c) => c.target_layout,
                Decision::Leave(_) => active,
            });
            if let Decision::Correct(c) = &decision {
                // A correction switches the layout, so the next word arrives in
                // the new one -- exactly what the real program does.
                active = c.target_layout;
                if active == intended && recovered_at.is_none() {
                    recovered_at = Some(i);
                }
            }
        }

        if start == intended || recovered_at.is_some() {
            o.recovered += 1;
            o.words_lost += lost;
        }
    }
    o
}

fn env(key: &str, default: f64) -> f64 {
    std::env::var(key).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

fn load(path: impl AsRef<Path>, script: Script) -> std::io::Result<Vec<(String, u64)>> {
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
        .filter_map(|line| {
            let (word, count) = line.trim().split_once(' ')?;
            let word = word.to_lowercase();
            let count: u64 = count.trim().parse().ok()?;
            (word.chars().count() >= 3 && word.chars().all(in_script)).then_some((word, count))
        })
        .collect())
}

/// Deterministic 80/20 split, hashed on the word so the same word always lands
/// on the same side across runs and threshold changes stay comparable.
fn split(words: &[(String, u64)]) -> (Vec<(String, u64)>, Vec<(String, u64)>) {
    words.iter().cloned().partition(|(w, _)| {
        let mut h = DefaultHasher::new();
        w.hash(&mut h);
        h.finish() % 5 != 0
    })
}
