//! The game language's closed vocabulary and its tokenizer.
//!
//! Every word a game sentence may use is listed here. A word that isn't is an
//! error with a "did you mean" hint (or skipped with `--lenient`). Things have
//! a singular and a plural and map to one tag; numbers keep their source text.

use crate::parser::Diag;
use std::fmt;

/// Things the language knows: (singular = the tag, plural).
pub const THINGS: &[(&str, &str)] = &[
    ("coin", "coins"),
    ("gem", "gems"),
    ("star", "stars"),
    ("apple", "apples"),
    ("heart", "hearts"),
    ("key", "keys"),
    ("slime", "slimes"),
    ("spider", "spiders"),
    ("skeleton", "skeletons"),
    ("zombie", "zombies"),
    ("blob", "blobs"),
    ("bat", "bats"),
    ("ghost", "ghosts"),
    ("alien", "aliens"),
    ("spike", "spikes"),
    ("saw", "saws"),
    ("lava", "lavas"),
    ("door", "doors"),
    ("flag", "flags"),
    ("portal", "portals"),
    ("exit", "exits"),
    ("enemy", "enemies"),
    ("pickup", "pickups"),
    ("hazard", "hazards"),
    ("shot", "shots"),
    ("bomb", "bombs"),
    ("shield", "shields"),
    ("fish", "fish"),
    ("perch", "perches"),
    ("roach", "roaches"),
    ("pike", "pikes"),
    ("hook", "hooks"),
    ("hole", "holes"),
];

/// Sound effects Droid can play.
pub const SOUNDS: &[&str] = &[
    "shoot", "laser", "hit", "explosion", "jump", "coin", "pickup", "powerup", "hurt", "click", "win", "lose",
    "bounce", "step", "splash",
];

/// Music waveforms.
pub const WAVES: &[&str] = &["square", "saw", "triangle", "sine"];

/// One-shot model animations.
pub const ANIMS: &[&str] = &["jump", "attack", "hit", "spin", "wave", "nod"];

/// Named keys (single letters are keys too).
pub const KEYS: &[&str] = &["space", "enter", "shift", "left", "right", "up", "down"];

/// Number words.
pub const NUMBER_WORDS: &[(&str, f32)] = &[
    ("one", 1.0),
    ("two", 2.0),
    ("three", 3.0),
    ("four", 4.0),
    ("five", 5.0),
    ("six", 6.0),
    ("seven", 7.0),
    ("eight", 8.0),
    ("nine", 9.0),
    ("ten", 10.0),
];

/// Modifiers that shift a sentence's code profile.
pub const MODIFIERS: &[&str] = &["safely", "quickly", "simply"];

/// Grammar words: everything the parser matches besides things, sounds,
/// keys, numbers and modifiers.
pub const GRAMMAR: &[&str] = &[
    "a", "above", "across", "add", "an", "and", "animate", "animation", "are", "arrow", "arrows", "as", "at",
    "back", "beats", "below", "both", "bottom", "bounce", "by", "cycles", "destroy", "down", "each", "every",
    "faster", "face", "falls", "fewer", "flies", "for", "forward", "frame", "frames", "game", "goes", "ground",
    "has", "held", "is", "jumps", "when", "high", "higher", "hits", "in", "it", "keys", "lands", "left", "life", "lives", "loses",
    "marches", "middle", "minute", "move", "moves", "music", "next", "no", "of", "on", "per", "percent",
    "play", "player", "points", "point", "presses", "random", "reaches", "restart", "right", "safe", "score",
    "second", "seconds", "shoots", "show", "side", "sideways", "smoothly", "sound", "speeding", "start",
    "stepping", "squash", "the", "then", "time", "times", "to", "top", "touches", "toward", "towards",
    "turning", "twice", "unit", "units", "up", "using", "walks", "wander", "wanders", "wave", "waves",
    "where", "while", "wins", "with", "within", "patrols",
    // fishing: hooks on lines, the shout, bites and catches
    "after", "bites", "comes", "follows", "from", "hangs", "holds", "hooked", "its", "line", "loud", "meter",
    "new", "or", "pulling", "rises", "shout", "shouts", "sinking", "swims", "fighting", "bursts", "stays", "gets",
    "away",
];

/// Every word the game language knows, sorted and unique.
pub fn vocabulary() -> Vec<&'static str> {
    let mut v: Vec<&'static str> = Vec::new();
    for (s, p) in THINGS {
        v.push(s);
        v.push(p);
    }
    v.extend(SOUNDS);
    v.extend(WAVES);
    v.extend(ANIMS);
    v.extend(KEYS);
    v.extend(NUMBER_WORDS.iter().map(|(w, _)| *w));
    v.extend(MODIFIERS);
    v.extend(GRAMMAR);
    v.sort_unstable();
    v.dedup();
    v
}

/// Is `w` (lowercase) a word the language knows? Single letters are keys.
pub fn known(w: &str) -> bool {
    (w.len() == 1 && w.chars().all(|c| c.is_ascii_lowercase())) || vocabulary().binary_search(&w).is_ok()
}

/// A thing word: (tag, plural).
pub fn thing(w: &str) -> Option<(&'static str, bool)> {
    THINGS.iter().find_map(|(s, p)| {
        if *s == w {
            Some((*s, false))
        } else if *p == w {
            Some((*s, true))
        } else {
            None
        }
    })
}

#[derive(Clone, Debug, PartialEq)]
pub enum Tok {
    /// A known word, lowercase.
    Word(String),
    /// A number and how it was written.
    Num(f32, String),
    /// Quoted text: a custom tag, a sprite name, a message, notes.
    Quote(String),
    Comma,
    Period,
}

impl fmt::Display for Tok {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Tok::Word(w) => write!(f, "{w}"),
            Tok::Num(_, s) => write!(f, "{s}"),
            Tok::Quote(q) => write!(f, "\"{q}\""),
            Tok::Comma => write!(f, ","),
            Tok::Period => write!(f, "."),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    pub tok: Tok,
    /// Byte offsets in the source, for quoting sentences back.
    pub start: usize,
    pub end: usize,
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut cur = vec![i; b.len() + 1];
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        prev = cur;
    }
    prev[b.len()]
}

/// Closest known word; ties go to the alphabetically first, so hints are deterministic.
pub fn closest(word: &str) -> Option<&'static str> {
    let word = word.to_lowercase();
    let limit = (word.chars().count() / 3).max(1) + 1;
    vocabulary()
        .into_iter()
        .filter(|v| v.len() > 1)
        .map(|v| (levenshtein(&word, v), v))
        .filter(|(d, _)| *d <= limit)
        .min()
        .map(|(_, v)| v)
}

pub fn unknown_word(word: &str) -> Diag {
    let d = Diag::new(format!("the game language doesn't know the word \"{word}\""));
    match closest(word) {
        Some(c) => d.hint(format!("did you mean \"{c}\"? (or pass --lenient to skip unknown words)")),
        None => d.hint("rephrase with known words, or pass --lenient to skip it (`senne --game --vocabulary` lists them)"),
    }
}

fn number_word(w: &str) -> Option<f32> {
    NUMBER_WORDS.iter().find(|(n, _)| *n == w).map(|(_, v)| *v)
}

/// Text -> tokens. Unknown words are errors unless `lenient`, which drops them.
pub fn tokenize(src: &str, lenient: bool) -> Result<Vec<Token>, Vec<Diag>> {
    let b = src.as_bytes();
    let mut out = Vec::new();
    let mut diags = Vec::new();
    let mut i = 0;
    let is_open = |c: char| c == '"' || c == '\u{201c}' || c == '\u{201d}';
    while i < b.len() {
        let c = src[i..].chars().next().expect("in bounds");
        let w = c.len_utf8();
        if is_open(c) {
            let start = i;
            let body = i + w;
            let mut j = body;
            loop {
                match src[j..].chars().next() {
                    None => {
                        diags.push(Diag::new(format!("a quote is never closed: {}", &src[start..])).hint("close it with \""));
                        return Err(diags);
                    }
                    Some(ch) if is_open(ch) => break,
                    Some(ch) => j += ch.len_utf8(),
                }
            }
            let close = src[j..].chars().next().expect("quote").len_utf8();
            out.push(Token { tok: Tok::Quote(src[body..j].to_string()), start, end: j + close });
            i = j + close;
        } else if c.is_ascii_digit() || (c == '-' && b.get(i + 1).is_some_and(|d| d.is_ascii_digit())) {
            let start = i;
            i += 1;
            while i < b.len() && b[i].is_ascii_digit() {
                i += 1;
            }
            if i + 1 < b.len() && b[i] == b'.' && b[i + 1].is_ascii_digit() {
                i += 1;
                while i < b.len() && b[i].is_ascii_digit() {
                    i += 1;
                }
            }
            let text = &src[start..i];
            let n: f32 = text.parse().expect("digits parse");
            out.push(Token { tok: Tok::Num(n, text.to_string()), start, end: i });
        } else if c.is_alphabetic() {
            let start = i;
            while i < b.len() {
                let ch = src[i..].chars().next().expect("in bounds");
                if ch.is_alphanumeric() || ch == '\'' || ch == '_' {
                    i += ch.len_utf8();
                } else {
                    break;
                }
            }
            let word = src[start..i].to_lowercase();
            if let Some(n) = number_word(&word) {
                out.push(Token { tok: Tok::Num(n, word), start, end: i });
            } else if known(&word) {
                out.push(Token { tok: Tok::Word(word), start, end: i });
            } else if !lenient {
                diags.push(unknown_word(&src[start..i]));
            }
        } else if c == ',' {
            out.push(Token { tok: Tok::Comma, start: i, end: i + 1 });
            i += 1;
        } else if c == '.' || c == '!' {
            out.push(Token { tok: Tok::Period, start: i, end: i + 1 });
            i += 1;
        } else if c.is_whitespace() {
            i += w;
        } else if lenient {
            i += w;
        } else {
            diags.push(Diag::new(format!("unexpected character '{c}'")).hint("sentences use words, numbers, quotes, commas and periods"));
            i += w;
        }
    }
    if diags.is_empty() {
        Ok(out)
    } else {
        Err(diags)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toks(s: &str) -> Vec<Tok> {
        tokenize(s, false).unwrap().into_iter().map(|t| t.tok).collect()
    }

    #[test]
    fn numbers_decimals_and_negatives() {
        assert_eq!(
            toks("falls below -12.5."),
            vec![Tok::Word("falls".into()), Tok::Word("below".into()), Tok::Num(-12.5, "-12.5".into()), Tok::Period]
        );
    }

    #[test]
    fn quotes_keep_their_text() {
        assert_eq!(toks("show \"COINS {score}\""), vec![Tok::Word("show".into()), Tok::Quote("COINS {score}".into())]);
    }

    #[test]
    fn unknown_word_gets_a_hint() {
        let d = tokenize("each slime patrolls", false).unwrap_err();
        assert!(d[0].message.contains("patrolls"));
        assert!(d[0].hint.as_deref().unwrap().contains("\"patrols\""), "{:?}", d[0].hint);
    }

    #[test]
    fn lenient_drops_unknown_words() {
        assert_eq!(toks("the"), tokenize("the fluffy", true).unwrap().into_iter().map(|t| t.tok).collect::<Vec<_>>());
    }

    #[test]
    fn plurals_map_to_one_tag() {
        assert_eq!(thing("enemies"), Some(("enemy", true)));
        assert_eq!(thing("coin"), Some(("coin", false)));
        assert_eq!(thing("fish"), Some(("fish", false)));
        assert_eq!(thing("perches"), Some(("perch", true)));
    }

    #[test]
    fn vocabulary_is_sorted_and_unique() {
        let v = vocabulary();
        let mut s = v.clone();
        s.sort_unstable();
        s.dedup();
        assert_eq!(v, s);
    }
}
