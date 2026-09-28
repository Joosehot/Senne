//! "sort them". `stable` keeps equal items in order; `unstable` is faster.
//! Floats sort with `total_cmp`.

use super::{Env, Out, Pattern, Variant};
use crate::codegen::Gen;
use crate::lexicon::Verb;
use crate::model::{Scalar, Ty, Value};
use crate::parser::Clause;

pub struct Sort;

impl Pattern for Sort {
    fn name(&self) -> &'static str {
        "sort"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["stable", "unstable"]
    }
    fn matches(&self, c: &Clause) -> bool {
        c.verb == Verb::Sort
    }
    fn variants(&self, _c: &Clause, env: &Env) -> Vec<Variant> {
        match env.ty {
            Ty::Many(_) => vec![
                Variant::new("stable", env.ty).sorted(),
                Variant::new("unstable", env.ty).sorted(),
            ],
            _ => vec![],
        }
    }
    fn emit(&self, variant: &str, _c: &Clause, g: &mut Gen, input: &Value) -> Out {
        let v = &input.name;
        let call = match (variant, input.scalar()) {
            ("stable", Some(Scalar::Float)) => "sort_by(f64::total_cmp)",
            (_, Some(Scalar::Float)) => "sort_unstable_by(f64::total_cmp)",
            ("stable", _) => "sort()",
            _ => "sort_unstable()",
        };
        let out = g.make_mut(input);
        g.line(format!("{v}.{call};"));
        Out::Done(out)
    }
}

pub fn sort_call(scalar: Option<Scalar>) -> &'static str {
    if scalar == Some(Scalar::Float) {
        "sort_unstable_by(f64::total_cmp)"
    } else {
        "sort_unstable()"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_util::*;

    #[test]
    fn sorts_lists() {
        let c = clause("sort them");
        let v = Sort.variants(&c, &env(Ty::Many(Scalar::Str)));
        assert_eq!(names(v.clone()), vec!["stable", "unstable"]);
        assert!(v.iter().all(|v| v.sorted));
    }

    #[test]
    fn cannot_sort_one_value() {
        let c = clause("sort them");
        assert!(Sort.variants(&c, &env(Ty::One(Scalar::Str))).is_empty());
    }
}
