//! "sum them", "add them up" -> total of a list of numbers.

use super::{Env, Out, Pattern, Variant};
use crate::codegen::Gen;
use crate::lexicon::Verb;
use crate::model::{Ty, Value};
use crate::parser::Clause;

pub struct Sum;

impl Pattern for Sum {
    fn name(&self) -> &'static str {
        "sum"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["iter_sum"]
    }
    fn matches(&self, c: &Clause) -> bool {
        c.verb == Verb::Sum
    }
    fn variants(&self, _c: &Clause, env: &Env) -> Vec<Variant> {
        match env.ty {
            Ty::Many(s) if s.is_numeric() => vec![Variant::new("iter_sum", Ty::One(s))],
            _ => vec![],
        }
    }
    fn emit(&self, _variant: &str, _c: &Clause, g: &mut Gen, input: &Value) -> Out {
        let Ty::Many(s) = input.ty else { unreachable!("variants() checked") };
        let total = g.fresh("total");
        g.line(format!("let {total}: {} = {}.iter().sum();", s.rust(), input.name));
        Out::Done(g.value(total, Ty::One(s), false))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Scalar;
    use crate::rules::test_util::*;

    #[test]
    fn sums_numbers() {
        let c = clause("sum them");
        assert_eq!(names(Sum.variants(&c, &env(Ty::Many(Scalar::Float)))), vec!["iter_sum"]);
    }

    #[test]
    fn cannot_sum_text() {
        let c = clause("sum them");
        assert!(Sum.variants(&c, &env(Ty::Many(Scalar::Str))).is_empty());
    }
}
