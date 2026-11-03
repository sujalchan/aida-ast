# aida-ast

`aida-ast` reads Scratch 3 project archives (`.sb3`), parses their targets,
variables, lists, block connections, inputs, and procedures, then generates
source code from that project. It has two output paths:

- **AIDA text** (`.aida`), a readable representation of Scratch blocks.
- **Rust runtime code**, a generated project module that works with
  [`aida-runtime`](../aida-runtime/README.md).

`aida-ast` is a compiler and source generator. It does not execute the Scratch
project. The runtime crate builds and runs the generated Rust module.

## Generate and run a Scratch project

Run these commands from the repository root, which contains the Cargo workspace
and both component directories. The generator writes the selected project to
`aida-runtime/src/program.rs`; the runtime build then compiles that file.

```sh
cargo run -p aida-ast -- --runtime aida-runtime/src/program.rs path/to/project.sb3 &&
cargo build --release -p aida-runtime &&
./target/release/aida-runtime --turbo
```

To build a different project, run the generator again with its `.sb3` path
before rebuilding the runtime. Building `aida-runtime` alone uses whichever
project was most recently generated into `program.rs`.

For Scratch-compatible maths, omit `--fast-math`. Fast maths is an opt-in
generator option; runtime scheduling is selected separately:

```sh
cargo run -p aida-ast -- --fast-math --runtime aida-runtime/src/program.rs path/to/project.sb3 &&
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
cargo run -p aida-ast -- --tests
```

With no arguments, the legacy text-generation path reads `example.sb3` from the
current directory and writes `project.json` and `project.aida` there.

## What it parses and generates

The parser reads the project archive and resolves Scratch block IDs and
connections into project, target, variable, list, and block data. The AIDA text
generator renders a readable intermediate form. The Rust generator emits
resumable scripts and project state for the runtime, using runtime helpers for
Scratch value conversions, operators, waits, broadcasts, clones, and scheduling.

See the [currently implemented Scratch blocks](scratch-block-support.md) for
the full opcode list and separate status for AIDA text generation and Rust
runtime behavior. The [runtime API and generated-code guide](docs/runtime-api.md)
describes the generated module contract and current limitations.

## Current implementation

These links go to the implementation used by this repository:

- [Scratch archive extraction](src/extractor.rs)
- [Project and block parsing](src/parser.rs)
- [AIDA text generation](src/aida/)
- [Rust code generation](src/rust/)
- [Runtime scheduler and Scratch value helpers](../aida-runtime/src/runtime/)

The implementation is under active development; the source and guides above
describe the current code rather than promising support for every Scratch block.
