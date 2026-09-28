//! "remove duplicates". Three ways, and the search picks by context:
//! - `sorted_dedup`: `dedup()` alone, only valid right after a sort
//! - `keep_order`: a HashSet of seen items, keeps first occurrences in order
//! - `sort_dedup`: sort then dedup, fast but reorders the list

use super::sort::sort_call;
use super::{Env, Out, Pattern, Variant};
use crate::codegen::Gen;
use crate::lexicon::Verb;
use crate::model::{Scalar, Ty, Value};
use crate::parser::Clause;

pub struct Dedupe;

impl Pattern for Dedupe {
    fn name(&self) -> &'static str {
        "dedupe"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["sorted_dedup", "keep_order", "sort_dedup"]
    }
    fn matches(&self, c: &Clause) -> bool {
        c.verb == Verb::Dedupe
    }
    fn variants(&self, _c: &Clause, env: &Env) -> Vec<Variant> {
        let Ty::Many(s) = env.ty else { return vec![] };
        let mut out = Vec::new();
        if env.sorted {
            out.push(Variant::new("sorted_dedup", env.ty).sorted());
        }
        if s != Scalar::Float {
            out.push(Variant::new("keep_order", env.ty));
        }
        out.push(Variant::new("sort_dedup", env.ty).sorted());
        out
    }
    fn emit(&self, variant: &str, _c: &Clause, g: &mut Gen, input: &Value) -> Out {
        let v = &input.name;
        let out = g.make_mut(input);
        match variant {
            "sorted_dedup" => g.line(format!("{v}.dedup();")),
            "keep_order" => {
                g.import("std::collections::HashSet");
                let seen = g.fresh("seen");
                let key = if input.scalar().is_some_and(|s| s.is_copy()) { "*item" } else { "item.clone()" };
                g.line(format!("let mut {seen} = HashSet::new();"));
                g.line(format!("{v}.retain(|item| {seen}.insert({key}));"));
            }
            _ => {
                g.line(format!("{v}.{};", sort_call(input.scalar())));
                g.line(format!("{v}.dedup();"));
            }
        }
        Out::Done(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_util::*;

    #[test]
    fn cheap_dedup_only_after_sort() {
        let c = clause("remove duplicates");
        let unsorted = env(Ty::Many(Scalar::Str));
        assert!(!names(Dedupe.variants(&c, &unsorted)).contains(&"sorted_dedup"));
        let sorted = Env { sorted: true, ..unsorted };
        assert!(names(Dedupe.variants(&c, &sorted)).contains(&"sorted_dedup"));
    }

    #[test]
    fn floats_cannot_be_hashed() {
        let c = clause("remove duplicates");
        assert!(!names(Dedupe.variants(&c, &env(Ty::Many(Scalar::Float)))).contains(&"keep_order"));
    }
}
