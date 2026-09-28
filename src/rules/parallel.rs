//! "spawn 4 workers, each ...", "for each file in parallel, ..." -> scoped
//! threads over chunks of the list. No 'static bounds, no Arc: the threads
//! borrow the list and are joined when the scope ends.

use super::for_each::item_ty;
use super::{Env, Out, Pattern, Variant};
use crate::codegen::Gen;
use crate::lexicon::Verb;
use crate::model::{plural, Ty, Value};
use crate::parser::Clause;

pub struct Parallel;

impl Pattern for Parallel {
    fn name(&self) -> &'static str {
        "parallel"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["scoped_fixed", "scoped_auto"]
    }
    fn matches(&self, c: &Clause) -> bool {
        c.verb == Verb::Spawn
    }
    fn variants(&self, c: &Clause, env: &Env) -> Vec<Variant> {
        let Some(s) = item_ty(env.ty) else { return vec![] };
        if env.in_worker {
            return vec![]; // no nested pools
        }
        let name = if c.num.is_some() { "scoped_fixed" } else { "scoped_auto" };
        vec![Variant::new(name, Ty::One(s))]
    }
    fn emit(&self, variant: &str, c: &Clause, g: &mut Gen, input: &Value) -> Out {
        let s = item_ty(input.ty).expect("variants() checked");
        g.import("std::thread");

        // The workers need a slice to chunk.
        let (src, pattern_ref) = match input.ty {
            Ty::One(_) => {
                let v = g.fresh("lines");
                g.line(format!("let {v}: Vec<&str> = {}.lines().collect();", input.name));
                (v, true)
            }
            _ => (input.name.clone(), s.is_copy()),
        };
        let workers = g.fresh("workers");
        let count = match (variant, c.num) {
            ("scoped_fixed", Some(n)) => n.max(1).to_string(),
            _ => "thread::available_parallelism().map_or(4, |n| n.get())".to_string(),
        };
        g.line(format!("let {workers} = {count};"));
        let chunk_size = g.fresh("chunk_size");
        g.line(format!("let {chunk_size} = {src}.len().div_ceil({workers}).max(1);"));

        let item_name = g.fresh(s.item_name());
        // &str / &String / &PathBuf items are borrowed; numbers are copied out.
        let item = g.value(item_name.clone(), Ty::One(s), !s.is_copy());
        let binding = if pattern_ref { format!("&{item_name}") } else { item_name.clone() };

        let (was_loop, was_worker) = (g.in_loop, g.in_worker);
        g.in_loop = true;
        g.in_worker = true;
        let (body, out) = g.capture(|g| g.gen_block(&c.body, item.clone()));
        g.in_loop = was_loop;
        g.in_worker = was_worker;

        if out.version != item.version && out.ty != Ty::Unit {
            let os = out.scalar().expect("search keeps loop results scalar");
            let coll = g.fresh(&plural(&out.name));
            let elem = g.rust_ty(Ty::One(os));
            g.open(format!("let {coll}: Vec<{elem}> = thread::scope(|s| {{"));
            g.line(format!("let handles: Vec<_> = {src}"));
            g.indent();
            g.line(format!(".chunks({chunk_size})"));
            g.open(".map(|chunk| {");
            g.open("s.spawn(move || {");
            g.line("let mut out = Vec::new();");
            g.open(format!("for {binding} in chunk {{"));
            g.splice(body);
            g.line(format!("out.push({});", out.owned()));
            g.close("}");
            g.line("out");
            g.close("})");
            g.close("})");
            g.line(".collect();");
            g.dedent();
            g.line("handles");
            g.indent();
            g.line(".into_iter()");
            g.line(".flat_map(|handle| handle.join().expect(\"worker thread panicked\"))");
            g.line(".collect()");
            g.dedent();
            g.close("});");
            Out::Done(g.value(coll, Ty::Many(os), false))
        } else {
            g.open("thread::scope(|s| {");
            g.open(format!("for chunk in {src}.chunks({chunk_size}) {{"));
            g.open("s.spawn(move || {");
            g.open(format!("for {binding} in chunk {{"));
            g.splice(body);
            g.close("}");
            g.close("});");
            g.close("}");
            g.close("});");
            Out::Done(if out.consumed { input.sunk() } else { input.clone() })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Scalar;
    use crate::rules::test_util::*;

    #[test]
    fn worker_count_picks_variant() {
        let c = clause("spawn 4 workers");
        assert_eq!(names(Parallel.variants(&c, &env(Ty::Many(Scalar::Str)))), vec!["scoped_fixed"]);
        let c = clause("for each line in parallel");
        assert_eq!(names(Parallel.variants(&c, &env(Ty::Many(Scalar::Str)))), vec!["scoped_auto"]);
    }

    #[test]
    fn no_nested_pools() {
        let c = clause("spawn 4 workers");
        let e = Env { in_worker: true, ..env(Ty::Many(Scalar::Str)) };
        assert!(Parallel.variants(&c, &e).is_empty());
    }
}
