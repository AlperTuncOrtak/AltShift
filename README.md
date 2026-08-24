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

Measured on held-out OpenSubtitles word lists (Russian ↔ English):

| | rate |
|---|---|
| Correct words wrongly rewritten | **0.096%** |
| Wrong-layout words rescued | **91.2%** |

The first number is the one that matters. A word we fail to fix costs you a
keystroke; a word we wrongly "fix" makes you watch software corrupt your
writing. The budget is 0.1% and the thresholds are tuned to sit just under it.

## Two rules that do not bend

**It never touches the network.** No update check, no telemetry, no crash
reporting, no analytics. For a program that watches your keystrokes, the only
convincing guarantee is one you can verify yourself with a firewall — not a
promise in a README.

**When in doubt, it does nothing.** Password fields, email addresses, URLs,
file paths, anything containing a digit, capitalised words mid-sentence, code
identifiers — all left alone. The keystroke buffer holds the current word
only, in memory, capped, and is never written to disk. See
[`crates/guards`](crates/guards/src/lib.rs), which exists as its own crate so
this claim is one auditable file.

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
