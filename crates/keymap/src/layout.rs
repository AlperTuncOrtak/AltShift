//! Keyboard layout tables and the stroke <-> text conversions built on them.

use crate::key::{Key, Stroke, KEY_COUNT};

/// Writing system a layout produces. Used to decide whether two layouts are
/// even worth comparing: swapping between two Latin layouts is a spell-check
/// problem, not a layout problem, and is out of scope by design.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Script {
    Latin,
    Cyrillic,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum LayoutId {
    /// United States QWERTY.
    UsQwerty,
    /// Russian ЙЦУКЕН.
    RuYcuken,
}

impl LayoutId {
    pub const ALL: [LayoutId; 2] = [LayoutId::UsQwerty, LayoutId::RuYcuken];

    pub fn layout(self) -> &'static Layout {
        match self {
            LayoutId::UsQwerty => &US_QWERTY,
            LayoutId::RuYcuken => &RU_YCUKEN,
        }
    }
}

pub struct Layout {
    pub id: LayoutId,
    pub name: &'static str,
    pub script: Script,
    /// `[unshifted, shifted]` per key, indexed by `Key::index`.
    /// `'\0'` marks a key that produces nothing in this layout.
    table: [[char; 2]; KEY_COUNT],
}

impl Layout {
    /// The character this layout produces for a keypress.
    pub fn char_for(&self, stroke: Stroke) -> Option<char> {
        let c = self.table[stroke.key.index()][stroke.shift as usize];
        (c != '\0').then_some(c)
    }

    /// Render a run of keypresses as the text this layout would have produced.
    ///
    /// Returns `None` if any stroke is unmapped. A partially rendered candidate
    /// is worse than no candidate: it would let us offer the user text they
    /// could not have typed.
    pub fn render(&self, strokes: &[Stroke]) -> Option<String> {
        strokes.iter().map(|&s| self.char_for(s)).collect()
    }

    /// Inverse of [`Layout::char_for`].
    pub fn stroke_for(&self, c: char) -> Option<Stroke> {
        for &key in Key::ALL.iter() {
            let row = &self.table[key.index()];
            if row[0] == c {
                return Some(Stroke::new(key, false));
            }
            if row[1] == c {
                return Some(Stroke::new(key, true));
            }
        }
        None
    }

    /// The keypresses a user would make to type `text` on this layout.
    ///
    /// This is what lets the accuracy harness simulate wrong-layout typing from
    /// ordinary corpus text, so the engine can be measured without a keyboard
    /// hook and without any OS permissions.
    pub fn strokes_for(&self, text: &str) -> Option<Vec<Stroke>> {
        text.chars().map(|c| self.stroke_for(c)).collect()
    }
}

#[rustfmt::skip]
pub static US_QWERTY: Layout = Layout {
    id: LayoutId::UsQwerty,
    name: "English (US, QWERTY)",
    script: Script::Latin,
    table: [
        ['`','~'], ['1','!'], ['2','@'], ['3','#'], ['4','$'], ['5','%'], ['6','^'],
        ['7','&'], ['8','*'], ['9','('], ['0',')'], ['-','_'], ['=','+'],

        ['q','Q'], ['w','W'], ['e','E'], ['r','R'], ['t','T'], ['y','Y'], ['u','U'],
        ['i','I'], ['o','O'], ['p','P'], ['[','{'], [']','}'], ['\\','|'],

        ['a','A'], ['s','S'], ['d','D'], ['f','F'], ['g','G'], ['h','H'], ['j','J'],
        ['k','K'], ['l','L'], [';',':'], ['\'','"'],

        ['z','Z'], ['x','X'], ['c','C'], ['v','V'], ['b','B'], ['n','N'], ['m','M'],
        [',','<'], ['.','>'], ['/','?'],
    ],
};

#[rustfmt::skip]
pub static RU_YCUKEN: Layout = Layout {
    id: LayoutId::RuYcuken,
    name: "Russian (ЙЦУКЕН)",
    script: Script::Cyrillic,
    table: [
        ['ё','Ё'], ['1','!'], ['2','"'], ['3','№'], ['4',';'], ['5','%'], ['6',':'],
        ['7','?'], ['8','*'], ['9','('], ['0',')'], ['-','_'], ['=','+'],

        ['й','Й'], ['ц','Ц'], ['у','У'], ['к','К'], ['е','Е'], ['н','Н'], ['г','Г'],
        ['ш','Ш'], ['щ','Щ'], ['з','З'], ['х','Х'], ['ъ','Ъ'], ['\\','/'],

        ['ф','Ф'], ['ы','Ы'], ['в','В'], ['а','А'], ['п','П'], ['р','Р'], ['о','О'],
        ['л','Л'], ['д','Д'], ['ж','Ж'], ['э','Э'],

        ['я','Я'], ['ч','Ч'], ['с','С'], ['м','М'], ['и','И'], ['т','Т'], ['ь','Ь'],
        ['б','Б'], ['ю','Ю'], ['.',','],
    ],
};

#[cfg(test)]
mod tests {
    use super::*;

    /// The canonical example: "привет" typed while the US layout was active.
    #[test]
    fn ghbdtn_is_privet() {
        let strokes = US_QWERTY.strokes_for("ghbdtn").unwrap();
        assert_eq!(RU_YCUKEN.render(&strokes).unwrap(), "привет");
    }

    #[test]
    fn the_swap_is_symmetric() {
        let strokes = RU_YCUKEN.strokes_for("привет").unwrap();
        assert_eq!(US_QWERTY.render(&strokes).unwrap(), "ghbdtn");
    }

    /// Every layout must round-trip through its own table, or a correction
    /// could silently mangle text that was already correct.
    #[test]
    fn every_key_round_trips() {
        for id in LayoutId::ALL {
            let layout = id.layout();
            for &key in Key::ALL.iter() {
                for shift in [false, true] {
                    let stroke = Stroke::new(key, shift);
                    if let Some(c) = layout.char_for(stroke) {
                        assert_eq!(
                            layout.stroke_for(c),
                            Some(stroke),
                            "{}: {c:?} did not round-trip",
                            layout.name
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn shift_is_carried_through_the_swap() {
        let strokes = US_QWERTY.strokes_for("Ghbdtn").unwrap();
        assert_eq!(RU_YCUKEN.render(&strokes).unwrap(), "Привет");
    }

    /// A character the layout cannot produce must not yield a half-rendered
    /// candidate.
    #[test]
    fn unmapped_input_yields_no_candidate() {
        assert!(US_QWERTY.strokes_for("café").is_none());
    }
}
