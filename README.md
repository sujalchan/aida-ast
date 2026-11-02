# aida-ast

`aida-ast` is a lightweight Rust tool that converts Scratch (`.sb3`) project archives into a custom Abstract Syntax Tree (AST) representation (`.aida`). 

It provides the foundational parsing pipeline needed to translate visual block-based Scratch projects into structured, strongly-typed code representations.

---

## Features

- **`.sb3` Extraction:** Directly reads Scratch project archives and extracts `project.json` into memory or disk.
- **Target Analysis:** Parses Stage and Sprite targets along with their associated properties.
- **AST Preparation:** Lays the groundwork for transforming Scratch's flat block representations into hierarchical execution trees.

---

## Installation & Setup

Ensure you have Rust installed. For the complete Scratch-to-Rust workflow,
check out the base `sujalchan/aida` project with both component repositories as
submodules. Keep `aida-ast` and `aida-runtime` side-by-side under the `aida`
root; the generator writes generated Rust into `aida-runtime/src/program.rs`.

```bash
# Clone the base project and its component repositories
git clone --recurse-submodules https://github.com/sujalchan/aida.git
cd aida

# If the base project was cloned without its submodules
git submodule update --init --recursive

# Build both workspace members
cargo build --release --workspace
```

The expected layout is:

```text
aida/
├── Cargo.toml
├── aida-ast/       # https://github.com/sujalchan/aida-ast
└── aida-runtime/   # https://github.com/sujalchan/aida-runtime
```

Run the full pipeline from the base project root so Cargo can find both
workspace members and the generated runtime destination. See the [base project
README](https://github.com/sujalchan/aida) for run commands.
