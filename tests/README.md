# Scratch fixture tests

SOME TESTS LOOP FOREVER FOR THIS CASE USE --max-tick 1000, --test or only evaluate for 10 seconds

The `.sb3` projects in `tests/src` are end-to-end checks for Rust generation
and runtime behavior. The files in `tests/expected` are the behavioral
reference logs. Read the matching expected log before changing lowering,
scheduling, broadcasts, clone behavior, waits, or variable updates. Compare the
ordered observable output, not just the final variable values: execution order
is part of these fixtures' behavior.

## Run the fixture projects

From the repository root, run every project in release mode with the launcher:

```sh
./run.sh
```

The launcher compiles both `aida-ast` and `aida-runtime` with Cargo's
`--release` profile. In the all-projects mode it applies a 1,000 scheduler-tick
limit to each fixture, including projects with intentional loops. Output can
be saved for review:

```sh
./run.sh > /tmp/aida-fixtures.log 2>&1
```

Run an individual fixture with the same release pipeline and explicit bound:

```sh
./run.sh aida-ast/tests/src/clone-stop-test.sb3 --max-ticks 1000
```

Clone and broadcast scheduling can vary with frame pacing. Capture these cases
at 30 FPS, 40 FPS, and turbo, keeping the runs bounded:

```sh
./run.sh aida-ast/tests/src/clone-stop-test.sb3 --fps 30 --max-ticks 1000
./run.sh aida-ast/tests/src/clone-stop-test.sb3 --fps 40 --max-ticks 1000
./run.sh aida-ast/tests/src/clone-stop-test.sb3 --turbo --max-ticks 1000

./run.sh aida-ast/tests/src/clone-with-variables.sb3 --fps 30 --max-ticks 1000
./run.sh aida-ast/tests/src/clone-with-variables.sb3 --fps 40 --max-ticks 1000
./run.sh aida-ast/tests/src/clone-with-variables.sb3 --turbo --max-ticks 1000

./run.sh aida-ast/tests/src/broadcast-test.sb3 --fps 30 --max-ticks 1000
./run.sh aida-ast/tests/src/broadcast-test.sb3 --fps 40 --max-ticks 1000
./run.sh aida-ast/tests/src/broadcast-test.sb3 --turbo --max-ticks 1000
```

On Windows, `run.bat` accepts the same project and runtime options. It also
uses release builds and bounds all-projects runs to 1,000 ticks per fixture.

## Sensing-of fixtures

| Project | Coverage |
|---|---|
| `src/sensing-of.sb3` | The seven supplied Stage and sprite property checks, including a sprite with no scripts. |
| `src/sensing-of-mutations.sb3` | Live sprite position, direction, costume, size, and volume plus changing Stage backdrop and volume. |

## Costume-size fixture

| Project | Coverage |
|---|---|
| `src/costume-size.sb3` | Scratch costume-dependent size limits, fractional size changes, costume switching, and costume number/name reporters. |

The matching expected log contains eight `say: "true"` lines in script order.

## Compare expected logs

Expected logs use the project basename. For example,
`expected/clone-with-variables.txt` corresponds to
`src/clone-with-variables.sb3`. Timing-sensitive fixtures use mode-specific
files, such as `expected/clone-stop-test-fps30.txt`,
`expected/clone-stop-test-fps40.txt`, and
`expected/clone-stop-test-turbo.txt`.

When reviewing a result:

1. Read the matching expected file before changing code.
2. Run the fixture in release mode with the documented FPS/turbo setting and
   tick limit.
3. Compare the ordered `say:` outputs and any explicitly listed variable or
   list outputs. Ignore Cargo build messages and compiler warnings.
4. If order or values differ, investigate the generated Rust and scheduler
   behavior. Update an expected log only when the new result is intentional
   and has been checked against the fixture's control flow.

Some projects print a variable value for every assignment. Keep expected logs
focused on the stable observable sequence needed to detect ordering changes;
do not copy unbounded per-step traces into the expected directory.

## Run generator and unit tests

The end-to-end launcher runs generated Rust programs. The following commands
run Rust unit tests and regenerate the per-fixture Rust source snapshots; they
do not replace the runtime fixture runs above:

```sh
cargo test -p aida-ast
cargo test -p aida-runtime
cargo run --release -p aida-ast -- --tests
```

Generated Rust source snapshots are stored under `tests/output`; expected
runtime logs are stored separately under `tests/expected`.
