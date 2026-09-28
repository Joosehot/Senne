//! "wait 200ms", "sleep 2 seconds".

use super::{Env, Out, Pattern, Variant};
use crate::codegen::Gen;
use crate::lexicon::Verb;
use crate::model::Value;
use crate::parser::Clause;

pub struct Wait;

impl Pattern for Wait {
    fn name(&self) -> &'static str {
        "wait"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["sleep"]
    }
    fn matches(&self, c: &Clause) -> bool {
        c.verb == Verb::Wait
    }
    fn variants(&self, c: &Clause, env: &Env) -> Vec<Variant> {
        // A duration needs a unit: "wait 3" is ambiguous.
        if c.millis.is_none() {
            return vec![];
        }
        vec![Variant::new("sleep", env.ty).sink()]
    }
    fn emit(&self, _variant: &str, c: &Clause, g: &mut Gen, input: &Value) -> Out {
        g.import("std::thread");
        g.import("std::time::Duration");
        let ms = c.millis.expect("variants() checked");
        g.line(format!("thread::sleep(Duration::from_millis({ms}));"));
        Out::Done(input.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Ty;
    use crate::rules::test_util::*;

    #[test]
    fn waits_with_unit() {
        assert_eq!(names(Wait.variants(&clause("wait 200ms"), &env(Ty::Unit))), vec!["sleep"]);
    }

    #[test]
    fn rejects_bare_number() {
        assert!(Wait.variants(&clause("wait 3"), &env(Ty::Unit)).is_empty());
    }
}
