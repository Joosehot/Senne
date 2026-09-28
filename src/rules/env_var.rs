//! "read the environment variable PORT or \"8080\"" -> String.

use super::{Env, Fallible, Msg, Out, Pattern, Variant};
use crate::codegen::Gen;
use crate::lexicon::{Noun, Verb};
use crate::model::{fmt_text, rust_str, Scalar, Ty, Value};
use crate::parser::Clause;

pub struct EnvVar;

/// PORT -> port, LOG_LEVEL -> log_level (if that makes a plain identifier).
fn ident(var: &str) -> Option<String> {
    let lower = var.to_ascii_lowercase();
    let ok = lower.chars().next().is_some_and(|c| c.is_ascii_lowercase())
        && lower.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    ok.then_some(lower)
}

impl Pattern for EnvVar {
    fn name(&self) -> &'static str {
        "env_var"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["var"]
    }
    fn matches(&self, c: &Clause) -> bool {
        c.verb == Verb::Read && c.has(Noun::Env)
    }
    fn variants(&self, c: &Clause, _env: &Env) -> Vec<Variant> {
        // The variable's name must be in the sentence.
        if c.literal.is_none() {
            return vec![];
        }
        vec![Variant::new("var", Ty::One(Scalar::Str)).fallible()]
    }
    fn emit(&self, _variant: &str, c: &Clause, g: &mut Gen, _input: &Value) -> Out {
        g.import("std::env");
        let name = c.literal.as_deref().expect("variants() checked");
        Out::Fallible(Fallible {
            expr: format!("env::var({})", rust_str(name)),
            ok: Ty::One(Scalar::Str),
            msg: Msg::new(format!("failed to read environment variable {}", fmt_text(name)), vec![]),
            name: ident(name),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_util::*;

    #[test]
    fn named_variable() {
        let c = clause("read the environment variable PORT");
        assert_eq!(names(EnvVar.variants(&c, &env(Ty::Unit))), vec!["var"]);
    }

    #[test]
    fn unnamed_variable_has_no_variant() {
        let c = clause("read the environment variable");
        assert!(EnvVar.variants(&c, &env(Ty::Unit)).is_empty());
    }
}
