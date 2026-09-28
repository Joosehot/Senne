//! "skip blank lines". Inside a loop: `continue` on a blank item. On a list:
//! `retain`. On whole text: split it into its non-blank lines.

use super::{Env, Out, Pattern, Variant};
use crate::codegen::Gen;
use crate::lexicon::Verb;
use crate::model::{Scalar, Ty, Value};
use crate::parser::Clause;

pub struct SkipBlank;

impl Pattern for SkipBlank {
    fn name(&self) -> &'static str {
        "skip_blank"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["filter"]
    }
    fn matches(&self, c: &Clause) -> bool {
        c.verb == Verb::SkipBlank
    }
    fn variants(&self, _c: &Clause, env: &Env) -> Vec<Variant> {
        match env.ty {
            Ty::One(Scalar::Str) if env.in_loop => vec![Variant::new("filter", env.ty).sink()],
            Ty::One(Scalar::Str) | Ty::Many(Scalar::Str) => {
                vec![Variant::new("filter", Ty::Many(Scalar::Str))]
            }
            _ => vec![],
        }
    }
    fn emit(&self, _variant: &str, _c: &Clause, g: &mut Gen, input: &Value) -> Out {
        let v = &input.name;
        match input.ty {
            Ty::One(_) if g.in_loop => {
                g.open(format!("if {v}.trim().is_empty() {{"));
                g.line("continue;");
                g.close("}");
                Out::Done(input.clone())
            }
            Ty::One(_) => {
                let lines = g.fresh("lines");
                g.line(format!(
                    "let {lines}: Vec<String> = {v}.lines().filter(|l| !l.trim().is_empty()).map(str::to_owned).collect();"
                ));
                Out::Done(g.value(lines, Ty::Many(Scalar::Str), false))
            }
            _ => {
                let out = g.make_mut(input);
                g.line(format!("{v}.retain(|l| !l.trim().is_empty());"));
                Out::Done(out)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_util::*;

    #[test]
    fn in_a_loop_it_is_a_check() {
        let c = clause("skip blank lines");
        let e = Env { in_loop: true, ..env(Ty::One(Scalar::Str)) };
        assert!(!SkipBlank.variants(&c, &e)[0].produces);
    }

    #[test]
    fn needs_text() {
        let c = clause("skip blank lines");
        assert!(SkipBlank.variants(&c, &env(Ty::Many(Scalar::Int))).is_empty());
    }
}
