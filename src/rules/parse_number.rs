//! "parse it as an integer" / "as a decimal" -> i64 / f64 from text.

use super::{Env, Fallible, Msg, Out, Pattern, Variant};
use crate::codegen::Gen;
use crate::lexicon::{Noun, Verb};
use crate::model::{Scalar, Ty, Value};
use crate::parser::Clause;

pub struct ParseNumber;

fn target(c: &Clause) -> Scalar {
    if c.has(Noun::Float) {
        Scalar::Float
    } else {
        Scalar::Int
    }
}

impl Pattern for ParseNumber {
    fn name(&self) -> &'static str {
        "parse_number"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["parse"]
    }
    fn matches(&self, c: &Clause) -> bool {
        c.verb == Verb::Parse
    }
    fn variants(&self, c: &Clause, env: &Env) -> Vec<Variant> {
        if env.ty != Ty::One(Scalar::Str) {
            return vec![];
        }
        vec![Variant::new("parse", Ty::One(target(c))).fallible()]
    }
    fn emit(&self, _variant: &str, c: &Clause, _g: &mut Gen, input: &Value) -> Out {
        let s = target(c);
        let v = &input.name;
        Out::Fallible(Fallible {
            expr: format!("{v}.trim().parse::<{}>()", s.rust()),
            ok: Ty::One(s),
            msg: Msg::new(format!("could not parse {{{v}:?}} as {}", s.describe()), vec![]),
            name: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_util::*;

    #[test]
    fn parses_text() {
        let c = clause("parse it as a decimal");
        let v = ParseNumber.variants(&c, &env(Ty::One(Scalar::Str)));
        assert_eq!(v[0].output, Ty::One(Scalar::Float));
    }

    #[test]
    fn needs_text() {
        let c = clause("parse it as an integer");
        assert!(ParseNumber.variants(&c, &env(Ty::Many(Scalar::Str))).is_empty());
    }
}
