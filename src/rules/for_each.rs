//! "for each line, ..." -> a `for` loop. If the body makes a value, the loop
//! collects one per item into a Vec (skipped items via `continue` drop out).

use super::{Env, Out, Pattern, Variant};
use crate::codegen::Gen;
use crate::lexicon::Verb;
use crate::model::{plural, Scalar, Ty, Value};
use crate::parser::Clause;

pub struct ForEach;

/// Type of one item when iterating a value of type `ty`.
pub fn item_ty(ty: Ty) -> Option<Scalar> {
    match ty {
        Ty::One(Scalar::Str) => Some(Scalar::Str), // lines of the text
        Ty::Many(s) => Some(s),
        _ => None,
    }
}

impl Pattern for ForEach {
    fn name(&self) -> &'static str {
        "for_each"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["for_loop"]
    }
    fn matches(&self, c: &Clause) -> bool {
        c.verb == Verb::ForEach
    }
    fn variants(&self, _c: &Clause, env: &Env) -> Vec<Variant> {
        match item_ty(env.ty) {
            Some(s) => vec![Variant::new("for_loop", Ty::One(s))],
            None => vec![],
        }
    }
    fn emit(&self, _variant: &str, c: &Clause, g: &mut Gen, input: &Value) -> Out {
        let s = item_ty(input.ty).expect("variants() checked");
        let (iter, borrowed) = match input.ty {
            Ty::One(_) => (format!("{}.lines()", input.name), true),
            _ if s.is_copy() => (format!("{}.iter().copied()", input.name), false),
            _ => (format!("&{}", input.name), true),
        };
        let item_name = g.fresh(s.item_name());
        let item = g.value(item_name.clone(), Ty::One(s), borrowed);

        let (was_loop, was_worker) = (g.in_loop, g.in_worker);
        g.in_loop = true;
        let (body, out) = g.capture(|g| g.gen_block(&c.body, item.clone()));
        g.in_loop = was_loop;
        g.in_worker = was_worker;

        let header = format!("for {item_name} in {iter} {{");
        if out.version != item.version && out.ty != Ty::Unit {
            let os = out.scalar().expect("search keeps loop results scalar");
            let coll = g.fresh(&plural(&out.name));
            g.line(format!("let mut {coll} = Vec::new();"));
            g.open(header);
            g.splice(body);
            g.line(format!("{coll}.push({});", out.owned()));
            g.close("}");
            let mut v = g.value(coll, Ty::Many(os), false);
            v.mutable = true;
            Out::Done(v)
        } else {
            g.open(header);
            g.splice(body);
            g.close("}");
            Out::Done(if out.consumed { input.sunk() } else { input.clone() })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_util::*;

    #[test]
    fn iterates_lists_and_text() {
        let c = clause("for each line, print it");
        assert_eq!(names(ForEach.variants(&c, &env(Ty::Many(Scalar::Int)))), vec!["for_loop"]);
        assert_eq!(names(ForEach.variants(&c, &env(Ty::One(Scalar::Str)))), vec!["for_loop"]);
    }

    #[test]
    fn cannot_iterate_a_number() {
        let c = clause("for each line, print it");
        assert!(ForEach.variants(&c, &env(Ty::One(Scalar::Int))).is_empty());
    }
}
