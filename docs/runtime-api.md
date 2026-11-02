# Generated Rust and Runtime API

This document specifies the Rust interface emitted by `aida-ast --tests` and
the runtime helpers that generated code calls. It describes the current
implementation; it does not promise behavior for blocks that the generator
does not yet lower.

## Generated module contract

For each `.sb3` file directly inside `aida-ast/tests/src`, `aida-ast --tests`
writes a Rust file with the same base name to `aida-ast/tests/output`. The
generated file is a module for the runtime crate, not a standalone crate. Its
imports expect the runtime module to be available as `crate::runtime`.

Generated modules expose:

```rust
pub fn run()
```

`run` initializes local `ScratchValue` variables from the Scratch project,
then executes supported event script bodies in generated order. The current
implementation returns `()`; Scratch variables stay local to the function.
It does not schedule events concurrently.

Initial values come from the variable values saved in the `.sb3`: strings stay
`Text`, booleans stay `Boolean`, and numeric values are emitted as round-trip
`f64` literals. A `data_setvariableto` statement replaces the value with the
result's Scratch type, preserving text and boolean values instead of coercing
everything to a number.

Example integration:

```rust
mod runtime;
mod program;

fn main() {
    program::run();
}
```

The runtime module must publicly re-export `ScratchValue` and the helper
functions below. This is the interface generated modules use:

```rust
pub use operators::*;
pub use random::{ScratchRandom, scratch_random};
pub use value::{ScratchInput, ScratchValue, ScratchValueView};
```

## Values

`ScratchValue` is the runtime value type:

```rust
pub enum ScratchValue {
    Number(f64),
    Text(String),
    Boolean(bool),
}
```

The runtime implements `From<f64>`, `From<bool>`, `From<&str>`, and
`From<String>`. Generated variables are declared as `ScratchValue`; numbers,
strings, and booleans are converted with `.into()`. Operator helpers accept
`impl ScratchInput`, which supports literals and both owned and borrowed
`ScratchValue`s. Generated variable reporters pass references so operators
can inspect text without cloning its `String`.

## Operator helpers

Every argument listed as `Value` below has the Rust type `impl ScratchInput`.

| Helper | Signature result | Scratch operation |
| --- | --- | --- |
| `scratch_bool(value)` | `bool` | Scratch truth conversion |
| `scratch_to_number(value)` | `f64` | Scratch arithmetic numeric conversion |
| `scratch_add(left, right)` | `f64` | Numeric addition |
| `scratch_subtract(left, right)` | `f64` | Numeric subtraction |
| `scratch_multiply(left, right)` | `f64` | Numeric multiplication |
| `scratch_divide(left, right)` | `f64` | Numeric division |
| `scratch_mod(left, right)` | `f64` | Modulo using the divisor's sign |
| `scratch_equals(left, right)` | `bool` | Scratch equality |
| `scratch_greater_than(left, right)` | `bool` | Scratch greater-than comparison |
| `scratch_less_than(left, right)` | `bool` | Scratch less-than comparison |
| `scratch_compare(left, right)` | `Ordering` | Three-way Scratch comparison |
| `scratch_and(left, right)` | `bool` | Boolean AND after Scratch truth conversion |
| `scratch_or(left, right)` | `bool` | Boolean OR after Scratch truth conversion |
| `scratch_not(value)` | `bool` | Boolean NOT after Scratch truth conversion |
| `scratch_join(left, right)` | `String` | Concatenate Scratch text conversions |
| `scratch_letter(index, text)` | `String` | 1-based character lookup |
| `scratch_length(value)` | `f64` | Text length in UTF-16 code units |
| `scratch_contains(text, search)` | `bool` | Case-insensitive text containment |
| `scratch_round(value)` | `f64` | Round halfway values toward positive infinity |
| `scratch_random(from, to)` | `f64` | Random number; inclusive integer bounds when both Scratch inputs are integer-form values. Decimal-form text such as `"10.0"` selects a continuous range. |
| `scratch_sin(degrees)` | `f64` | Degree-based sine rounded to 10 decimal places |
| `scratch_cos(degrees)` | `f64` | Degree-based cosine rounded to 10 decimal places |
| `scratch_tan(degrees)` | `f64` | Degree-based tangent with Scratch rounding and singularities |
| `scratch_asin_degrees(value)` | `f64` | Inverse sine in degrees |
| `scratch_acos_degrees(value)` | `f64` | Inverse cosine in degrees |
| `scratch_atan_degrees(value)` | `f64` | Inverse tangent in degrees |

Numeric conversion treats booleans as `1` or `0`; trimmed numeric text parses as
a number, and invalid or empty numeric text becomes `0`. Truth conversion treats
zero, NaN, empty text, the text `0`, and case-insensitive `false` as false.
Comparisons compare numerically when both operands can be converted; otherwise
they compare lowercase text by UTF-16 code units.

## Floating-point precision

Scratch does not apply a general decimal-place limit when assigning values to
variables. The VM's `data_setvariableto` stores the reporter result directly.
The sine and cosine reporters themselves round their results to 10 decimal
places; tangent does too, except that it returns infinities at its exact
singularities (90° and -270° return positive infinity; -90° and 270° return
negative infinity). Other math reporters retain normal floating-point
precision. The Rust helpers `scratch_sin`, `scratch_cos`, and `scratch_tan`
mirror these rules.

## Current Rust lowering

The generator currently lowers:

- Event hats: green flag, sprite clicked, stage clicked, key pressed, and
  broadcast received. Their statement chains are emitted sequentially.
- Variable statements: `data_setvariableto` and `data_changevariableby`.
- Custom procedure definitions and calls. Definitions become Rust functions;
  calls pass dynamic Scratch arguments and mutable references to the variables
  used by the procedure body.
- Loop statements: `control_repeat`, `control_repeat_until`, and
  `control_while`. Their substacks are lowered recursively; repeat counts use
  Scratch rounding, and boolean conditions are converted using Scratch truth rules.
- `looks_say` statements: emitted as `print_value!(name, expression)` at the same
  point in the script. For variable reporters, the macro prints the Scratch
  variable name and its value using Rust `Debug` formatting.
- Arithmetic reporters: add, subtract, multiply, divide, and modulo.
- Comparison and boolean reporters: equals, greater than, less than, AND, OR,
  and NOT.
- Text reporters: join, letter of, length, and contains.
- Numeric reporters: random, round, and the math operator reporter (`abs`, `floor`,
  `ceiling`, `sqrt`, trig functions, logarithms, and exponentiation).
- Variable reporters and numeric/text literal reporters.

Arithmetic and text/boolean reporters call the matching `scratch_*` helper.
Known numeric literals go directly to the corresponding `f64` method, such as
`5.5_f64.floor()`. Other math operator inputs use `scratch_to_number(value)`
before calling the method. Sine, cosine, and tangent use dedicated runtime
helpers that match Scratch VM's 10-decimal result rounding; tangent also
preserves Scratch's exact ±90° and ±270° infinity cases. This keeps text and
number literals distinct in generated source while applying Scratch conversion
when a value is dynamic. Numeric comparisons use comparison helpers rather than
Rust's native operators, and preserve whether each input was represented as
text or numeric in the Scratch project.

`run()` does not add output or other behavior beyond the generated scripts.
Random values are non-deterministic by default. `ScratchRandom::seeded(seed)` is
available when a repeatable sequence is needed.

## Unsupported behavior

An unsupported reporter or malformed input emits a typed `todo!()` expression;
an unsupported statement emits `todo!()` as a statement. These compile, but
panic if execution reaches them. Lists and control blocks other than the listed
loops are not yet lowered by this Rust generator.

Custom block definitions are emitted as Rust functions with `ScratchValue`
arguments and mutable references for Scratch variables used by their bodies.
Procedure call blocks invoke those functions. The functions are only emitted
when the Scratch project defines them.

The separate AIDA text generator remains available for `.aida` output. Its
block coverage does not imply equivalent Rust lowering.
