//! Senne: descriptive English in, deterministic Rust out.
//!
//! sentence -> lexicon (tokens) -> parser (clause tree) -> search (beam over
//! rule choices) -> codegen (Rust source). No model in the loop: the same
//! sentence and the same rules.toml always give byte-identical output.

pub mod codegen;
pub mod config;
pub mod diagnose;
pub mod explain;
pub mod lexicon;
pub mod model;
pub mod parser;
pub mod rules;
pub mod search;

use config::Config;
use parser::{Diag, Program};
use search::Outcome;

pub struct Options {
    /// Ignore words the lexicon doesn't know instead of failing.
    pub lenient: bool,
    /// Per-statement comments naming the words and the rule.
    pub comments: bool,
}

impl Default for Options {
    fn default() -> Options {
        Options { lenient: false, comments: true }
    }
}

pub struct Compiled {
    pub program: Program,
    pub outcome: Outcome,
    pub code: String,
}

pub fn compile(sentence: &str, cfg: &Config, opts: &Options) -> Result<Compiled, Vec<Diag>> {
    let tokens = lexicon::tokenize(sentence);
    let program = parser::parse(sentence, tokens, opts.lenient)?;
    let outcome = search::search(&program, cfg)?;
    let code = codegen::generate(
        &program,
        &outcome.choices,
        cfg,
        &codegen::GenOptions { comments: opts.comments },
    )?;
    Ok(Compiled { program, outcome, code })
}
