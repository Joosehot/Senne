//! "log it", "log \"starting\"" outside an error handler -> stderr. (Inside
//! "if it fails, ..." logging is part of the error strategy.)

use super::print::print_value;
use super::{Env, Out, Pattern, Variant};
use crate::codegen::Gen;
use crate::lexicon::Verb;
use crate::model::{Ty, Value};
use crate::parser::Clause;

pub struct Log;

impl Pattern for Log {
    fn name(&self) -> &'static str {
        "log"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["eprintln"]
    }
    fn matches(&self, c: &Clause) -> bool {
        c.verb == Verb::Log
    }
    fn variants(&self, c: &Clause, env: &Env) -> Vec<Variant> {
        if env.ty == Ty::Unit && c.literal.is_none() {
            return vec![];
        }
        vec![Variant::new("eprintln", env.ty).sink()]
    }
    fn emit(&self, _variant: &str, c: &Clause, g: &mut Gen, input: &Value) -> Out {
        print_value(g, "eprintln", "debug", c, input);
        Out::Done(if input.ty == Ty::Unit { input.clone() } else { input.sunk() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Scalar;
    use crate::rules::test_util::*;

    #[test]
    fn logs_a_value() {
        let c = clause("log it");
        assert_eq!(names(Log.variants(&c, &env(Ty::One(Scalar::Int)))), vec!["eprintln"]);
    }

    #[test]
    fn nothing_to_log() {
        let c = clause("log it");
        assert!(Log.variants(&c, &env(Ty::Unit)).is_empty());
    }
}
