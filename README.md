# aida-ast

`aida-ast` reads Scratch 3 project archives (`.sb3`), parses their targets,
variables, lists, block connections, inputs, and procedures, then generates
Rust runtime code for [`aida-runtime`](../aida-runtime/README.md).

`aida-ast` is a compiler and source generator. It does not execute the Scratch
project. The runtime crate builds and runs the generated Rust module.

## Generate and run a Scratch project

Run these commands from the repository root, which contains the Cargo workspace
and both component directories. The generator writes the selected project to
`aida-runtime/src/program.rs`; the runtime build then compiles that file.

```sh
cargo run --release -p aida-ast -- --runtime aida-runtime/src/program.rs path/to/project.sb3 &&
cargo build --release -p aida-runtime &&
./target/release/aida-runtime --turbo
```

To build a different project, run the generator again with its `.sb3` path
before rebuilding the runtime. Building `aida-runtime` alone uses whichever
project was most recently generated into `program.rs`.

For Scratch-compatible maths, omit `--fast-math`. Fast maths is an opt-in
generator option; runtime scheduling is selected separately:

```sh
cargo run --release -p aida-ast -- --fast-math --runtime aida-runtime/src/program.rs path/to/project.sb3 &&
cargo build --release -p aida-runtime &&
./target/release/aida-runtime --turbo
```

`--turbo` removes host frame pacing; it does not skip Scratch waits. The runtime
also accepts `--fps <number>` to set a frame rate. See the [AIDA run guide](../README.md)
for the combined launcher and more examples.

## Other commands

`--tests` generates a Rust module for every `.sb3` fixture directly inside
`aida-ast/tests/src` and writes the modules to `aida-ast/tests/output`:

```sh
cargo run --release -p aida-ast -- --tests
```

The command requires either `--runtime <output.rs> <input.sb3>` or `--tests`.

## What it parses and generates

The parser reads the project archive and resolves Scratch block IDs and
connections into project, target, variable, list, and block data. The Rust
generator emits resumable scripts and project state for the runtime, using
runtime helpers for Scratch value conversions, operators, waits, broadcasts,
clones, and scheduling.

### Implemented Rust blocks

The Rust backend currently executes these supported groups:

- **Control:** conditionals, repeat/while/forever loops, waits, all
  `control_stop` options, clone creation/deletion, and custom procedure calls,
  including warp procedures.
- **Events:** green-flag, broadcast, broadcast-and-wait, clone-start, key-press,
  sprite/stage click, greater-than sensor, and backdrop-switch scripts.
- **Data:** shared stage variables, sprite-local and clone-local variables,
  lists, and list monitor visibility. Dynamic show/hide blocks for scalar
  variable monitors are not implemented.
- **Looks:** say/think bubbles (including timed versions), backdrop selection
  and cycling, and the backdrop number/name reporter. Graphical mode renders
  the bubbles and selected backdrop; `say` also emits its value to the console.
- **Motion:** go to x/y, go to a named or random position, change x/y, and timed
  glides, with per-sprite and per-clone positions rendered in graphical mode.
- **Operators:** arithmetic, comparisons, boolean and text reporters, random,
  rounding, and the supported Scratch math operations.
- **Sensing:** key state and mouse state/position use graphical input; the
  timer works in both graphical and headless modes.

This is the executable Rust backend list. The [block support table](scratch-block-support.md)
lists catalogued opcodes and their Rust-runtime status.
The [runtime guide](docs/runtime-api.md) describes scheduling and edge cases.

## Current implementation

These links go to the implementation used by this repository:

- [Scratch archive extraction](src/extractor.rs)
- [Project and block parsing](src/parser.rs)
- [Rust code generation](src/rust/)
- [Runtime scheduler and Scratch value helpers](../aida-runtime/src/runtime/)

The implementation is under active development; the source and guides above
describe the current code rather than promising support for every Scratch block.

## Rust generator layout

The Rust backend is grouped by compiler responsibility:

- `src/rust/project/` analyzes project identifiers and state, gathers monitors,
  selects runtime imports, and assembles the generated program.
- `src/rust/scripts/` discovers event hats and emits script starters and
  procedure signatures.
- `src/rust/statements/` builds script graphs and generates direct procedures,
  resumable statements, opcode dispatch, structured control flow, and loop
  optimizations.
- `src/rust/expressions/` handles general Scratch reporters separately from
  proven-native local numeric expressions.
- `src/rust/direct_numeric.rs` and `src/rust/literals.rs` keep specialized
  numeric generation and shared Rust literal formatting focused.
- `src/rust/tests/` groups generator tests by control flow, data, events,
  expressions, procedures, rendering, and overall generation.

The general and optimized expression/statement paths remain distinct where
they preserve Scratch coercion rules or avoid work in generated hot loops.
