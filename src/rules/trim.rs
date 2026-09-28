//! "trim it", "trim each line".

use super::{Env, Out, Pattern, Variant};
use crate::codegen::Gen;
use crate::lexicon::Verb;
use crate::model::{Scalar, Ty, Value};
use crate::parser::Clause;

pub struct Trim;

impl Pattern for Trim {
    fn name(&self) -> &'static str {
        "trim"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["trim"]
    }
    fn matches(&self, c: &Clause) -> bool {
        c.verb == Verb::Trim
    }
    fn variants(&self, _c: &Clause, env: &Env) -> Vec<Variant> {
        match env.ty {
            Ty::One(Scalar::Str) | Ty::Many(Scalar::Str) => vec![Variant::new("trim", env.ty)],
            _ => vec![],
        }
    }
    fn emit(&self, _variant: &str, _c: &Clause, g: &mut Gen, input: &Value) -> Out {
        let v = &input.name;
        match input.ty {
            Ty::One(_) => {
                g.line(format!("let {v} = {v}.trim();"));
                Out::Done(g.value(v.clone(), input.ty, true))
            }
            _ => {
                g.line(format!(
                    "let {v}: Vec<String> = {v}.iter().map(|l| l.trim().to_owned()).collect();"
                ));
                Out::Done(g.value(v.clone(), input.ty, false))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_util::*;

    #[test]
    fn trims_text() {
        let c = clause("trim it");
        assert_eq!(names(Trim.variants(&c, &env(Ty::One(Scalar::Str)))), vec!["trim"]);
    }

    #[test]
    fn cannot_trim_numbers() {
        let c = clause("trim it");
        assert!(Trim.variants(&c, &env(Ty::One(Scalar::Int))).is_empty());
    }
}
