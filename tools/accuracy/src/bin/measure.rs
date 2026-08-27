//! Latency benchmark for the correction engine.
//!
//! Accuracy tells us whether the engine decides correctly; this tells us
//! whether it decides fast enough. The two are separate budgets and a change
//! can pass one while breaking the other.
//!
//! Speed here is a requirement, not a feature. What bothers a user is not the
//! mistake but *seeing* it: a correction that lands within a few tens of
//! milliseconds of the space bar is never perceived at all, while one that
//! takes a third of a second is watched. Windows adds a hard limit on top --
//! a low-level keyboard hook that does not return in ~300 ms is silently
//! removed -- which is why the decision must never run on the hook thread.

use engine::{Decision, Engine, Thresholds};
use guards::Context;
use keymap::{LayoutId, Stroke, US_QWERTY};
use lang::LanguageModel;
use std::time::Instant;

/// Budget for a single decision, from the plan.
const DECIDE_BUDGET_US: u128 = 5_000;

fn main() {
    let en = match load("data/en.txt") {
        Ok(w) => w,
        Err(e) => return eprintln!("data/en.txt: {e}\nRun ./fetch-wordlists.sh first."),
    };
    let ru = match load("data/ru.txt") {
        Ok(w) => w,
        Err(e) => return eprintln!("data/ru.txt: {e}\nRun ./fetch-wordlists.sh first."),
    };

    // Model construction happens once at startup, so it is a launch-time cost
    // rather than a typing-time one -- but a slow one still shows up as an app
    // that feels broken for its first seconds.
    let start = Instant::now();
    let engine = Engine::new()
        .with_model(LayoutId::UsQwerty, LanguageModel::train("en", en))
        .with_model(LayoutId::RuYcuken, LanguageModel::train("ru", ru))
        .with_thresholds(Thresholds::default());
    println!("model load          {:>8.1} ms", start.elapsed().as_secs_f64() * 1000.0);

    // The platform layer has positively cleared this field as ordinary text.
    // `Context::default()` would not do: it means "unknown", which the guards
    // treat as a password field, and the benchmark would then measure the
    // rejection path instead of the scoring it exists to time.
    let ctx = Context { is_password_field: Some(false), ..Context::default() };

    let both = [LayoutId::UsQwerty, LayoutId::RuYcuken];
    let one = [LayoutId::UsQwerty];

    println!();
    println!("{:<20}{:>12}{:>12}", "", "per call", "vs budget");
    for (label, word, layouts) in [
        // Cyrillic typed while US is active: the full path, ending in a rewrite.
        ("correction", "ghbdtn", &both[..]),
        // Ordinary English: scored, then left alone.
        ("left alone", "message", &both[..]),
        // Only one layout installed, so there is no candidate to compare.
        ("single layout", "message", &one[..]),
    ] {
        let strokes = strokes(word);
        let per_call = time(|| {
            engine.decide(&strokes, LayoutId::UsQwerty, layouts, &ctx, None);
        });
        println!(
            "{label:<20}{:>9.1} µs{:>11}",
            per_call as f64 / 1000.0,
            if per_call <= DECIDE_BUDGET_US * 1000 { "ok" } else { "OVER" }
        );
    }

    // Sanity: the benchmark must be timing a real decision, not a guard
    // rejection that returns before any scoring happens.
    let d = engine.decide(&strokes("ghbdtn"), LayoutId::UsQwerty, &both, &ctx, None);
    match d {
        Decision::Correct(c) => println!("\nsanity: ghbdtn -> {}", c.to),
        other => println!("\nsanity FAILED: expected a correction, got {other:?}"),
    }
}

/// Median of many runs, in nanoseconds. A single measurement is noise.
fn time(mut f: impl FnMut()) -> u128 {
    const RUNS: usize = 2_000;
    for _ in 0..RUNS / 10 {
        f(); // warm caches so the first samples are not the slow ones
    }
    let mut samples: Vec<u128> = (0..RUNS)
        .map(|_| {
            let t = Instant::now();
            f();
            t.elapsed().as_nanos()
        })
        .collect();
    samples.sort_unstable();
    samples[samples.len() / 2]
}

/// The keys a user presses to type `word` on a US layout.
///
/// `ghbdtn` is Latin -- it is what the *screen* shows when Cyrillic is typed in
/// the wrong layout, so the strokes behind it come from the US table. Asking
/// the Russian table for them yields `None`, which is how this benchmark used
/// to panic on its second statement.
fn strokes(word: &str) -> Vec<Stroke> {
    US_QWERTY.strokes_for(word).expect("benchmark words must be typeable on US QWERTY")
}

/// Read at run time, not via `include_str!`.
///
/// The word lists are fetched rather than committed, so embedding them at
/// compile time makes a fresh clone fail to build before it can even reach the
/// fetch script.
fn load(path: &str) -> std::io::Result<Vec<(String, u64)>> {
    Ok(lang::parse_frequency_list(&std::fs::read_to_string(path)?).collect())
}
