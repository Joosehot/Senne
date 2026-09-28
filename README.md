# Senne

Describe the code in plain words and get deterministic Rust back. It feels like prompting an AI but behaves like a compiler.

```
$ senne 'read the file "numbers.txt", for each line skip blank lines, parse it as an integer,
         and if it fails, log the error and continue. Then sum them and print the total.'
```
```rust
pub fn run() -> Result<(), Box<dyn Error>> {
    // read the file "numbers.txt"  [read_file/to_string + propagate]
    let contents = fs::read_to_string("numbers.txt")?;
    // for each line  [for_each/for_loop]
    let mut numbers = Vec::new();
    for line in contents.lines() {
        // skip blank lines  [skip_blank/filter]
        if line.trim().is_empty() {
            continue;
        }
        // parse it as an integer  [parse_number/parse + handle]
        let n = match line.trim().parse::<i64>() {
            Ok(value) => value,
            Err(err) => {
                eprintln!("could not parse {line:?} as an integer: {err}");
                continue;
            }
        };
        numbers.push(n);
    }
    // sum them  [sum/iter_sum]
    let total: i64 = numbers.iter().sum();
    // print the total  [print/display]
    println!("{total}");
    Ok(())
}
```

The same sentence always produces byte-identical output, and no model runs in the generation path. Every statement carries a comment with the words that asked for it and the rule that wrote it.

## Architecture (same as FeelRight)

| FeelRight | Senne | Role |
|---|---|---|
| `theory.rs` | `lexicon.rs` | closed vocabulary: phrases → verbs, nouns, modifiers, connectives |
| `parser.rs` | `parser.rs` | tokens → clause tree (error handlers, loops, retries folded in) |
| `src/rules/` | `src/rules/` | one file per pattern; offers variants and emits Rust |
| `search.rs` | `search.rs` | beam search over variant × error strategy × retry style |
| `rules.toml` | `rules.toml` | every weight; no numbers in rule code |
| tension curve | modifiers | "safely", "quickly" and "simply" shift the safety/brevity/perf profile |
| breakable rules | judges | `no_panic` is breakable, and brevity forgives part of the penalty |
| `midi.rs` | `codegen.rs` | winning choices → Rust source |

**Where the tension is.** Every choice has safety/brevity/perf axes in `rules.toml`. A clause's profile starts at `[profile]` and shifts with its modifiers. For example, "simply read the file and print it" gives `.expect(...)`, while the plain sentence gives `?`. "Safely write" gives a temp file plus a rename, and "quickly read the lines" gives a `BufReader`. Judges look across the whole program: using `?` makes `run()` return `Result` (a cost), mixing `?` with `expect` is penalized, and a shared retry helper pays off once two steps use it. These cross-clause interactions are why the search is a beam and not a greedy pick.

## v0 patterns (19) and judges (4)

Patterns: `read_file`, `read_lines` (collect/buffered), `write_file` (plain/atomic), `env_var`, `parse_number`, `create_dir`, `list_dir`, `for_each`, `parallel` (scoped threads over chunks), `sum`, `count`, `sort` (stable/unstable), `dedupe` (sorted_dedup/keep_order/sort_dedup), `skip_blank`, `trim`, `print` (display/each_line/debug), `log`, `wait`, `retry_helper`.

Error strategies are chosen for every fallible step: `propagate` (`?`), `expect`, `or_default` ("or 0"), and `handle` ("if it fails, log the error and continue / give up"). Retry styles: inline loop or shared helper, each with a fixed delay or backoff.

Judges: `no_panic`, `result_signature`, `consistent_errors`, `helper_reuse`.

## Grammar, briefly

- Every verb starts a clause. Nouns, quoted text, numbers and modifiers attach to the clause they appear in.
- "if it fails, …" handles the previous clause. The handler ends at "then", "after that" or ".".
- "for each …" and "spawn N workers, each …" open a loop. The loop ends at "after that", ".", or an aggregate verb (sum, count, sort, remove duplicates, write).
- "retry N times" attaches to the step it's glued to ("retry reading …") or to the one before it. Used alone ("retry this safely three times"), it emits a reusable `retry` function.
- An unknown word is an error that suggests the closest known word (`--lenient` ignores it instead). A value that nothing uses is also an error. The engine never guesses.

## Usage

```
senne "sentence"            # Rust to stdout
senne -f spec.txt -o out.rs
senne --explain "..."       # tokens, clauses, every candidate and score (stderr)
senne --bare "..."          # no per-statement comments
senne --rules my_rules.toml # different weights
senne --vocabulary          # every known word and phrase
```

## The proof

`cargo test` runs:
- unit tests: one passing and one failing test per pattern, plus the lexicon, parser, judges and config
- `tests/golden.rs`: each sentence in `examples/*.txt` is compiled 6 times in-process and must be byte-identical and match `examples/out/*.rs`. Every golden file must also compile with `rustc -D warnings`.

Run `SENNE_BLESS=1 cargo test` to accept intended output changes.

## Non-goals for v0

General-purpose English, arbitrary business logic, and type inference across a whole codebase. The generated code uses std only.

## License

Proprietary, all rights reserved. See [LICENSE](LICENSE). Rust code that Senne generates belongs to whoever ran it.
