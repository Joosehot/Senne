//! The lexicon: words and phrases -> intent tokens. This is Senne's music
//! theory: a closed, hand-written vocabulary. Anything outside it is an error
//! (or ignored with `--lenient`), never a guess.

use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Verb {
    Read,
    Write,
    Parse,
    Retry,
    Spawn,
    ForEach,
    Log,
    Continue,
    Stop,
    Sum,
    Count,
    Sort,
    Dedupe,
    SkipBlank,
    Trim,
    Print,
    Wait,
    Create,
    List,
}

impl Verb {
    /// Verbs that work on a whole collection. Inside a loop they close it:
    /// "for each line, parse it, then sum them" sums after the loop.
    pub fn is_aggregate(self) -> bool {
        matches!(self, Verb::Sum | Verb::Count | Verb::Sort | Verb::Dedupe | Verb::Write)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Noun {
    File,
    Files,
    Lines,
    Integer,
    Float,
    Env,
    Dir,
    Error,
    Workers,
    It,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Modifier {
    Safely,
    Quickly,
    Simply,
    Backoff,
    Atomically,
    Parallel,
}

/// Modifiers that shift the clause profile; each needs `[modifiers.<key>]`.
pub const MODIFIER_KEYS: &[&str] = &["safely", "quickly", "simply"];

impl Modifier {
    pub fn profile_key(self) -> Option<&'static str> {
        match self {
            Modifier::Safely => Some("safely"),
            Modifier::Quickly => Some("quickly"),
            Modifier::Simply => Some("simply"),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Conn {
    /// "then": ends an error handler.
    Then,
    /// "after that", "finally": ends an error handler and any open loop.
    After,
    And,
    Comma,
    /// "." or ";": ends everything.
    Period,
    /// "if it fails", "on error": the following clauses handle the previous one.
    IfFails,
    /// "or", "defaulting to": the next value is a fallback.
    Fallback,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lex {
    Verb(Verb),
    Noun(Noun),
    Mod(Modifier),
    Conn(Conn),
    /// Recognized but meaningless here ("the", "times", "if missing").
    Filler,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TokKind {
    Lex(Lex),
    Num(u64),
    Millis(u64),
    Quoted(String),
    Unknown,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    pub kind: TokKind,
    /// Source text, original case.
    pub text: String,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            TokKind::Lex(Lex::Verb(v)) => write!(f, "[{v:?}]"),
            TokKind::Lex(Lex::Noun(n)) => write!(f, "<{n:?}>"),
            TokKind::Lex(Lex::Mod(m)) => write!(f, "~{m:?}"),
            TokKind::Lex(Lex::Conn(c)) => write!(f, "|{c:?}|"),
            TokKind::Lex(Lex::Filler) => write!(f, "_"),
            TokKind::Num(n) => write!(f, "#{n}"),
            TokKind::Millis(ms) => write!(f, "{ms}ms"),
            TokKind::Quoted(s) => write!(f, "{s:?}"),
            TokKind::Unknown => write!(f, "?{}", self.text),
        }
    }
}

use Conn as C;
use Lex::{Conn as K, Filler as F, Mod as M, Noun as N, Verb as V};
use Modifier as Md;
use Noun as No;
use Verb as Vb;

/// Phrase table. Longest match wins; within a length, table order doesn't
/// matter because phrases are unique.
pub const PHRASES: &[(&str, Lex)] = &[
    // --- verbs
    ("read", V(Vb::Read)),
    ("load", V(Vb::Read)),
    ("open", V(Vb::Read)),
    ("get", V(Vb::Read)),
    ("fetch", V(Vb::Read)),
    ("write", V(Vb::Write)),
    ("save", V(Vb::Write)),
    ("store", V(Vb::Write)),
    ("parse", V(Vb::Parse)),
    ("convert", V(Vb::Parse)),
    ("retry", V(Vb::Retry)),
    ("try again", V(Vb::Retry)),
    ("spawn", V(Vb::Spawn)),
    ("start", V(Vb::Spawn)),
    ("for each", V(Vb::ForEach)),
    ("for every", V(Vb::ForEach)),
    ("each", V(Vb::ForEach)),
    ("every", V(Vb::ForEach)),
    ("go through", V(Vb::ForEach)),
    ("loop over", V(Vb::ForEach)),
    ("log", V(Vb::Log)),
    ("report", V(Vb::Log)),
    ("warn", V(Vb::Log)),
    ("print the error", V(Vb::Log)),
    ("show the error", V(Vb::Log)),
    ("continue", V(Vb::Continue)),
    ("keep going", V(Vb::Continue)),
    ("move on", V(Vb::Continue)),
    ("carry on", V(Vb::Continue)),
    ("skip", V(Vb::Continue)),
    ("skip it", V(Vb::Continue)),
    ("ignore it", V(Vb::Continue)),
    ("stop", V(Vb::Stop)),
    ("give up", V(Vb::Stop)),
    ("bail", V(Vb::Stop)),
    ("bail out", V(Vb::Stop)),
    ("abort", V(Vb::Stop)),
    ("return the error", V(Vb::Stop)),
    ("propagate", V(Vb::Stop)),
    ("propagate the error", V(Vb::Stop)),
    ("sum", V(Vb::Sum)),
    ("add up", V(Vb::Sum)),
    ("count", V(Vb::Count)),
    ("sort", V(Vb::Sort)),
    ("order", V(Vb::Sort)),
    ("remove duplicates", V(Vb::Dedupe)),
    ("drop duplicates", V(Vb::Dedupe)),
    ("dedupe", V(Vb::Dedupe)),
    ("dedup", V(Vb::Dedupe)),
    ("deduplicate", V(Vb::Dedupe)),
    ("skip blank lines", V(Vb::SkipBlank)),
    ("skip empty lines", V(Vb::SkipBlank)),
    ("skip blank ones", V(Vb::SkipBlank)),
    ("skip empty ones", V(Vb::SkipBlank)),
    ("ignore blank lines", V(Vb::SkipBlank)),
    ("ignore empty lines", V(Vb::SkipBlank)),
    ("drop blank lines", V(Vb::SkipBlank)),
    ("drop empty lines", V(Vb::SkipBlank)),
    ("remove blank lines", V(Vb::SkipBlank)),
    ("remove empty lines", V(Vb::SkipBlank)),
    ("skip it if it is blank", V(Vb::SkipBlank)),
    ("skip it if blank", V(Vb::SkipBlank)),
    ("skip it if empty", V(Vb::SkipBlank)),
    ("trim", V(Vb::Trim)),
    ("strip", V(Vb::Trim)),
    ("print", V(Vb::Print)),
    ("show", V(Vb::Print)),
    ("display", V(Vb::Print)),
    ("output", V(Vb::Print)),
    ("wait", V(Vb::Wait)),
    ("sleep", V(Vb::Wait)),
    ("pause", V(Vb::Wait)),
    ("create", V(Vb::Create)),
    ("make", V(Vb::Create)),
    ("ensure", V(Vb::Create)),
    ("list", V(Vb::List)),
    // --- nouns
    ("file", N(No::File)),
    ("files", N(No::Files)),
    ("line", N(No::Lines)),
    ("lines", N(No::Lines)),
    ("integer", N(No::Integer)),
    ("integers", N(No::Integer)),
    ("int", N(No::Integer)),
    ("number", N(No::Integer)),
    ("numbers", N(No::Integer)),
    ("float", N(No::Float)),
    ("floats", N(No::Float)),
    ("decimal", N(No::Float)),
    ("decimals", N(No::Float)),
    ("environment variable", N(No::Env)),
    ("env var", N(No::Env)),
    ("env variable", N(No::Env)),
    ("variable", N(No::Env)),
    ("directory", N(No::Dir)),
    ("folder", N(No::Dir)),
    ("dir", N(No::Dir)),
    ("error", N(No::Error)),
    ("worker", N(No::Workers)),
    ("workers", N(No::Workers)),
    ("thread", N(No::Workers)),
    ("threads", N(No::Workers)),
    ("it", N(No::It)),
    ("this", N(No::It)),
    ("that", N(No::It)),
    ("them", N(No::It)),
    ("result", N(No::It)),
    ("results", N(No::It)),
    ("total", N(No::It)),
    ("value", N(No::It)),
    ("values", N(No::It)),
    // --- modifiers
    ("safely", M(Md::Safely)),
    ("safe", M(Md::Safely)),
    ("carefully", M(Md::Safely)),
    ("robustly", M(Md::Safely)),
    ("reliably", M(Md::Safely)),
    ("quickly", M(Md::Quickly)),
    ("fast", M(Md::Quickly)),
    ("efficiently", M(Md::Quickly)),
    ("large", M(Md::Quickly)),
    ("huge", M(Md::Quickly)),
    ("simply", M(Md::Simply)),
    ("just", M(Md::Simply)),
    ("briefly", M(Md::Simply)),
    ("quick and dirty", M(Md::Simply)),
    ("backoff", M(Md::Backoff)),
    ("backing off", M(Md::Backoff)),
    ("exponential backoff", M(Md::Backoff)),
    ("atomically", M(Md::Atomically)),
    ("in parallel", M(Md::Parallel)),
    ("concurrently", M(Md::Parallel)),
    ("parallel", M(Md::Parallel)),
    // --- connectives
    ("then", K(C::Then)),
    ("and then", K(C::Then)),
    ("next", K(C::Then)),
    ("after that", K(C::After)),
    ("afterwards", K(C::After)),
    ("finally", K(C::After)),
    ("when done", K(C::After)),
    ("and", K(C::And)),
    ("if it fails", K(C::IfFails)),
    ("if that fails", K(C::IfFails)),
    ("if this fails", K(C::IfFails)),
    ("if anything fails", K(C::IfFails)),
    ("if it errors", K(C::IfFails)),
    ("if there is an error", K(C::IfFails)),
    ("if it does not parse", K(C::IfFails)),
    ("if it doesn't parse", K(C::IfFails)),
    ("if parsing fails", K(C::IfFails)),
    ("if reading fails", K(C::IfFails)),
    ("if writing fails", K(C::IfFails)),
    ("if all attempts fail", K(C::IfFails)),
    ("if they all fail", K(C::IfFails)),
    ("when it fails", K(C::IfFails)),
    ("on error", K(C::IfFails)),
    ("on failure", K(C::IfFails)),
    ("otherwise", K(C::IfFails)),
    ("or", K(C::Fallback)),
    ("or else", K(C::Fallback)),
    ("default to", K(C::Fallback)),
    ("defaulting to", K(C::Fallback)),
    ("fall back to", K(C::Fallback)),
    ("falling back to", K(C::Fallback)),
    ("with a default of", K(C::Fallback)),
    ("or a default", K(C::Fallback)),
    ("or default", K(C::Fallback)),
    // --- filler: recognized, carries no intent
    ("the", F),
    ("a", F),
    ("an", F),
    ("of", F),
    ("to", F),
    ("into", F),
    ("as", F),
    ("from", F),
    ("in", F),
    ("with", F),
    ("using", F),
    ("its", F),
    ("all", F),
    ("times", F),
    ("time", F),
    ("attempts", F),
    ("tries", F),
    ("up", F),
    ("contents", F),
    ("content", F),
    ("text", F),
    ("named", F),
    ("called", F),
    ("at", F),
    ("path", F),
    ("between", F),
    ("delay", F),
    ("apart", F),
    ("so", F),
    ("please", F),
    ("process", F),
    ("handle", F),
    ("if missing", F),
    ("if needed", F),
    ("if it does not exist", F),
    ("if it doesn't exist", F),
    ("does not exist", F),
    ("sum of", F),
    ("count of", F),
    ("number of", F),
    ("on", F),
    ("over", F),
    ("across", F),
];

pub const NUMBER_WORDS: &[(&str, u64)] = &[
    ("zero", 0),
    ("one", 1),
    ("once", 1),
    ("two", 2),
    ("twice", 2),
    ("three", 3),
    ("thrice", 3),
    ("four", 4),
    ("five", 5),
    ("six", 6),
    ("seven", 7),
    ("eight", 8),
    ("nine", 9),
    ("ten", 10),
    ("twelve", 12),
    ("sixteen", 16),
    ("twenty", 20),
];

const MILLIS_UNITS: &[(&str, u64)] = &[
    ("ms", 1),
    ("millisecond", 1),
    ("milliseconds", 1),
    ("s", 1000),
    ("sec", 1000),
    ("secs", 1000),
    ("second", 1000),
    ("seconds", 1000),
];

const MAX_PHRASE_WORDS: usize = 6;

fn lookup(phrase: &str) -> Option<Lex> {
    PHRASES.iter().find(|(p, _)| *p == phrase).map(|(_, l)| *l)
}

fn is_known_word(word: &str) -> bool {
    PHRASES.iter().any(|(p, _)| p.split(' ').any(|w| w == word))
        || NUMBER_WORDS.iter().any(|(w, _)| *w == word)
}

/// Undo English inflection for words the lexicon doesn't list directly:
/// "reading" -> "read", "parsing" -> "parse", "logging" -> "log",
/// "retries" -> "retry". Only returns forms the lexicon knows.
pub fn stem(word: &str) -> Option<String> {
    let mut tries: Vec<String> = Vec::new();
    if let Some(base) = word.strip_suffix("ing") {
        tries.push(base.to_string());
        tries.push(format!("{base}e"));
        let b = base.as_bytes();
        if b.len() >= 2 && b[b.len() - 1] == b[b.len() - 2] {
            tries.push(base[..base.len() - 1].to_string());
        }
    }
    if let Some(base) = word.strip_suffix("ies") {
        tries.push(format!("{base}y"));
    }
    if let Some(base) = word.strip_suffix("ied") {
        tries.push(format!("{base}y"));
    }
    if let Some(base) = word.strip_suffix("es") {
        tries.push(base.to_string());
    }
    if let Some(base) = word.strip_suffix('s') {
        tries.push(base.to_string());
    }
    if let Some(base) = word.strip_suffix("ed") {
        tries.push(base.to_string());
        tries.push(format!("{base}e"));
    }
    tries.into_iter().find(|t| lookup(t).is_some_and(|l| matches!(l, Lex::Verb(_))))
}

#[derive(Debug)]
enum Raw {
    Word(String, String), // (normalized lowercase, original)
    Quoted(String),
    Number(u64, String),
    Punct(Conn, char),
}

fn scan(src: &str) -> Vec<Raw> {
    let chars: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '"' || c == '\u{201c}' || c == '\u{201d}' || c == '`' {
            let close = |ch: char| ch == '"' || ch == '\u{201d}' || ch == '\u{201c}' || ch == '`';
            let start = i + 1;
            let mut j = start;
            while j < chars.len() && !close(chars[j]) {
                j += 1;
            }
            out.push(Raw::Quoted(chars[start..j].iter().collect()));
            i = j + 1;
        } else if c.is_ascii_digit() {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            let digits: String = chars[start..i].iter().collect();
            let n = digits.parse().unwrap_or(u64::MAX);
            out.push(Raw::Number(n, digits));
        } else if c.is_alphabetic() || c == '\'' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '\'' || chars[i] == '_') {
                i += 1;
            }
            let original: String = chars[start..i].iter().collect();
            let lower = original.to_lowercase().replace('\u{2019}', "'");
            out.push(Raw::Word(lower, original));
        } else if c == ',' || c == ':' {
            out.push(Raw::Punct(Conn::Comma, c));
            i += 1;
        } else if c == '.' || c == ';' || c == '!' || c == '?' {
            out.push(Raw::Punct(Conn::Period, c));
            i += 1;
        } else {
            // whitespace, hyphens, anything else separates words
            i += 1;
        }
    }
    out
}

/// Sentence -> tokens. Deterministic: a pure function of the input text.
pub fn tokenize(src: &str) -> Vec<Token> {
    let raw = scan(src);
    let mut out: Vec<Token> = Vec::new();
    let mut i = 0;
    while i < raw.len() {
        match &raw[i] {
            Raw::Quoted(s) => {
                out.push(Token { kind: TokKind::Quoted(s.clone()), text: format!("\"{s}\"") });
                i += 1;
            }
            Raw::Punct(conn, ch) => {
                out.push(Token { kind: TokKind::Lex(Lex::Conn(*conn)), text: ch.to_string() });
                i += 1;
            }
            Raw::Number(n, digits) => {
                // "200ms", "200 ms", "2 seconds"
                if let Some(Raw::Word(w, orig)) = raw.get(i + 1) {
                    if let Some((_, mul)) = MILLIS_UNITS.iter().find(|(u, _)| u == w) {
                        out.push(Token {
                            kind: TokKind::Millis(n.saturating_mul(*mul)),
                            text: format!("{digits} {orig}"),
                        });
                        i += 2;
                        continue;
                    }
                }
                out.push(Token { kind: TokKind::Num(*n), text: digits.clone() });
                i += 1;
            }
            Raw::Word(..) => {
                // Longest phrase starting here.
                let mut matched = None;
                for len in (1..=MAX_PHRASE_WORDS).rev() {
                    let Some(words) = words_at(&raw, i, len) else { continue };
                    let phrase = words.iter().map(|(l, _)| l.as_str()).collect::<Vec<_>>().join(" ");
                    if let Some(lex) = lookup(&phrase) {
                        let text = words.iter().map(|(_, o)| o.as_str()).collect::<Vec<_>>().join(" ");
                        matched = Some((lex, len, text));
                        break;
                    }
                }
                if let Some((lex, len, text)) = matched {
                    out.push(Token { kind: TokKind::Lex(lex), text });
                    i += len;
                    continue;
                }
                let Raw::Word(lower, orig) = &raw[i] else { unreachable!() };
                let kind = if let Some((_, n)) = NUMBER_WORDS.iter().find(|(w, _)| w == lower) {
                    // "three seconds"
                    match raw.get(i + 1) {
                        Some(Raw::Word(u, _)) if MILLIS_UNITS.iter().any(|(m, _)| m == u) => {
                            let mul = MILLIS_UNITS.iter().find(|(m, _)| m == u).unwrap().1;
                            i += 1;
                            TokKind::Millis(n * mul)
                        }
                        _ => TokKind::Num(*n),
                    }
                } else if let Some(lex) = stem(lower).and_then(|s| lookup(&s)) {
                    TokKind::Lex(lex)
                } else {
                    TokKind::Unknown
                };
                out.push(Token { kind, text: orig.clone() });
                i += 1;
            }
        }
    }
    out
}

fn words_at(raw: &[Raw], start: usize, len: usize) -> Option<Vec<(String, String)>> {
    let mut words = Vec::with_capacity(len);
    for r in raw.get(start..start + len)? {
        let Raw::Word(l, o) = r else { return None };
        // allow inflected verbs as the first word of a phrase ("skipping blank lines")
        let l = if words.is_empty() && !is_known_word(l) {
            stem(l).unwrap_or_else(|| l.clone())
        } else {
            l.clone()
        };
        words.push((l, o.clone()));
    }
    Some(words)
}

/// Every single word the lexicon knows, for "did you mean" suggestions.
pub fn vocabulary() -> Vec<&'static str> {
    let mut v: Vec<&'static str> = PHRASES
        .iter()
        .filter(|(_, l)| !matches!(l, Lex::Filler))
        .map(|(p, _)| *p)
        .collect();
    v.sort_unstable();
    v.dedup();
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(s: &str) -> Vec<TokKind> {
        tokenize(s).into_iter().map(|t| t.kind).collect()
    }

    #[test]
    fn phrases_are_unique() {
        let mut seen: Vec<&str> = PHRASES.iter().map(|(p, _)| *p).collect();
        seen.sort_unstable();
        let before = seen.len();
        seen.dedup();
        assert_eq!(before, seen.len());
    }

    #[test]
    fn longest_phrase_wins() {
        assert_eq!(
            kinds("skip blank lines"),
            vec![TokKind::Lex(Lex::Verb(Verb::SkipBlank))]
        );
        assert_eq!(
            kinds("if it fails"),
            vec![TokKind::Lex(Lex::Conn(Conn::IfFails))]
        );
    }

    #[test]
    fn numbers_durations_and_quotes() {
        assert_eq!(kinds("three"), vec![TokKind::Num(3)]);
        assert_eq!(kinds("200ms"), vec![TokKind::Millis(200)]);
        assert_eq!(kinds("2 seconds"), vec![TokKind::Millis(2000)]);
        assert_eq!(kinds("\"a.txt\""), vec![TokKind::Quoted("a.txt".into())]);
    }

    #[test]
    fn inflections_reach_the_base_verb() {
        assert_eq!(kinds("reading"), vec![TokKind::Lex(Lex::Verb(Verb::Read))]);
        assert_eq!(kinds("parsing"), vec![TokKind::Lex(Lex::Verb(Verb::Parse))]);
        assert_eq!(kinds("logging"), vec![TokKind::Lex(Lex::Verb(Verb::Log))]);
        assert_eq!(kinds("retries"), vec![TokKind::Lex(Lex::Verb(Verb::Retry))]);
    }

    #[test]
    fn unknown_words_stay_unknown() {
        assert_eq!(kinds("frobnicate"), vec![TokKind::Unknown]);
    }
}
