# Scratch block support

This catalog inventories current Scratch runtime opcodes, editor and serialized-input helpers, and legacy compatibility blocks. It records executable behavior in the generated Rust runtime. Parsing editor shadows and structural procedure records is supported, but those records are not standalone runtime commands.

## Opcode audit

The opcode audit used Scratch Blocks `develop` at [`95817145`](https://github.com/scratchfoundation/scratch-blocks/tree/95817145c68b72e10282e36be403b2e0981c1359), including its [control](https://github.com/scratchfoundation/scratch-blocks/blob/95817145c68b72e10282e36be403b2e0981c1359/src/blocks/control.ts), [data](https://github.com/scratchfoundation/scratch-blocks/blob/95817145c68b72e10282e36be403b2e0981c1359/src/blocks/data.ts), and [procedure](https://github.com/scratchfoundation/scratch-blocks/blob/95817145c68b72e10282e36be403b2e0981c1359/src/blocks/procedures.ts) definitions, and Scratch VM `develop` at [`b3266a0`](https://github.com/scratchfoundation/scratch-vm/tree/b3266a0cfe5122f20b72ccd738a3dd4dff4fc5a5). Its [control](https://github.com/scratchfoundation/scratch-vm/blob/b3266a0cfe5122f20b72ccd738a3dd4dff4fc5a5/src/blocks/scratch3_control.js), [events](https://github.com/scratchfoundation/scratch-vm/blob/b3266a0cfe5122f20b72ccd738a3dd4dff4fc5a5/src/blocks/scratch3_event.js), [looks](https://github.com/scratchfoundation/scratch-vm/blob/b3266a0cfe5122f20b72ccd738a3dd4dff4fc5a5/src/blocks/scratch3_looks.js), [motion](https://github.com/scratchfoundation/scratch-vm/blob/b3266a0cfe5122f20b72ccd738a3dd4dff4fc5a5/src/blocks/scratch3_motion.js), [sensing](https://github.com/scratchfoundation/scratch-vm/blob/b3266a0cfe5122f20b72ccd738a3dd4dff4fc5a5/src/blocks/scratch3_sensing.js), and [Pen](https://github.com/scratchfoundation/scratch-vm/blob/b3266a0cfe5122f20b72ccd738a3dd4dff4fc5a5/src/extensions/scratch3_pen/index.js) primitive maps distinguish executable hidden blocks from retained compatibility no-ops; Scratch Blocks distinguishes runtime blocks from serialized menu and procedure-editor inputs. Runtime status marks are maintained against the generated Rust backend in [expressions](src/rust/expressions/), [statements](src/rust/statements/), and [scripts](src/rust/scripts/).

## Runtime opcode support

### **134 current Scratch runtime opcodes**

| Fully implemented | Partially implemented | Not implemented |
|---:|---:|---:|
| **121** | **0** | **13** |

These totals count only the current Scratch runtime opcode section below. Editor and serialized-input helpers and legacy compatibility opcodes are listed separately.

**Generated counts:** current runtime: 134 (121 implemented, 0 partial, 13 unsupported); editor and serialized helpers: 30; legacy no-ops: 9; legacy executable: 12; total classified identifiers: 185.

The totals summarize the inventory sections below.

## Status key

- **?**: the generated Rust runtime implements the listed Scratch behavior.
- **-**: partial support, such as a recognized event hat without runtime scheduling or a reporter supported only through its monitor path.
- **x**: the Rust backend does not execute this current Scratch block yet; imported projects remain parseable and unsupported behavior is shown in generated code rather than silently removed.
- **Input/structure** in the helper section means the serialized value or procedure metadata is consumed by its parent or by custom-procedure lowering; it is not a standalone runtime command.

## Current Scratch 3 runtime opcodes

These are Scratch VM blocks that represent runtime behavior, including hats, reporter reads, monitor-visible operations, sound, and Pen drawing. Unsupported behavior remains listed here with `x`.

| Scratch opcode | Rust runtime |
|---|:---:|

### Control

| Scratch opcode | Rust runtime |
|---|:---:|
| `control_forever` | ✓ |
| `control_repeat` | ✓ |
| `control_if` | ✓ |
| `control_if_else` | ✓ |
| `control_stop` | ✓ |
| `control_wait` | ✓ |
| `control_wait_until` | ✓ |
| `control_repeat_until` | ✓ |
| `control_start_as_clone` | ✓ |
| `control_create_clone_of` | ✓ |
| `control_delete_this_clone` | ✓ |

### Data: variables and lists

| Scratch opcode | Rust runtime |
|---|:---:|
| `data_variable` | ✓ |
| `data_setvariableto` | ✓ |
| `data_changevariableby` | ✓ |
| `data_showvariable` | ✓ |
| `data_hidevariable` | ✓ |
| `data_listcontents` | ✓ |
| `data_addtolist` | ✓ |
| `data_deleteoflist` | ✓ |
| `data_deletealloflist` | ✓ |
| `data_insertatlist` | ✓ |
| `data_replaceitemoflist` | ✓ |
| `data_itemoflist` | ✓ |
| `data_itemnumoflist` | ✓ |
| `data_lengthoflist` | ✓ |
| `data_listcontainsitem` | ✓ |
| `data_showlist` | ✓ |
| `data_hidelist` | ✓ |

### Events

| Scratch opcode | Rust runtime |
|---|:---:|
| `event_whenflagclicked` | ✓ |
| `event_whenthisspriteclicked` | ✓ |
| `event_whenstageclicked` | ✓ |
| `event_whenbroadcastreceived` | ✓ |
| `event_whenbackdropswitchesto` | ✓ |
| `event_whengreaterthan` | ✓ |
| `event_broadcast` | ✓ |
| `event_broadcastandwait` | ✓ |
| `event_whenkeypressed` | ✓ |

### Looks

| Scratch opcode | Rust runtime |
|---|:---:|
| `looks_sayforsecs` | ✓ |
| `looks_say` | ✓ |
| `looks_thinkforsecs` | ✓ |
| `looks_think` | ✓ |
| `looks_show` | ✓ |
| `looks_hide` | ✓ |
| `looks_changeeffectby` | x |
| `looks_seteffectto` | x |
| `looks_cleargraphiceffects` | x |
| `looks_changesizeby` | ✓ |
| `looks_setsizeto` | ✓ |
| `looks_size` | ✓ |
| `looks_switchcostumeto` | ✓ |
| `looks_nextcostume` | ✓ |
| `looks_switchbackdropto` | ✓ |
| `looks_gotofrontback` | x |
| `looks_goforwardbackwardlayers` | x |
| `looks_backdropnumbername` | ✓ |
| `looks_costumenumbername` | ✓ |
| `looks_switchbackdroptoandwait` | x |
| `looks_nextbackdrop` | ✓ |

### Motion

| Scratch opcode | Rust runtime |
|---|:---:|
| `motion_movesteps` | ✓ |
| `motion_turnright` | ✓ |
| `motion_turnleft` | ✓ |
| `motion_pointindirection` | ✓ |
| `motion_pointtowards` | ✓ |
| `motion_gotoxy` | ✓ |
| `motion_goto` | ✓ |
| `motion_glidesecstoxy` | ✓ |
| `motion_glideto` | ✓ |
| `motion_changexby` | ✓ |
| `motion_setx` | ✓ |
| `motion_changeyby` | ✓ |
| `motion_sety` | ✓ |
| `motion_ifonedgebounce` | ✓ |
| `motion_setrotationstyle` | ✓ |
| `motion_xposition` | ✓ |
| `motion_yposition` | ✓ |
| `motion_direction` | ✓ |

### Operators

| Scratch opcode | Rust runtime |
|---|:---:|
| `operator_add` | ✓ |
| `operator_subtract` | ✓ |
| `operator_multiply` | ✓ |
| `operator_divide` | ✓ |
| `operator_random` | ✓ |
| `operator_lt` | ✓ |
| `operator_equals` | ✓ |
| `operator_gt` | ✓ |
| `operator_and` | ✓ |
| `operator_or` | ✓ |
| `operator_not` | ✓ |
| `operator_join` | ✓ |
| `operator_letter_of` | ✓ |
| `operator_length` | ✓ |
| `operator_contains` | ✓ |
| `operator_mod` | ✓ |
| `operator_round` | ✓ |
| `operator_mathop` | ✓ |

### Sensing

| Scratch opcode | Rust runtime |
|---|:---:|
| `sensing_touchingobject` | x |
| `sensing_touchingcolor` | x |
| `sensing_coloristouchingcolor` | x |
| `sensing_distanceto` | x |
| `sensing_askandwait` | x |
| `sensing_answer` | x |
| `sensing_keypressed` | ✓ |
| `sensing_mousedown` | ✓ |
| `sensing_mousex` | ✓ |
| `sensing_mousey` | ✓ |
| `sensing_online` | ✓ |
| `sensing_setdragmode` | x |
| `sensing_loudness` | ✓ |
| `sensing_timer` | ✓ |
| `sensing_resettimer` | ✓ |
| `sensing_of` | ✓ |
| `sensing_current` | ✓ |
| `sensing_dayssince2000` | ✓ |
| `sensing_username` | ✓ |

### Sound

| Scratch opcode | Rust runtime |
|---|:---:|
| `sound_play` | ✓ |
| `sound_playuntildone` | ✓ |
| `sound_stopallsounds` | ✓ |
| `sound_seteffectto` | ✓ |
| `sound_changeeffectby` | ✓ |
| `sound_cleareffects` | ✓ |
| `sound_changevolumeby` | ✓ |
| `sound_setvolumeto` | ✓ |
| `sound_volume` | ✓ |

### Pen extension

| Scratch opcode | Rust runtime |
|---|:---:|
| `pen_clear` | ✓ |
| `pen_stamp` | ✓ |
| `pen_penDown` | ✓ |
| `pen_penUp` | ✓ |
| `pen_setPenColorToColor` | ✓ |
| `pen_changePenColorParamBy` | ✓ |
| `pen_setPenColorParamTo` | ✓ |
| `pen_changePenSizeBy` | ✓ |
| `pen_setPenSizeTo` | ✓ |

### My Blocks

| Scratch opcode | Rust runtime |
|---|:---:|
| `argument_reporter_boolean` | ✓ |
| `argument_reporter_string_number` | ✓ |
| `procedures_call` | ✓ |

## Editor and serialized input helpers

These records are retained in imported `.sb3` graphs and remain available to parent-block and procedure lowering. They are not standalone runtime opcodes and are excluded from the current runtime count.

### Dropdown menu shadows

| Scratch opcode | Role in the serialized project |
|---|---|
| `control_create_clone_of_menu` | clone target selected by `control_create_clone_of` |
| `data_listindexall` | index selector consumed by list reporters and mutations |
| `data_listindexrandom` | index selector consumed by list reporters and mutations |
| `event_broadcast_menu` | broadcast message name and ID consumed by event blocks |
| `event_touchingobjectmenu` | target selection serialized for the touching-object hat |
| `looks_backdrops` | backdrop dropdown value consumed by Stage backdrop blocks |
| `looks_costume` | costume dropdown value consumed by costume blocks |
| `motion_glideto_menu` | target selected by `motion_glideto` |
| `motion_goto_menu` | target selected by `motion_goto` |
| `motion_pointtowards_menu` | target selected by `motion_pointtowards` |
| `sensing_distancetomenu` | target selected by `sensing_distanceto` |
| `sensing_keyoptions` | key name consumed by key reporter and hat inputs |
| `sensing_of_object_menu` | object target selected by `sensing_of` |
| `sensing_touchingobjectmenu` | target selection consumed by touching reporters |
| `sound_sounds_menu` | sound asset name consumed by sound commands |
| `pen_menu_colorParam` | color parameter selected by Pen color commands |

### Literal and input shadows

| Scratch opcode | Role in the serialized project |
|---|---|
| `colour_picker` | color literal consumed by a parent input |
| `math_angle` | numeric literal input |
| `math_integer` | numeric literal input |
| `math_number` | numeric literal input |
| `math_positive_number` | numeric literal input |
| `math_whole_number` | numeric literal input |
| `matrix` | matrix literal input |
| `note` | note-number literal input |
| `text` | text literal input |

### Procedure editing and structure

| Scratch opcode | Role in the serialized project |
|---|---|
| `argument_editor_boolean` | custom-block boolean argument editor metadata |
| `argument_editor_string_number` | custom-block string/number argument editor metadata |
| `procedures_declaration` | custom-block declaration metadata |
| `procedures_definition` | identifies a procedure body that AIDA compiles |
| `procedures_prototype` | procedure signature, argument defaults, and warp metadata |

Numeric and text shadows still lower as values. List index shadows preserve their selected string (`1`, `last`, `all`, or `random`) for the consuming list operation. Procedure definitions and prototypes remain necessary input to generated custom-block frames.

## Legacy compatibility

Scratch VM keeps these serialized opcodes as no-ops. The generated runtime preserves their no-op behavior instead of emitting an unsupported-block panic. Reporter no-ops return the Scratch-compatible undefined value; numeric, string, and boolean conversions retain Scratch behavior.

| Scratch opcode | Scratch VM behavior | Rust runtime |
|---|---|:---:|
| `looks_hideallsprites` | no-op | ✓ |
| `looks_changestretchby` | no-op | ✓ |
| `looks_setstretchto` | no-op | ✓ |
| `motion_scroll_right` | no-op | ✓ |
| `motion_scroll_up` | no-op | ✓ |
| `motion_align_scene` | no-op | ✓ |
| `motion_xscroll` | returns undefined | ✓ |
| `motion_yscroll` | returns undefined | ✓ |
| `sensing_userid` | returns undefined | ✓ |

## Legacy executable compatibility

These hidden or older blocks have executable primitive or hat paths in Scratch VM but are outside the ordinary current palette. This section records compatibility behavior separately from current palette support.

| Scratch opcode | Rust runtime |
|---|:---:|
| `control_all_at_once` | x |
| `control_clear_counter` | x |
| `control_get_counter` | x |
| `control_incr_counter` | x |
| `control_for_each` | x |
| `control_while` | ✓ |
| `event_whentouchingobject` | - |
| `sensing_loud` | x |
| `pen_changePenHueBy` | ✓ |
| `pen_changePenShadeBy` | ✓ |
| `pen_setPenHueToNumber` | ✓ |
| `pen_setPenShadeToNumber` | ✓ |

## Live monitor sources

Monitor display support does not imply that a reporter can be used by a script or that it is a Scratch variable. Only `data_variable` refers to a project variable; variable monitors resolve the same ID and owner scope as that reporter.

| Monitor | Live value source |
|---|---|
| `data_variable`, `data_listcontents` | Canonical variable or complete list state. Lists render only the visible row range. |
| `motion_xposition`, `motion_yposition`, `motion_direction`, `looks_size` | Motion reporters read the scheduled sprite in scripts; monitors use the named sprite snapshot. |
| `looks_backdropnumbername` | Selected Stage backdrop name or one-based number. |
| `sensing_current`, `sensing_dayssince2000`, `sensing_mousex`, `sensing_mousey`, `sensing_mousedown` | Current runtime values, refreshed independently from script reporter evaluation. |
| `sensing_keypressed` | Saved project monitor records use live held-key state and their reporter input when its block is present; Scratch does not expose a standard checkbox monitor. |
| `sensing_timer` | Renderer clock, updated while the monitor is visible. |
| `sensing_online`, `sensing_username`, `sensing_loudness` | Runtime reporter values; online is `false`, username defaults to `playerNNNN`, and loudness is `-1` without microphone input. |

Other reporter monitors can display their saved project value, but are not live unless a source is listed above. Monitor data is batched at the renderer cadence; do not add monitor polling to generated blocks or variable-read hot paths.

Keyboard hats and `sensing_keypressed` share normalized held-key state. The key reporter evaluates its connected input in the executing target context.
