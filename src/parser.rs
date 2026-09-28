//! Tokens -> a tree of clauses.
//!
//! Grammar (informal, and deliberately small):
//! - every verb starts a clause; nouns, numbers, quoted text and modifiers
//!   attach to the clause they appear in (or the one before, if they stand
//!   alone: "read the file, safely")
//! - "retry N times" folds into the clause it's glued to ("retry reading
//!   ...") or else the one before it; alone it asks for a retry helper
//! - "if it fails" opens an error handler for the previous clause; "then",
//!   "after that" or "." close it
//! - "for each ..." / "spawn N workers" open a loop; "after that", "." or an
//!   aggregate verb (sum, count, sort, dedupe, write) close it

use crate::lexicon::{Conn, Lex, Modifier, Noun, TokKind, Token, Verb};
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub struct RetrySpec {
    pub times: u32,
    pub backoff: bool,
    pub delay_ms: Option<u64>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Fallback {
    /// "or a default": the type's default value.
    Default,
    Num(u64),
    Text(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Clause {
    /// Preorder index; the search keys its choices by it.
    pub id: usize,
    pub verb: Verb,
    pub nouns: Vec<Noun>,
    pub mods: Vec<Modifier>,
    pub num: Option<u64>,
    pub millis: Option<u64>,
    /// Quoted text or an ALL_CAPS word: a path, a variable name, a message.
    pub literal: Option<String>,
    pub fallback: Option<Fallback>,
    pub retry: Option<RetrySpec>,
    /// "if it fails, ..." clauses.
    pub on_error: Vec<Clause>,
    /// Loop body for for-each / workers.
    pub body: Vec<Clause>,
    /// Source words, for explanations and comments.
    pub words: String,
}

impl Clause {
    pub fn has(&self, n: Noun) -> bool {
        self.nouns.contains(&n)
    }
    pub fn has_mod(&self, m: Modifier) -> bool {
        self.mods.contains(&m)
    }
    pub fn is_loop(&self) -> bool {
        matches!(self.verb, Verb::ForEach | Verb::Spawn)
    }
}

#[derive(Clone, Debug)]
pub struct Program {
    pub sentence: String,
    pub tokens: Vec<Token>,
    pub clauses: Vec<Clause>,
}

impl Program {
    /// All clauses in preorder (body and handlers included).
    pub fn walk(&self) -> Vec<&Clause> {
        fn go<'a>(cs: &'a [Clause], out: &mut Vec<&'a Clause>) {
            for c in cs {
                out.push(c);
                go(&c.on_error, out);
                go(&c.body, out);
            }
        }
        let mut out = Vec::new();
        go(&self.clauses, &mut out);
        out
    }
}

/// A problem with the sentence. The engine never guesses past one.
#[derive(Clone, Debug, PartialEq)]
pub struct Diag {
    pub message: String,
    pub hint: Option<String>,
}

impl Diag {
    pub fn new(message: impl Into<String>) -> Diag {
        Diag { message: message.into(), hint: None }
    }
    pub fn hint(mut self, hint: impl Into<String>) -> Diag {
        self.hint = Some(hint.into());
        self
    }
}

impl fmt::Display for Diag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "error: {}", self.message)?;
        if let Some(h) = &self.hint {
            write!(f, "\n  hint: {h}")?;
        }
        Ok(())
    }
}

/// A clause before structure is known.
#[derive(Clone, Debug, Default)]
struct Raw {
    verb: Option<Verb>,
    nouns: Vec<Noun>,
    mods: Vec<Modifier>,
    nums: Vec<u64>,
    millis: Option<u64>,
    literals: Vec<String>,
    fallback: Option<Fallback>,
    pending_fallback: bool,
    words: Vec<String>,
    /// Directly follows another clause's verb with no connective between.
    glued: bool,
    retry: Option<RetrySpec>,
}

impl Raw {
    fn is_empty(&self) -> bool {
        self.verb.is_none()
            && self.nouns.is_empty()
            && self.mods.is_empty()
            && self.nums.is_empty()
            && self.millis.is_none()
            && self.literals.is_empty()
            && self.fallback.is_none()
    }

    fn absorb(&mut self, other: Raw) {
        self.nouns.extend(other.nouns);
        self.mods.extend(other.mods);
        self.nums.extend(other.nums);
        self.millis = self.millis.or(other.millis);
        self.literals.extend(other.literals);
        self.fallback = self.fallback.take().or(other.fallback);
        self.words.extend(other.words);
    }
}

#[derive(Debug)]
enum Item {
    B(Conn),
    C(Raw),
}

pub fn parse(sentence: &str, tokens: Vec<Token>, lenient: bool) -> Result<Program, Vec<Diag>> {
    let mut diags = Vec::new();
    let items = segment(&tokens, lenient, &mut diags);
    let items = merge_verbless(items);
    let items = fold_retries(items, &mut diags);
    let mut clauses = structure(items, &mut diags);
    if clauses.is_empty() && diags.is_empty() {
        diags.push(
            Diag::new("the sentence has no action in it")
                .hint("start with a verb: read, write, parse, retry, sort, print, ..."),
        );
    }
    let mut next = 0;
    number(&mut clauses, &mut next);
    if diags.is_empty() {
        Ok(Program { sentence: sentence.trim().to_string(), tokens, clauses })
    } else {
        Err(diags)
    }
}

fn number(cs: &mut [Clause], next: &mut usize) {
    for c in cs {
        c.id = *next;
        *next += 1;
        number(&mut c.on_error, next);
        number(&mut c.body, next);
    }
}

fn is_env_name(s: &str) -> bool {
    s.len() > 1
        && s.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
        && s.chars().any(|c| c.is_ascii_uppercase())
}

fn segment(tokens: &[Token], lenient: bool, diags: &mut Vec<Diag>) -> Vec<Item> {
    let mut items = Vec::new();
    let mut cur = Raw::default();
    let mut glue_next = false;
    let flush = |cur: &mut Raw, items: &mut Vec<Item>| {
        if cur.pending_fallback && cur.fallback.is_none() {
            cur.fallback = Some(Fallback::Default);
        }
        let done = std::mem::take(cur);
        if !done.is_empty() {
            items.push(Item::C(done));
        }
    };
    for t in tokens {
        match &t.kind {
            TokKind::Lex(Lex::Conn(Conn::Fallback)) => {
                cur.pending_fallback = true;
                cur.words.push(t.text.clone());
            }
            TokKind::Lex(Lex::Conn(c)) => {
                flush(&mut cur, &mut items);
                items.push(Item::B(*c));
                glue_next = false;
            }
            TokKind::Lex(Lex::Verb(v)) => {
                // leading nouns/mods ("safely read ...") stay with this verb
                if cur.verb.is_some() {
                    flush(&mut cur, &mut items);
                    glue_next = true;
                }
                cur.verb = Some(*v);
                cur.glued = glue_next;
                glue_next = false;
                cur.words.push(t.text.clone());
            }
            TokKind::Lex(Lex::Noun(n)) => {
                cur.nouns.push(*n);
                cur.words.push(t.text.clone());
            }
            TokKind::Lex(Lex::Mod(m)) => {
                cur.mods.push(*m);
                cur.words.push(t.text.clone());
            }
            TokKind::Lex(Lex::Filler) => cur.words.push(t.text.clone()),
            TokKind::Num(n) => {
                if cur.pending_fallback && cur.fallback.is_none() {
                    cur.fallback = Some(Fallback::Num(*n));
                } else {
                    cur.nums.push(*n);
                }
                cur.words.push(t.text.clone());
            }
            TokKind::Millis(ms) => {
                cur.millis = Some(*ms);
                cur.words.push(t.text.clone());
            }
            TokKind::Quoted(s) => {
                if cur.pending_fallback && cur.fallback.is_none() {
                    cur.fallback = Some(Fallback::Text(s.clone()));
                } else {
                    cur.literals.push(s.clone());
                }
                cur.words.push(t.text.clone());
            }
            TokKind::Unknown => {
                if is_env_name(&t.text) {
                    cur.literals.push(t.text.clone());
                    cur.words.push(t.text.clone());
                } else if !lenient {
                    diags.push(crate::diagnose::unknown_word(&t.text));
                }
            }
        }
    }
    flush(&mut cur, &mut items);
    items
}

/// Stand-alone modifiers/values ("..., safely") join the clause before them,
/// or the one after if nothing came before.
fn merge_verbless(items: Vec<Item>) -> Vec<Item> {
    let mut out: Vec<Item> = Vec::new();
    let mut carry: Option<Raw> = None;
    for item in items {
        match item {
            Item::C(r) if r.verb.is_none() => {
                if let Some(prev) = out.iter_mut().rev().find_map(|i| match i {
                    Item::C(p) => Some(p),
                    Item::B(_) => None,
                }) {
                    prev.absorb(r);
                } else if let Some(c) = carry.as_mut() {
                    c.absorb(r);
                } else {
                    carry = Some(r);
                }
            }
            Item::C(mut r) => {
                if let Some(c) = carry.take() {
                    r.absorb(c);
                }
                out.push(Item::C(r));
            }
            b => out.push(b),
        }
    }
    out
}

fn fold_retries(mut items: Vec<Item>, diags: &mut Vec<Diag>) -> Vec<Item> {
    loop {
        let Some(k) = items.iter().position(|i| {
            matches!(i, Item::C(r) if r.verb == Some(Verb::Retry))
        }) else {
            break;
        };
        let Item::C(retry) = items.remove(k) else { unreachable!() };
        let mut spec = RetrySpec {
            times: retry.nums.first().map(|n| *n as u32).unwrap_or(0),
            backoff: retry.mods.contains(&Modifier::Backoff),
            delay_ms: retry.millis,
        };
        // "..., waiting 200ms between attempts"
        let mut j = k;
        while matches!(items.get(j), Some(Item::B(Conn::Comma | Conn::And))) {
            j += 1;
        }
        if let Some(Item::C(w)) = items.get(j) {
            if w.verb == Some(Verb::Wait) && w.millis.is_some() && !w.glued {
                spec.delay_ms = w.millis;
                items.drain(k..=j);
            }
        }
        let glued_next = matches!(items.get(k), Some(Item::C(r)) if r.glued);
        let target = if glued_next {
            Some(k)
        } else {
            (0..k).rev().find(|&i| matches!(items[i], Item::C(_)))
        };
        let extra_mods: Vec<Modifier> =
            retry.mods.iter().copied().filter(|m| *m != Modifier::Backoff).collect();
        match target {
            Some(t) => {
                let Item::C(r) = &mut items[t] else { unreachable!() };
                // "retry reading the file 3 times with backoff": the count and
                // the backoff landed on the read, which has no use for them.
                let target_is_loop = matches!(r.verb, Some(Verb::ForEach | Verb::Spawn));
                if spec.times == 0 && !target_is_loop && !r.nums.is_empty() {
                    spec.times = r.nums.remove(0) as u32;
                }
                if let Some(i) = r.mods.iter().position(|m| *m == Modifier::Backoff) {
                    r.mods.remove(i);
                    spec.backoff = true;
                }
                if spec.delay_ms.is_none() && r.verb != Some(Verb::Wait) {
                    spec.delay_ms = r.millis.take();
                }
                if r.retry.is_some() {
                    diags.push(Diag::new("two retries for the same step"));
                }
                r.retry = Some(spec);
                r.mods.extend(extra_mods);
                if t < k {
                    r.words.extend(retry.words);
                } else {
                    r.words.splice(0..0, retry.words);
                }
            }
            None => {
                // "retry this safely three times": emit a helper. Re-insert it
                // as a pattern clause of its own (verb stays Retry).
                let mut r = retry;
                r.retry = Some(spec);
                r.verb = Some(Verb::Retry);
                items.insert(k, Item::C(r));
                // It's already folded; mark by moving it past this loop.
                return finish_standalone(items, k, diags);
            }
        }
    }
    items
}

/// A stand-alone retry must be the only clause.
fn finish_standalone(items: Vec<Item>, k: usize, diags: &mut Vec<Diag>) -> Vec<Item> {
    let clauses = items.iter().filter(|i| matches!(i, Item::C(_))).count();
    if clauses > 1 {
        let _ = k;
        diags.push(
            Diag::new("\"retry\" doesn't say what to retry")
                .hint("put it next to the step: \"retry reading the file 3 times\""),
        );
    }
    items
}

struct Scope {
    clauses: Vec<Clause>,
}

fn to_clause(r: Raw, diags: &mut Vec<Diag>) -> Clause {
    let words = r.words.join(" ");
    let mut nouns = r.nouns;
    nouns.dedup();
    let mut mods = r.mods;
    mods.sort();
    mods.dedup();
    let mut verb = r.verb.expect("verbless raws were merged");
    if verb == Verb::ForEach && (mods.contains(&Modifier::Parallel) || nouns.contains(&Noun::Workers)) {
        verb = Verb::Spawn;
    }
    if verb == Verb::Spawn && !mods.contains(&Modifier::Parallel) {
        mods.push(Modifier::Parallel);
        mods.sort();
    }
    if r.literals.len() > 1 {
        diags.push(Diag::new(format!("\"{words}\" names more than one thing: {:?}", r.literals)));
    }
    if r.nums.len() > 1 {
        diags.push(Diag::new(format!("\"{words}\" has more than one number: {:?}", r.nums)));
    }
    Clause {
        id: 0,
        verb,
        nouns,
        mods,
        num: r.nums.first().copied(),
        millis: r.millis,
        literal: r.literals.into_iter().next(),
        fallback: r.fallback,
        retry: r.retry,
        on_error: Vec::new(),
        body: Vec::new(),
        words,
    }
}

fn structure(items: Vec<Item>, diags: &mut Vec<Diag>) -> Vec<Clause> {
    let mut stack: Vec<Scope> = vec![Scope { clauses: Vec::new() }];
    let mut handler = false;

    fn close_loop(stack: &mut Vec<Scope>) {
        let inner = stack.pop().expect("loop scope");
        let owner = stack
            .last_mut()
            .and_then(|s| s.clauses.last_mut())
            .expect("loop owner");
        owner.body = inner.clauses;
    }

    for item in items {
        match item {
            Item::B(Conn::IfFails) => {
                let has_anchor = stack.last().is_some_and(|s| !s.clauses.is_empty());
                if !has_anchor {
                    diags.push(
                        Diag::new("\"if it fails\" has nothing before it that could fail")
                            .hint("name the step first: \"read the file, and if it fails ...\""),
                    );
                }
                handler = true;
            }
            Item::B(Conn::Then) => handler = false,
            Item::B(Conn::After) | Item::B(Conn::Period) => {
                handler = false;
                while stack.len() > 1 {
                    close_loop(&mut stack);
                }
            }
            Item::B(_) => {}
            Item::C(raw) => {
                let c = to_clause(raw, diags);
                if handler {
                    if let Some(anchor) = stack.last_mut().and_then(|s| s.clauses.last_mut()) {
                        anchor.on_error.push(c);
                    }
                    continue;
                }
                if c.verb.is_aggregate() && stack.len() > 1 {
                    close_loop(&mut stack);
                }
                // "spawn 4 workers, each ..." : the "each" is the same loop
                if c.verb == Verb::ForEach && stack.len() > 1 {
                    let top_empty = stack.last().is_some_and(|s| s.clauses.is_empty());
                    let owner_is_spawn = stack[stack.len() - 2]
                        .clauses
                        .last()
                        .is_some_and(|o| o.verb == Verb::Spawn);
                    if top_empty && owner_is_spawn {
                        let n = stack.len();
                        let owner = stack[n - 2].clauses.last_mut().unwrap();
                        owner.nouns.extend(c.nouns);
                        owner.words = format!("{}, {}", owner.words, c.words);
                        continue;
                    }
                }
                match c.verb {
                    Verb::Continue | Verb::Stop => diags.push(
                        Diag::new(format!("\"{}\" only makes sense after \"if it fails\"", c.words))
                            .hint("e.g. \"parse it, and if it fails, log the error and continue\""),
                    ),
                    _ => {}
                }
                let opens = c.is_loop();
                stack.last_mut().unwrap().clauses.push(c);
                if opens {
                    stack.push(Scope { clauses: Vec::new() });
                }
            }
        }
    }
    while stack.len() > 1 {
        close_loop(&mut stack);
    }
    stack.pop().unwrap().clauses
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexicon::tokenize;

    fn p(s: &str) -> Program {
        parse(s, tokenize(s), false).unwrap_or_else(|d| panic!("{d:?}"))
    }

    #[test]
    fn handler_attaches_to_previous_clause() {
        let prog = p("read the file, and if it fails, log the error and continue");
        assert_eq!(prog.clauses.len(), 1);
        let read = &prog.clauses[0];
        assert_eq!(read.verb, Verb::Read);
        let verbs: Vec<Verb> = read.on_error.iter().map(|c| c.verb).collect();
        assert_eq!(verbs, vec![Verb::Log, Verb::Continue]);
    }

    #[test]
    fn retry_folds_into_glued_clause() {
        let prog = p("retry reading the file \"a.txt\" 3 times with backoff");
        assert_eq!(prog.clauses.len(), 1);
        let r = prog.clauses[0].retry.as_ref().unwrap();
        assert_eq!(r.times, 3);
        assert!(r.backoff);
        assert_eq!(prog.clauses[0].num, None);
    }

    #[test]
    fn retry_folds_into_previous_clause() {
        let prog = p("read \"a.txt\", retry 5 times, waiting 200ms between attempts");
        assert_eq!(prog.clauses.len(), 1);
        let r = prog.clauses[0].retry.as_ref().unwrap();
        assert_eq!(r.times, 5);
        assert_eq!(r.delay_ms, Some(200));
    }

    #[test]
    fn loop_closes_on_aggregate() {
        let prog = p("read the lines of \"n.txt\", for each line parse it, then sum them");
        let verbs: Vec<Verb> = prog.clauses.iter().map(|c| c.verb).collect();
        assert_eq!(verbs, vec![Verb::Read, Verb::ForEach, Verb::Sum]);
        assert_eq!(prog.clauses[1].body.len(), 1);
    }

    #[test]
    fn continue_outside_handler_is_an_error() {
        let s = "read the file and continue";
        assert!(parse(s, tokenize(s), false).is_err());
    }

    #[test]
    fn standalone_retry() {
        let prog = p("retry this safely three times");
        assert_eq!(prog.clauses.len(), 1);
        assert_eq!(prog.clauses[0].verb, Verb::Retry);
        assert_eq!(prog.clauses[0].retry.as_ref().unwrap().times, 3);
    }
}
