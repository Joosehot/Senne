//! Emits the final Rust source from the winning choices. Replaces
//! FeelRight's MIDI renderer: same input, byte-identical output, and every
//! statement carries a comment naming the words and the rule behind it.

use crate::config::Config;
use crate::lexicon::Verb;
use crate::model::{Scalar, Ty, Value};
use crate::parser::{Clause, Diag, Program};
use crate::rules::{self, error_strategy, Choice, Out};
use std::collections::{BTreeMap, BTreeSet};

const INDENT: &str = "    ";

pub struct Gen<'a> {
    pub cfg: &'a Config,
    choices: &'a BTreeMap<usize, Choice>,
    lines: Vec<(usize, String)>,
    depth: usize,
    imports: BTreeSet<String>,
    params: Vec<(String, String)>,
    taken: BTreeSet<String>,
    helpers: BTreeMap<String, String>,
    version: u32,
    comments: bool,
    /// Some clause uses `?` or `return Err`.
    pub returns_result: bool,
    pub in_loop: bool,
    pub in_worker: bool,
    /// Problems only visible once code exists (a value nobody uses).
    diags: Vec<Diag>,
}

/// Does `text` use the identifier `ident`?
pub fn mentions(text: &str, ident: &str) -> bool {
    text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_')).any(|w| w == ident)
}

impl<'a> Gen<'a> {
    fn new(cfg: &'a Config, choices: &'a BTreeMap<usize, Choice>, comments: bool) -> Gen<'a> {
        let taken = ["value", "err", "attempt", "s", "chunk", "handles", "handle", "out", "item", "entry", "e", "p", "l", "text", "file", "entries"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        Gen {
            cfg,
            choices,
            lines: Vec::new(),
            depth: 0,
            imports: BTreeSet::new(),
            params: Vec::new(),
            taken,
            helpers: BTreeMap::new(),
            version: 0,
            comments,
            returns_result: false,
            in_loop: false,
            in_worker: false,
            diags: Vec::new(),
        }
    }

    pub fn choice(&self, id: usize) -> &Choice {
        &self.choices[&id]
    }

    /// One or more lines at the current depth.
    pub fn line(&mut self, s: impl AsRef<str>) {
        for l in s.as_ref().lines() {
            self.lines.push((self.depth, l.to_string()));
        }
    }
    pub fn open(&mut self, s: impl AsRef<str>) {
        self.line(s);
        self.depth += 1;
    }
    pub fn close(&mut self, s: impl AsRef<str>) {
        self.depth -= 1;
        self.line(s);
    }
    pub fn indent(&mut self) {
        self.depth += 1;
    }
    pub fn dedent(&mut self) {
        self.depth -= 1;
    }

    pub fn import(&mut self, path: &str) {
        self.imports.insert(path.to_string());
    }

    pub fn add_helper(&mut self, name: &str, code: String) {
        self.helpers.entry(name.to_string()).or_insert(code);
    }

    /// A variable name not used yet: `lines`, `lines2`, ...
    pub fn fresh(&mut self, base: &str) -> String {
        let mut name = base.to_string();
        let mut n = 2;
        while self.taken.contains(&name) {
            name = format!("{base}{n}");
            n += 1;
        }
        self.taken.insert(name.clone());
        name
    }

    /// A parameter of `run()`, e.g. an unnamed path.
    pub fn param(&mut self, base: &str, ty: &str) -> String {
        let name = self.fresh(base);
        self.params.push((name.clone(), ty.to_string()));
        name
    }

    /// Register a new value (bumps the version so loops notice it).
    pub fn value(&mut self, name: String, ty: Ty, borrowed: bool) -> Value {
        self.version += 1;
        Value { name, ty, borrowed, consumed: false, version: self.version, mutable: false }
    }

    /// Rebind `v` as `let mut` unless it already is; returns the mutable value.
    pub fn make_mut(&mut self, v: &Value) -> Value {
        if !v.mutable {
            self.line(format!("let mut {0} = {0};", v.name));
        }
        let mut out = self.value(v.name.clone(), v.ty, false);
        out.mutable = true;
        out
    }

    /// Rust type text, importing what it needs.
    pub fn rust_ty(&mut self, ty: Ty) -> String {
        if matches!(ty, Ty::One(Scalar::Path) | Ty::Many(Scalar::Path)) {
            self.import("std::path::PathBuf");
        }
        ty.rust()
    }

    /// Run `f` into a separate buffer; returns its lines (depth relative to
    /// now) and its result.
    pub fn capture<R>(&mut self, f: impl FnOnce(&mut Gen<'a>) -> R) -> (Vec<(usize, String)>, R) {
        let saved = std::mem::take(&mut self.lines);
        let saved_depth = self.depth;
        self.depth = 0;
        let r = f(self);
        self.depth = saved_depth;
        let captured = std::mem::replace(&mut self.lines, saved);
        (captured, r)
    }

    /// Append captured lines under the current depth.
    pub fn splice(&mut self, lines: Vec<(usize, String)>) {
        for (d, l) in lines {
            self.lines.push((self.depth + d, l));
        }
    }

    /// Emit a list of clauses; returns the value left in "it".
    pub fn gen_block(&mut self, clauses: &[Clause], mut cur: Value) -> Value {
        // Words that made the current value, for "never used" errors.
        let mut made_by = String::from("each loop item");
        for c in clauses {
            let choice = self.choices[&c.id].clone();
            if self.comments {
                self.line(format!("// {}  [{}]", c.words, choice.label()));
            }
            let start = self.lines.len();
            let p = rules::pattern(choice.pattern);
            let next = match p.emit(choice.variant, c, self, &cur) {
                Out::Done(v) => v,
                Out::Fallible(f) => error_strategy::bind(self, c, &choice, f, &cur),
            };
            if cur.ty != Ty::Unit && !cur.consumed && next.version != cur.version && !self.used_since(start, &cur.name) {
                self.diags.push(
                    Diag::new(format!("the result of \"{made_by}\" is never used (\"{}\" doesn't use it)", c.words))
                        .hint("print it, write it, or drop that step"),
                );
            }
            if next.version != cur.version {
                made_by = c.words.clone();
            }
            cur = next;
        }
        cur
    }

    /// Was `name` used by any line emitted since `start`?
    pub fn used_since(&self, start: usize, name: &str) -> bool {
        self.lines[start..].iter().any(|(_, l)| !l.starts_with("//") && mentions(l, name))
    }

    fn render_body(&self) -> String {
        let mut s = String::new();
        for (d, l) in &self.lines {
            if l.is_empty() {
                s.push('\n');
            } else {
                s.push_str(&INDENT.repeat(d + 1));
                s.push_str(l);
                s.push('\n');
            }
        }
        s
    }
}

pub struct GenOptions {
    /// Per-statement comments naming the words and rule.
    pub comments: bool,
}

fn header(prog: &Program) -> String {
    let mut out = format!("// Generated by Senne {} from:\n", env!("CARGO_PKG_VERSION"));
    let mut line = String::from("//  ");
    for word in prog.sentence.split_whitespace() {
        if line.len() + word.len() + 1 > 90 {
            out.push_str(&line);
            out.push('\n');
            line = String::from("//  ");
        }
        line.push(' ');
        line.push_str(word);
    }
    out.push_str(&line);
    out.push_str("\n// Same sentence in, same code out. Change the sentence, not this file.\n");
    out
}

pub fn generate(
    prog: &Program,
    choices: &BTreeMap<usize, Choice>,
    cfg: &Config,
    opts: &GenOptions,
) -> Result<String, Vec<Diag>> {
    let mut g = Gen::new(cfg, choices, opts.comments);
    let standalone = prog.clauses.len() == 1 && prog.clauses[0].verb == Verb::Retry;
    let result = g.gen_block(&prog.clauses, Value::unit());
    if !g.diags.is_empty() {
        return Err(g.diags);
    }

    let mut out = header(prog);

    let mut run = String::new();
    if !standalone {
        let ret = (!result.consumed && result.ty != Ty::Unit).then_some(&result);
        let ret_ty = ret.map(|v| g.rust_ty(v.ty));
        if g.returns_result {
            g.import("std::error::Error");
        }
        let params: Vec<String> = g.params.iter().map(|(n, t)| format!("{n}: {t}")).collect();
        let sig_ret = match (&ret_ty, g.returns_result) {
            (Some(t), true) => format!(" -> Result<{t}, Box<dyn Error>>"),
            (None, true) => " -> Result<(), Box<dyn Error>>".to_string(),
            (Some(t), false) => format!(" -> {t}"),
            (None, false) => String::new(),
        };
        run.push_str(&format!("pub fn run({}){sig_ret} {{\n", params.join(", ")));
        run.push_str(&g.render_body());
        let tail = ret.map(|v| v.owned());
        match (tail, g.returns_result) {
            (Some(t), true) => run.push_str(&format!("{INDENT}Ok({t})\n")),
            (None, true) => run.push_str(&format!("{INDENT}Ok(())\n")),
            (Some(t), false) => run.push_str(&format!("{INDENT}{t}\n")),
            (None, false) => {}
        }
        run.push_str("}\n");
    }

    if !g.imports.is_empty() {
        out.push('\n');
        for i in &g.imports {
            out.push_str(&format!("use {i};\n"));
        }
    }
    if !run.is_empty() {
        out.push('\n');
        out.push_str(&run);
    }
    for code in g.helpers.values() {
        out.push('\n');
        out.push_str(code);
        out.push('\n');
    }
    Ok(out)
}
