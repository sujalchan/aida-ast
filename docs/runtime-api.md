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

`run` initializes shared `ScratchValue` state from the Scratch project, then
starts each green-flag script in stable block-ID order. Generated script and
procedure frames resume through the single-threaded runtime scheduler. Each
script runs until it completes or reaches a cooperative yield; a forever loop
therefore keeps `run()` active. `GeneratedProgram::step()` advances one
scheduler slot and returns to the host, so an application can interleave its
own work with script progress. `run()` is a convenience wrapper that continues
until all scripts complete.

Generated modules also expose `GeneratedProgram::new`, `new_with_clock`,
`advance`, `advance_frame`, `duration_until`, `restart`, `is_complete`, `state`,
and `run_until_complete` for applications that need to drive scripts
themselves. `advance()` runs at most one runnable script and returns
`Progressed`, `WaitingUntil(deadline)` or `Complete`. `advance_frame()` advances
each runnable script once in frame order, including clone and broadcast scripts
started during that frame. A host can schedule a timer for the returned deadline
and continue processing input and rendering without polling. `step()` remains a
compatibility convenience and returns false when no script can run now; use
`advance()` when waits must be distinguished from completion.

The headless `aida-runtime` launcher paces frames at 30 FPS by default (about
33.333 ms per frame). Pass `--fps <number>` to select another positive finite
rate, or `--turbo` to remove intentional frame pacing. The launcher measures
each frame with a monotonic clock and sleeps only for the remainder of its
target duration. Slow frames do not accumulate catch-up work. Turbo uses the
single-script `advance()` path, still honors Scratch waits, and sleeps until the
next script deadline when nothing is runnable. Paced execution and turbo can
therefore produce different script interleavings.

Use the flags with `./run.sh [path/to/project.sb3] --fps 60` or
`./run.sh [path/to/project.sb3] --turbo`. When launching the generated runtime
directly, pass Cargo arguments after `--`, for example
`cargo run -p aida-runtime -- --fps 60`.

`restart()` has green-flag semantics: it discards all active script and wait
frames, then starts a fresh instance of every green-flag script. Project
variables keep their current values, as Scratch's green flag stops threads and
starts the hats without resetting variable values. A future graphics layer
will also need to apply Scratch's sprite reset behavior, which this runtime
does not model.

The wait and green-flag behavior was checked against Scratch VM
[`v5.0.301-spork.1`](https://github.com/scratchfoundation/scratch-vm/tree/v5.0.301-spork.1)
(`cf22eeb`): [`runtime.greenFlag()`](https://github.com/scratchfoundation/scratch-vm/blob/v5.0.301-spork.1/src/engine/runtime.js)
stops active threads before starting green-flag hats, and
[`control_wait`](https://github.com/scratchfoundation/scratch-vm/blob/v5.0.301-spork.1/src/blocks/scratch3_control.js)
yields while its timer is pending. This runtime stores deadlines in integer
milliseconds and rounds fractional milliseconds up. Its host wake timing and
script ordering can differ from Scratch's frame-based sequencer.
The runtime's fixed target rate and turbo switch are headless host policies;
Scratch itself is driven by its rendering/frame loop, so exact wake and frame
timing can differ.

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
pub use scheduler::{Clock, ResumableScript, SchedulerStatus, ScriptScheduler, ScriptStatus, WarpBudget};
pub use value::{ScratchInput, ScratchState, ScratchValue, ScratchValueView};
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

- Green-flag hats: each becomes an independent resumable script sharing one
  live project state.
- Variable statements: `data_setvariableto` and `data_changevariableby`.
- Custom procedure definitions and calls. Calls push frames that retain the
  return location, dynamic arguments, loop counters, and inherited warp mode.
  Shared variables remain in the project's live state across yields.
- Loop statements: `control_repeat`, `control_repeat_until`, `control_while`,
  and `control_forever`. Their substacks are resumable; repeat counts use
  Scratch rounding, and boolean conditions use Scratch truth rules.
- `control_stop`: `this script` completes the current script, unwinding one
  custom-procedure frame when called inside a procedure; `other scripts in
  sprite` stops peer scripts on the current sprite or clone; `all` stops all
  scripts and removes clones.
- Clone blocks: `control_create_clone_of`, `control_start_as_clone`, and
  `control_delete_this_clone`. Clones inherit their source's position,
  clone-local variables, and lists; deleting a clone removes its active scripts.
- Broadcast blocks: `event_broadcast`, `event_broadcastandwait`, and
  `event_whenbroadcastreceived`. Broadcast receivers can restart, and waiting
  senders resume after matching receiver runs finish.
- `control_wait`: evaluates its duration once, stores an absolute wake deadline
  in the current script frame, and suspends only that script. A zero-duration
  wait still yields once. Waiting scripts do not occupy scheduler turns.
- `control_wait_until`: reevaluates its condition on resume. In warp mode its
  yield is suppressed until the warp time safeguard is reached.
- `looks_say` statements: emitted as `print_value!(name, expression)` at the same
  point in the script. For variable reporters, the macro prints the Scratch
  variable name and its value using Rust `Debug` formatting.
- Motion statements: `motion_gotoxy` and `motion_changexby` update the current
  sprite or clone's stored position. This headless runtime has no renderer.
- Arithmetic reporters: add, subtract, multiply, divide, and modulo.
- Comparison and boolean reporters: equals, greater than, less than, AND, OR,
  and NOT.
- Text reporters: join, letter of, length, and contains.
- Numeric reporters: random, round, and the math operator reporter (`abs`, `floor`,
  `ceiling`, `sqrt`, trig functions, logarithms, and exponentiation).
- Variable reporters and numeric/text literal reporters.
- Standard list reporters and statements: add, delete, delete all, insert,
  replace, get, item number, length, contains, contents, show, and hide. List
  identity is resolved from the Scratch list ID, so equal names in separate
  targets remain separate lists. Mutations use the same shared state as
  scheduled scripts. Clone-local list copies are isolated per clone.

`ScratchList` stores typed `ScratchValue`s in a `Vec`, uses Scratch's one-based
indices and `last`/`random`/`any`/`all` index forms where applicable, and caps
lists at 200,000 items. Appending to a full list is ignored. Inserting at a
valid position in a full list inserts the new value and drops the old tail;
appending beyond the limit is ignored. Invalid reads return empty text, while
invalid mutation indices have no effect. List monitor visibility is runtime
state separate from list contents. Saved monitor visibility is not loaded and
there is no graphics/monitor UI yet.

List conversion and limit behavior were checked against Scratch VM
`v5.0.301-spork.1` (`cf22eeb`), especially
[`Cast.toListIndex`](https://github.com/scratchfoundation/scratch-vm/blob/v5.0.301-spork.1/src/util/cast.js)
and the list opcodes in
[`scratch3_data.js`](https://github.com/scratchfoundation/scratch-vm/blob/v5.0.301-spork.1/src/blocks/scratch3_data.js).

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
panic if execution reaches them. See the [block support table](../scratch-block-support.md)
for the current opcode-by-opcode status. Motion support is limited to
`motion_gotoxy` and `motion_changexby`; other motion, visual, sensing, sound,
and extension behaviors are not simulated.

Normal and warp custom block calls use resumable procedure frames with owned
`ScratchValue` arguments. Normal loop boundaries yield while preserving the
caller and callee frames. Warp procedures suppress ordinary loop and
`control_wait_until` yields until a safe point observes that 500 ms have elapsed,
matching the Scratch VM warp budget. The runtime uses an injectable monotonic
clock so this limit can be tested without sleeping.

The Scratch VM's `control_wait` starts a timer and yields while that timer is
pending. The generated frame stores the evaluated deadline and the scheduler
reports the earliest deadline when all scripts are waiting. Inside warp mode,
the VM suppresses ordinary yields until its time budget expires; this
implementation follows that behavior by polling the wait at safe points until
the warp safeguard yields.

The blocking `run()` wrapper sleeps until the next deadline when every active
script is waiting. Event-loop hosts should use `advance()` and schedule the
returned deadline themselves. Infinite scripts intentionally keep the blocking
wrapper active; `is_complete()` reports when all finite scripts have been
removed from scheduling.

The separate AIDA text generator remains available for `.aida` output. Its
block coverage does not imply equivalent Rust lowering.
