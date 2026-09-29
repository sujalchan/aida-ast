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

Ensure you have Rust installed (`cargo` 1.70+ recommended).

```bash
# Clone the repository
git clone [https://github.com/sujalchan/aida-ast.git](https://github.com/sujalchan/aida-ast.git)
cd aida-ast

# Build the project
cargo build --release