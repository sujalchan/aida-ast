# Scratch to AIDA

`cargo run` reads `example.sb3`, keeps the existing console debug output, and
writes `project.aida`.

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
Text primitive tags keep text, even when it looks numeric. Expressions are preserved;
the generator does not evaluate `join`, arithmetic, or Scratch's runtime conversions.

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
They have readable AIDA representations. Modern VM extension opcodes such as
`music_playDrumForBeats` are separate names and require separate mappings.
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

Unknown opcodes, missing references, malformed inputs and graph cycles produce
visible `unknown` placeholders. Invalid compact variable/list records return a
parse error. Original Scratch IDs and procedure mutation data remain in the parser.

## Files

- `src/parser.rs`: targets, variables, lists, blocks, procedure metadata and input decoding.
- `src/aida.rs`: naming, target output, graph traversal and nested expressions.
- `src/aida/blocks.rs`: ordinary commands, reporters, menus and hats.
- `src/aida/procedures.rs`: procedure signatures, arguments and calls.
- `src/aida/tests.rs` and `block_cases.tsv`: regression and coverage checks.

Run `cargo fmt --check`, `cargo check`, and `cargo test` to verify changes.
