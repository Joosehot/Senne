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
senne --game -f behavior.txt --struct Name   # a Droid game (see below)
```

## The proof

`cargo test` runs:
- unit tests: one passing and one failing test per pattern, plus the lexicon, parser, judges and config
- `tests/golden.rs`: each sentence in `examples/*.txt` is compiled 6 times in-process and must be byte-identical and match `examples/out/*.rs`. Every golden file must also compile with `rustc -D warnings`.

Run `SENNE_BLESS=1 cargo test` to accept intended output changes.

## The game domain (`senne::game`)

A second, separate domain for [Droid](../AIgame-engine), where a whole game comes from one sentence. Droid's front decides *what* the game is and writes its behaviour as plain English (`behavior.txt`). Senne decides *how* that behaviour is coded and writes the game's `src/main.rs` against Droid's `Ctx` API, shaped like Droid's hand-written `games/coin_runner`. The Rust domain above is untouched.

```
$ senne --game -f behavior.txt --struct Platformer [--explain] [--bare] [--lenient] [-o src/main.rs]
```

```
Move the player sideways with the arrow keys at 7.5 units per second and face where it moves.
The player jumps with space at 14.5 units per second, twice at 12 units per second, higher while held, with the jump sound.
Each slime patrols 3 units to each side at 2.2 units per second.
When the player lands on a slime, destroy it and bounce the player up at 11 units per second.
When the player touches a slime, the player loses a life, safe for 1.5 seconds.
When the player touches the flag and no coins are left, the player wins "LEVEL CLEAR".
When the player presses r, restart the game.
```

Pipeline: `src/game/lexicon.rs` (closed vocabulary, singular/plural things, "did you mean") → `parser.rs` (standing behaviours, or a trigger plus actions; each option at most once; leftover words are errors) → `search.rs` (beam over code-level variants, width from `[search].beam`, deterministic tie-break) → `codegen.rs` → `explain.rs`.

- **Patterns** (`src/game/rules/`, one file each, each with a test where it applies and one where it doesn't). Standing: `move_player` (axis / raw / normalized), `jump` (on_press / buffered), `player_shoot` (countdown / timestamp), `chase` (hoisted / per_loop), `patrol` (bounds / sine), `fly` (velocity / anchored), `wander` (shared_timer / own_timer), `march` (move_by / velocity), `enemy_shoot` (hoisted / per_loop / fixed), `animate` (clock / engine). Triggers: `start`, `every_frame`, `every`, `touch`, `hit`, `lands_on`, `none_left`, `reaches`, `falls_below`, `key_press`. Actions: `destroy`, `score`, `lives`, `hurt`, `win`, `lose` (inline / helper each), `sound`, `music`, `show` (once, or for a live HUD every_frame / on_change), `bounce`, `back_to_start`, `squash`, `play_anim`, `restart`, `next_wave`, `random_shot`.
- **Judges** (`src/game/rules/judges.rs`): `helper_reuse` (a win/lose/hurt helper used once costs, the same block inlined twice costs, mixing costs; the "lose" inside each hurt block counts) and `hoist_reuse` (reading the player position once per frame pays from the second loop; mixing costs). A penalty for a lone helper or hoist waits until no later site could reuse it, so the beam does not prune the shared version early.
- **rules.toml**: `[game.profile]`, `[game.rules.<pattern>]` variants on safety / brevity / perf, `[game.judges.<name>]`, and `[game.tuning]`, which holds every constant of the generated code (smoothing rate, jump buffer, coyote time, blink rate, shot size, stomp height, text positions, ...). Unknown or missing patterns, variants, judges or tuning keys fail at load time. `safely`, `quickly` and `simply` shift a sentence's profile.
- **Language additions** beyond Droid's contract list: `a random <thing> shoots down|up|at the player ...` (an action, for space-invader bombs), `each <thing> shoots down|up ...`, `the player hits` as a synonym of `touches`, `towards`, `with the arrows`, and single letters as keys.
- **Proof** (`tests/game.rs`): every `examples/game/*.txt` compiles 6 times to byte-identical Rust matching `examples/game/out/*.rs` (`SENNE_BLESS=1` to accept), and every golden type-checks with `rustc -D warnings` against `tests/game_stub/droid.rs`, a signature stub of Droid's API. `07_every_form.txt` uses every sentence form. The goldens have also been built against the real `droid` crate with `RUSTFLAGS=-D warnings`.

## Non-goals for v0

General-purpose English, arbitrary business logic, and type inference across a whole codebase. The generated code uses std only.

## License

MIT, see [LICENSE](LICENSE). Rust code that Senne generates belongs to whoever ran it.
