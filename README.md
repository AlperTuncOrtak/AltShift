# AltShift

Fixes text typed with the wrong keyboard layout, then switches the layout so
the rest of the sentence comes out right.

```
keys pressed:  G  H  B  D  T  N
you got:       ghbdtn
you meant:     привет
```

The trick is to remember **which key was pressed**, not which letter appeared.
The letter is a function of the key and the active layout; the key is the
invariant. On space, AltShift renders the buffered keys under every installed
layout, scores each reading against that language, and rewrites the word only
if the alternative wins by a clear margin.

## Status

Early. The engine works and is measured; nothing is wired to a keyboard yet.

Measured on held-out OpenSubtitles word lists (Russian ↔ English), replaying
runs of words and letting a correction switch the layout the way the real
program does:

| | |
|---|---|
| Recovers from a wrong layout | **100%** |
| Words lost before it recovers | **1.07** |
| Correct words wrongly rewritten | **0.026%** |

The first word is always lost — nothing can see a layout mistake before you
make it. 1.07 means that after that one word, it is essentially always right.

The last number is the one that matters. A word we fail to fix costs you a
keystroke; a word we wrongly "fix" makes you watch software corrupt your
writing. The budget is 0.1%, and we sit at a quarter of it.

Both the language model and the thresholds are set by measurement, not taste:
`tools/accuracy` replays corpus words as keystrokes and scores the engine
against a held-out split, so a change that sounds clever but loses accuracy has
nowhere to hide.

## Privacy & What We Don't Do

For a program that watches your keystrokes, claiming to be secure is easy. Proving it is hard. Instead of telling you what we do, here is the verifiable list of what we **don't** do:

1. **We don't touch the network.** No telemetry, no crash reporting, no analytics, no auto-updates. AltShift is entirely offline. Our CI pipeline actively scans the dependency tree and fails if any network crate (like `reqwest` or `curl`) is introduced.
2. **We don't log your keystrokes.** The keystroke buffer only holds the *current* word in memory. It has a strict upper limit (so you can't type a novel into it). The moment you press Space or Enter, it clears.
3. **We don't write keystrokes to disk.** The only thing AltShift writes to your disk is its configuration file (`altshift_settings.json`), which stores your enabled/disabled state, slider preferences, and application blacklist. You can view, edit, or delete it anytime.
4. **We don't touch sensitive fields.** Passwords, email addresses, URLs, file paths, numbers, or capitalized words mid-sentence are strictly ignored. See exactly what we refuse to touch in **[`crates/guards`](crates/guards/src/lib.rs)**, which exists as its own isolated crate so this claim is just one easily auditable file.

## Download / Releases

**We do not compile the `.exe` files on our local machines.** Every release provided on the [Releases](https://github.com/AlperTuncOrtak/AltShift/releases) page is built entirely in the open by GitHub Actions. You can inspect the [build logs](https://github.com/AlperTuncOrtak/AltShift/actions) for any version to verify exactly which source code produced the installer.

For a program that reads your keystrokes, knowing exactly where the binary came from is non-negotiable.

## Scope

Different alphabets only: Cyrillic ↔ Latin first, then Greek, Arabic, Hebrew,
Armenian, Georgian — same code, one more table each.

Deliberately out of scope: restoring Turkish diacritics (`cocuk` → `çocuk`),
which is spell-checking rather than a layout problem; Japanese and Chinese,
where an IME sits in the way and the correct output is not unique; and reading
anything on your screen.

## Build

```sh
./fetch-wordlists.sh   # word lists are fetched, not committed
cargo test --workspace
cargo run --release -p accuracy
```

## Layout

| Crate | |
|---|---|
| `keymap` | physical keys and layout tables |
| `lang` | dictionary + character trigram scoring |
| `guards` | what must never be touched |
| `engine` | buffer, candidate scoring, the decision |
| `tools/accuracy` | measures the engine with no keyboard hook and no OS permissions |

Nothing above depends on an operating system, which is what lets the hard part
— deciding *whether* to rewrite someone's word — be developed and measured
anywhere and shipped everywhere unchanged.

## License

MIT
