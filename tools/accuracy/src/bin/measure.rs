use engine::{Engine, Buffer};
use keymap::{LayoutId, Stroke};
use guards::Context;
use std::time::Instant;

fn main() {
    println!("Loading models...");
    let start = Instant::now();
    let en_words = include_str!("../../../../data/en.txt").lines().map(String::from);
    let ru_words = include_str!("../../../../data/ru.txt").lines().map(String::from);
    
    let engine = Engine::new()
        .with_model(LayoutId::UsQwerty, lang::LanguageModel::train("en", en_words))
        .with_model(LayoutId::RuYcuken, lang::LanguageModel::train("ru", ru_words));
        
    let load_time = start.elapsed();
    println!("Model load time: {:?}", load_time);

    let mut buf = Buffer::new();
    let strokes = keymap::RU_YCUKEN.strokes_for("ghbdtn").unwrap(); // privet in RU
    
    let start = Instant::now();
    let decision = engine.decide(&strokes, LayoutId::UsQwerty, &[LayoutId::UsQwerty, LayoutId::RuYcuken], &Context::default(), None);
    let decide_time = start.elapsed();
    
    println!("Decide time: {:?}", decide_time);
    println!("Decision: {:?}", decision);
}
