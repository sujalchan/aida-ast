# Scratch to AIDA

`cargo run` reads `example.sb3`, keeps the existing console debug output, and
writes `project.aida`.

`cargo run -- --tests` generates Rust source for every `.sb3` file directly
inside `tests/src` and writes a matching `.rs` file into `tests/output`. The
output directory is created automatically. Generated functions call the
`ScratchValue` helpers from `aida-runtime` (`scratch_add`, `scratch_equals`,
`scratch_greater_than`, `scratch_join`, and related operations). Include the
generated file as a module in the runtime crate and call its `run()` function.
The scan includes files directly inside `tests/src`.
See the [generated Rust and runtime API spec](runtime-api.md) for the module
contract and current lowering coverage.

`aida-ast --runtime <output.rs> <input.sb3>` writes generated Rust to the given
path. The repository's `./run.sh [input.sb3]` uses this mode to replace
`aida-runtime/src/program.rs`, then builds and runs `aida-runtime`.

For VS Code syntax highlighting, see the [AIDA extension](../editors/vscode/README.md).

## Names and values

```text
var: test_2 = 0
set: test_2 = join("1", "1") + 2
change: test_2 by 1
list: shopping_list = ["apples", 2]
append: shopping_list with "pears"
```

Variable, list, procedure and parameter names are unquoted identifiers. Spaces
become underscores. Other punctuation is encoded as `_u<hex>_`, and names starting
with digits get an underscore prefix. Collisions get stable `__2`, `__3`, etc.
suffixes. References resolve through the original Scratch IDs. `global.name`
selects a stage variable or list when a sprite declaration shadows its name.
Cloud declarations use `cloud var:`. Stage and sprite display names remain quoted.

String **values** keep their quotes and their original content, including spaces.
Numeric primitive tags produce numbers, even when the JSON stores them as strings.
Numeric-looking text values such as `0.0` and `5` also produce unquoted numbers.
Blank numeric sockets become zero. Special list indexes such as `last`, `all` and
`random` remain selectors even when Scratch stores them in numeric shadow slots.
Other text values keep their quotes. Expressions are preserved;
the generator does not evaluate `join`, arithmetic, or Scratch's runtime conversions.
Scratch math dropdown blocks render as Rust floating-point methods, such as
`(9 as f64).sqrt()` and `(30 as f64).to_radians().sin()`. Inverse trigonometric
results convert back to degrees to match Scratch.

## Block coverage

All **180 concrete block names** in the requested list have AIDA generation support.
This covers literal and menu blocks; control, data, event, looks, motion, operator,
sensing and sound blocks; the listed extension demonstrations; and procedure and
argument blocks. The mappings use the input and field names from these
[Scratch Blocks definitions](https://github.com/scratchfoundation/scratch-blocks/tree/3faed99e7feb8908687db6cc05236c5b40a668c6).

The [block fixtures](../src/aida/block_cases.tsv) contain exact inputs, fields and
expected AIDA output for 176 blocks. Procedure graph tests cover the four remaining
`procedures_*` blocks. The fixtures are fixed expectations, independent of the
generator's mapping table.

These 13 entries are module/category/toolbox names rather than serialized blocks:
`colour`, `math`, `texts`, `control`, `data`, `defaultToolbox`, `event`, `extensions`,
`looks`, `motion`, `operators`, `sensing`, and `sound`.

`procedures_declaration` and `argument_editor_*` describe custom-block editor data.
Declarations render as `declare:` signatures; argument editor/reporters use
`argument(name)`. Prototypes supply definitions with their names, parameter order,
defaults and warp flag. They do not become executable scripts on their own.

The listed `extension_*` names come from Scratch Blocks' extension demonstrations.
They have readable AIDA representations. The generator also supports all 14
serialized [Scratch 3 Pen blocks and menu](https://github.com/scratchfoundation/scratch-vm/blob/develop/src/extensions/scratch3_pen/index.js),
including `pen_penDown`, `pen_penUp`, clear, stamp, color, size, and legacy hue/shade
commands. Other VM extension opcodes such as `music_playDrumForBeats` are separate
names and require separate mappings.
This project generates an intermediate representation; it does not execute blocks
or operate extension hardware. Legacy blocks are preserved as corresponding AIDA
operations, without inventing execution behavior for them.

## Traversal and procedures

Supported event hats and procedure definitions start scripts. Statements follow
`next`; control blocks recursively follow `SUBSTACK` and `SUBSTACK2`. Reporters
recursively follow active input references, preserving arithmetic and Boolean
grouping. Hidden shadow inputs do not replace their connected reporter. Empty
Boolean sockets become `false`. Disconnected reporters are omitted.

```text
on key_pressed("space") {
    call: greet("hello", true)
}

define: greet(message: value = "", enabled: bool = false) warp {
    if argument(enabled) {
        say argument(message)
    }
}
```

Calls retain the argument order from procedure mutation IDs. Recursive calls remain
calls. `%s` parameters use `value` (Scratch permits strings and numbers), `%n` uses
`number`, and `%b` uses `bool`. `warp` records execution without screen refresh.
The `for_each:` range uses `from 1 through value` to include the upper bound.

Saved procedure metadata can contain trailing defaults for parameters that no
longer exist. Definitions use only defaults corresponding to current argument IDs,
matching [Scratch's procedure argument handling](https://github.com/scratchfoundation/scratch-vm/blob/develop/src/blocks/scratch3_procedures.js).
Extra defaults do not cause the definition or its body to be omitted.

Unknown opcodes, missing references, malformed inputs and graph cycles produce
visible `unknown` placeholders. Invalid compact variable/list records return a
parse error. Original Scratch IDs and procedure mutation data remain in the parser.

## Files

- `src/parser.rs`: targets, variables, lists, blocks, procedure metadata and input decoding.
- `src/aida.rs`: AIDA output, naming, graph traversal and nested expressions.
- `src/rust/mod.rs`: public Rust-generator entry point.
- `src/rust/project.rs`: generated module setup and saved variable declarations.
- `src/rust/scripts.rs`: supported event-hat selection and script ordering.
- `src/rust/statements.rs`: statement-chain traversal and statement lowering.
- `src/rust/expressions.rs`: reporter and operator lowering.
- `src/rust/literals.rs`: Rust literal, field, and line formatting helpers.
- `src/aida/blocks.rs`: ordinary commands, reporters, menus and hats.
- `src/aida/procedures.rs`: procedure signatures, arguments and calls.
- `src/aida/tests.rs` and `block_cases.tsv`: regression and coverage checks.

Run `cargo fmt --check`, `cargo check`, and `cargo test` to verify changes.
