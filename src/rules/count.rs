//! "count them", "count the lines" -> usize.

use super::{Env, Out, Pattern, Variant};
use crate::codegen::Gen;
use crate::lexicon::Verb;
use crate::model::{Scalar, Ty, Value};
use crate::parser::Clause;

pub struct Count;

impl Pattern for Count {
    fn name(&self) -> &'static str {
        "count"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["len"]
    }
    fn matches(&self, c: &Clause) -> bool {
        c.verb == Verb::Count
    }
    fn variants(&self, _c: &Clause, env: &Env) -> Vec<Variant> {
        match env.ty {
            Ty::Many(_) | Ty::One(Scalar::Str) => vec![Variant::new("len", Ty::One(Scalar::Usize))],
            _ => vec![],
        }
    }
    fn emit(&self, _variant: &str, _c: &Clause, g: &mut Gen, input: &Value) -> Out {
        let count = g.fresh("count");
        let expr = match input.ty {
            Ty::One(_) => format!("{}.lines().count()", input.name),
            _ => format!("{}.len()", input.name),
        };
        g.line(format!("let {count} = {expr};"));
        Out::Done(g.value(count, Ty::One(Scalar::Usize), false))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_util::*;

    #[test]
    fn counts_lists_and_lines() {
        let c = clause("count them");
        assert_eq!(names(Count.variants(&c, &env(Ty::Many(Scalar::Path)))), vec!["len"]);
        assert_eq!(names(Count.variants(&c, &env(Ty::One(Scalar::Str)))), vec!["len"]);
    }

    #[test]
    fn cannot_count_a_number() {
        let c = clause("count them");
        assert!(Count.variants(&c, &env(Ty::One(Scalar::Int))).is_empty());
    }
}
