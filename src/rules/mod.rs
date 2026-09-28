//! The rule system. Two kinds of rules, both reading their numbers from
//! `rules.toml`:
//!
//! - **patterns** (one file each): recognize a clause, offer implementation
//!   variants that fit the current value type, and emit Rust for the chosen one
//! - **judges**: score choices across the whole program (no panics,
//!   consistent error style, helper reuse, Result signature)
//!
//! Fallible patterns don't decide how errors are handled; the search pairs
//! them with an error strategy (`error_strategy.rs`) and, when the sentence
//! asks for it, a retry style (`retry.rs`).

use crate::codegen::Gen;
use crate::config::{Axes, JudgeConfig};
use crate::model::{Ty, Value};
use crate::parser::Clause;

pub mod count;
pub mod create_dir;
pub mod dedupe;
pub mod env_var;
pub mod error_strategy;
pub mod for_each;
pub mod judges;
pub mod list_dir;
pub mod log;
pub mod parallel;
pub mod parse_number;
pub mod print;
pub mod read_file;
pub mod read_lines;
pub mod retry;
pub mod retry_helper;
pub mod skip_blank;
pub mod sort;
pub mod sum;
pub mod trim;
pub mod wait;
pub mod write_file;

pub use error_strategy::{Strategy, STRATEGIES};
pub use retry::{RetryStyle, RETRY_STYLES};

/// What the search knows at a clause.
#[derive(Clone, Debug, PartialEq)]
pub struct Env {
    /// Type of the current value ("it").
    pub ty: Ty,
    /// The current value is known to be sorted.
    pub sorted: bool,
    pub in_loop: bool,
    pub in_worker: bool,
}

impl Env {
    pub fn start() -> Env {
        Env { ty: Ty::Unit, sorted: false, in_loop: false, in_worker: false }
    }
}

/// One way to implement a clause.
#[derive(Clone, Debug, PartialEq)]
pub struct Variant {
    pub name: &'static str,
    /// Type of "it" afterwards.
    pub output: Ty,
    /// Makes a new value (false for sinks and in-place checks).
    pub produces: bool,
    /// Returns a Result the search must pair with an error strategy.
    pub fallible: bool,
    /// Leaves the value sorted.
    pub sorted: bool,
}

impl Variant {
    pub fn new(name: &'static str, output: Ty) -> Variant {
        Variant { name, output, produces: true, fallible: false, sorted: false }
    }
    pub fn fallible(mut self) -> Variant {
        self.fallible = true;
        self
    }
    pub fn sink(mut self) -> Variant {
        self.produces = false;
        self
    }
    pub fn sorted(mut self) -> Variant {
        self.sorted = true;
        self
    }
}

/// An error message: a format string plus its positional arguments.
#[derive(Clone, Debug, PartialEq)]
pub struct Msg {
    pub fmt: String,
    pub args: Vec<String>,
}

impl Msg {
    pub fn new(fmt: impl Into<String>, args: Vec<String>) -> Msg {
        Msg { fmt: fmt.into(), args }
    }
}

/// A `Result`-typed expression, handed to the error strategy.
#[derive(Clone, Debug, PartialEq)]
pub struct Fallible {
    pub expr: String,
    pub ok: Ty,
    pub msg: Msg,
    /// Preferred variable name (defaults to one derived from the type).
    pub name: Option<String>,
}

pub enum Out {
    Fallible(Fallible),
    /// The pattern emitted its statements; this is the new current value.
    Done(Value),
}

pub trait Pattern: Send + Sync {
    /// Key in `rules.toml` (`[rules.<name>]`).
    fn name(&self) -> &'static str;
    /// Every variant this pattern can offer (validated against rules.toml).
    fn all_variants(&self) -> &'static [&'static str];
    /// Does this pattern speak to the clause at all?
    fn matches(&self, c: &Clause) -> bool;
    /// Variants that fit here. Empty means "not with this value".
    fn variants(&self, c: &Clause, env: &Env) -> Vec<Variant>;
    /// Emit Rust for the chosen variant.
    fn emit(&self, variant: &str, c: &Clause, g: &mut Gen, input: &Value) -> Out;
}

pub fn registry() -> Vec<&'static dyn Pattern> {
    vec![
        &read_file::ReadFile,
        &read_lines::ReadLines,
        &write_file::WriteFile,
        &env_var::EnvVar,
        &parse_number::ParseNumber,
        &create_dir::CreateDir,
        &list_dir::ListDir,
        &for_each::ForEach,
        &parallel::Parallel,
        &sum::Sum,
        &count::Count,
        &sort::Sort,
        &dedupe::Dedupe,
        &skip_blank::SkipBlank,
        &trim::Trim,
        &print::Print,
        &log::Log,
        &wait::Wait,
        &retry_helper::RetryHelper,
    ]
}

pub fn pattern(name: &str) -> &'static dyn Pattern {
    registry()
        .into_iter()
        .find(|p| p.name() == name)
        .unwrap_or_else(|| panic!("unknown pattern {name}"))
}

/// The search's decision for one clause.
#[derive(Clone, Debug, PartialEq)]
pub struct Choice {
    pub pattern: &'static str,
    pub variant: &'static str,
    pub strategy: Option<Strategy>,
    pub retry: Option<RetryStyle>,
    /// Local score (pattern + strategy + retry + local judges).
    pub score: f32,
    /// Judges that fired on this choice, for --explain.
    pub notes: Vec<String>,
}

impl Choice {
    pub fn label(&self) -> String {
        let mut s = format!("{}/{}", self.pattern, self.variant);
        if let Some(st) = self.strategy {
            s.push_str(" + ");
            s.push_str(st.key());
        }
        if let Some(r) = self.retry {
            s.push_str(" + retry:");
            s.push_str(r.key());
        }
        s
    }
}

/// Counters the global judges look at.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Stats {
    /// Clauses using `?` or `return Err` (forces a Result signature).
    pub propagate: u32,
    /// Clauses using `.expect()`.
    pub expect: u32,
    /// Call sites of the shared retry helper.
    pub helper_uses: u32,
    pub brevity_sum: f32,
    pub clauses: u32,
}

impl Stats {
    pub fn mean_brevity(&self) -> f32 {
        if self.clauses == 0 {
            0.0
        } else {
            self.brevity_sum / self.clauses as f32
        }
    }
}

pub trait Judge: Send + Sync {
    /// Key in `rules.toml` (`[judges.<name>]`).
    fn name(&self) -> &'static str;
    /// Score one clause's choice under its profile.
    fn local(&self, _choice: &Choice, _profile: &Axes, _cfg: &JudgeConfig) -> Option<(f32, String)> {
        None
    }
    /// Score the program so far.
    fn global(&self, _stats: &Stats, _cfg: &JudgeConfig) -> Option<(f32, String)> {
        None
    }
}

pub fn judges() -> Vec<&'static dyn Judge> {
    vec![
        &judges::NoPanic,
        &judges::ResultSignature,
        &judges::ConsistentErrors,
        &judges::HelperReuse,
    ]
}

/// Where a path comes from: a quoted literal, the current value (a path in
/// a loop over files), or a parameter of the generated function.
pub struct PathArg {
    /// Expression usable as `impl AsRef<Path>`.
    pub expr: String,
    /// Expression that is a `&Path` (for `with_extension` etc.).
    pub path_expr: String,
    pub msg: Msg,
}

pub fn path_arg(c: &Clause, input: &Value, g: &mut Gen, param: &str, verb: &str, from_input: bool) -> PathArg {
    use crate::model::{fmt_text, rust_str, Scalar};
    if let Some(lit) = &c.literal {
        return PathArg {
            expr: rust_str(lit),
            path_expr: format!("Path::new({})", rust_str(lit)),
            msg: Msg::new(format!("failed to {verb} {}", fmt_text(lit)), vec![]),
        };
    }
    let name = if from_input && input.ty == Ty::One(Scalar::Path) {
        input.name.clone()
    } else {
        g.import("std::path::Path");
        g.param(param, "&Path")
    };
    PathArg {
        expr: name.clone(),
        path_expr: name.clone(),
        msg: Msg::new(format!("failed to {verb} {{}}"), vec![format!("{name}.display()")]),
    }
}

/// Test helpers shared by the pattern files.
#[cfg(test)]
pub mod test_util {
    use super::*;
    use crate::lexicon::tokenize;
    use crate::parser::parse;

    pub fn clause(s: &str) -> Clause {
        let prog = parse(s, tokenize(s), false).unwrap_or_else(|d| panic!("{d:?}"));
        prog.clauses[0].clone()
    }

    pub fn env(ty: Ty) -> Env {
        Env { ty, ..Env::start() }
    }

    pub fn names(vs: Vec<Variant>) -> Vec<&'static str> {
        vs.into_iter().map(|v| v.name).collect()
    }
}
