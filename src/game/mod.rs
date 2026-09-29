//! `senne::game`: game sentences in, Rust against Droid's `Ctx` API out.
//!
//! Same shape as the Rust engine and a separate module, so its patterns are
//! untouched: lexicon (closed vocabulary) -> parser (typed sentences) ->
//! search (beam over code-level variants, judges across the program) ->
//! codegen (one `main.rs` shaped like Droid's coin_runner). No model runs;
//! the same sentences and the same rules.toml give byte-identical output.

pub mod codegen;
pub mod explain;
pub mod lexicon;
pub mod parser;
pub mod rules;
pub mod search;

use crate::config::Config;
use crate::parser::Diag;

pub struct GameOptions {
    /// Name of the generated game type (CamelCase).
    pub struct_name: String,
    /// Skip unknown words instead of failing.
    pub lenient: bool,
    /// Comment every block with its sentence and [pattern/variant].
    pub comments: bool,
}

impl Default for GameOptions {
    fn default() -> GameOptions {
        GameOptions { struct_name: "DroidGame".into(), lenient: false, comments: true }
    }
}

pub struct GameCompiled {
    pub code: String,
    /// What --explain prints: every candidate weighed, judges and totals.
    pub explain: String,
}

/// Names the generated file already uses.
const RESERVED: &[&str] = &["Game", "Ctx", "Status", "Vec3", "BodyKind", "BTreeMap", "Default", "Self"];

fn check_name(name: &str) -> Result<(), Diag> {
    let ok = name.chars().next().is_some_and(|c| c.is_ascii_uppercase())
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    if !ok {
        return Err(Diag::new(format!("\"{name}\" is not a type name")).hint("use CamelCase, like CoinRunner"));
    }
    if RESERVED.contains(&name) {
        return Err(Diag::new(format!("\"{name}\" clashes with a name the game code uses")).hint("pick another, like MyGame"));
    }
    Ok(())
}

pub fn compile(text: &str, cfg: &Config, opts: &GameOptions) -> Result<GameCompiled, Vec<Diag>> {
    check_name(&opts.struct_name).map_err(|d| vec![d])?;
    let program = parser::parse(text, opts.lenient)?;
    let outcome = search::search(&program, cfg)?;
    let code = codegen::generate(
        &program,
        &outcome,
        cfg,
        &codegen::GenOptions { struct_name: opts.struct_name.clone(), comments: opts.comments },
    );
    let explain = explain::explain(&program, &outcome, cfg);
    Ok(GameCompiled { code, explain })
}

/// Every word the game language knows.
pub fn vocabulary() -> Vec<&'static str> {
    lexicon::vocabulary()
}
