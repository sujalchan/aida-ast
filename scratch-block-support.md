# Scratch block support

This is the opcode-level support snapshot for AIDA's Scratch 3 generator and runtime. It lists 170 standard Scratch block cases and four custom-procedure opcodes. It omits 11 dropdown menu shadow opcodes because Scratch stores those selections as inputs to parent blocks rather than dispatching the menu shadows as standalone VM primitives. Their selected field values still flow through AIDA's parent-block lowering where relevant. The TSV also contains extension examples used by generator tests; those examples and third-party extensions are omitted from this catalog.

The opcode inventory follows Scratch VM's standard block packages: [control](https://github.com/scratchfoundation/scratch-vm/blob/develop/src/blocks/scratch3_control.js), [data](https://github.com/scratchfoundation/scratch-vm/blob/develop/src/blocks/scratch3_data.js), [events](https://github.com/scratchfoundation/scratch-vm/blob/develop/src/blocks/scratch3_event.js), [looks](https://github.com/scratchfoundation/scratch-vm/blob/develop/src/blocks/scratch3_looks.js), [motion](https://github.com/scratchfoundation/scratch-vm/blob/develop/src/blocks/scratch3_motion.js), [operators](https://github.com/scratchfoundation/scratch-vm/blob/develop/src/blocks/scratch3_operators.js), [procedures](https://github.com/scratchfoundation/scratch-vm/blob/develop/src/blocks/scratch3_procedures.js), [sensing](https://github.com/scratchfoundation/scratch-vm/blob/develop/src/blocks/scratch3_sensing.js), and [sound](https://github.com/scratchfoundation/scratch-vm/blob/develop/src/blocks/scratch3_sound.js). AIDA's explicit text mappings are in [`src/aida/blocks.rs`](src/aida/blocks.rs); control-flow and procedure rendering is also implemented in [`src/aida.rs`](src/aida.rs) and [`src/aida/procedures.rs`](src/aida/procedures.rs).

## Status key

- **✓**: AIDA generates Scratch-specific AIDA text, or the Rust backend implements the block's runtime behavior.
- **-**: AIDA renders the block, but it is structural data, only works as a parent block input, or is supported only as a project monitor. This includes procedure definition blocks, click hats, greater-than sensor hats, and motion/size reporters currently exposed as monitors rather than script inputs.
- **x**: the Rust runtime backend has no implementation; generated code reaches an unsupported-block placeholder if the block executes.

`AIDA text output` describes the text translator, not execution. Runtime marks describe code paths in [`src/rust/expressions.rs`](src/rust/expressions.rs), [`src/rust/statements.rs`](src/rust/statements.rs), and [`src/rust/scripts.rs`](src/rust/scripts.rs); they do not promise parity for every Scratch edge case.

| Scratch opcode | AIDA text output | Rust runtime |
|---|:---:|:---:|
| **Catalog** | **170 Scratch block cases + 4 procedure opcodes** | **92 implemented ✓, 19 partial -, 63 not implemented x** |

### Control

| Scratch opcode | AIDA text output | Rust runtime |
|---|:---:|:---:|
| `control_all_at_once` | ✓ | x |
| `control_clear_counter` | ✓ | x |
| `control_create_clone_of` | ✓ | ✓ |
| `control_delete_this_clone` | ✓ | ✓ |
| `control_for_each` | ✓ | x |
| `control_forever` | ✓ | ✓ |
| `control_get_counter` | ✓ | x |
| `control_if` | ✓ | ✓ |
| `control_if_else` | ✓ | ✓ |
| `control_incr_counter` | ✓ | x |
| `control_repeat` | ✓ | ✓ |
| `control_repeat_until` | ✓ | ✓ |
| `control_start_as_clone` | ✓ | ✓ |
| `control_stop` | ✓ | ✓ |
| `control_wait` | ✓ | ✓ |
| `control_wait_until` | ✓ | ✓ |
| `control_while` | ✓ | ✓ |

### Data: variables and lists

| Scratch opcode | AIDA text output | Rust runtime |
|---|:---:|:---:|
| `data_addtolist` | ✓ | ✓ |
| `data_changevariableby` | ✓ | ✓ |
| `data_deletealloflist` | ✓ | ✓ |
| `data_deleteoflist` | ✓ | ✓ |
| `data_hidelist` | ✓ | ✓ |
| `data_hidevariable` | ✓ | ✓ |
| `data_insertatlist` | ✓ | ✓ |
| `data_itemnumoflist` | ✓ | ✓ |
| `data_itemoflist` | ✓ | ✓ |
| `data_lengthoflist` | ✓ | ✓ |
| `data_listcontainsitem` | ✓ | ✓ |
| `data_listcontents` | ✓ | ✓ |
| `data_listindexall` | ✓ | ✓ |
| `data_listindexrandom` | ✓ | ✓ |
| `data_replaceitemoflist` | ✓ | ✓ |
| `data_setvariableto` | ✓ | ✓ |
| `data_showlist` | ✓ | ✓ |
| `data_showvariable` | ✓ | ✓ |
| `data_variable` | ✓ | ✓ |

### Events

| Scratch opcode | AIDA text output | Rust runtime |
|---|:---:|:---:|
| `event_broadcast` | ✓ | ✓ |
| `event_broadcastandwait` | ✓ | ✓ |
| `event_whenbackdropswitchesto` | ✓ | ✓ |
| `event_whenbroadcastreceived` | ✓ | ✓ |
| `event_whenflagclicked` | ✓ | ✓ |
| `event_whengreaterthan` | ✓ | - |
| `event_whenkeypressed` | ✓ | ✓ |
| `event_whenstageclicked` | ✓ | - |
| `event_whenthisspriteclicked` | ✓ | - |
| `event_whentouchingobject` | ✓ | - |

### Looks

| Scratch opcode | AIDA text output | Rust runtime |
|---|:---:|:---:|
| `looks_backdropnumbername` | ✓ | ✓ |
| `looks_backdrops` | ✓ | - |
| `looks_changeeffectby` | ✓ | x |
| `looks_changesizeby` | ✓ | x |
| `looks_changestretchby` | ✓ | x |
| `looks_cleargraphiceffects` | ✓ | x |
| `looks_costume` | ✓ | - |
| `looks_costumenumbername` | ✓ | x |
| `looks_goforwardbackwardlayers` | ✓ | x |
| `looks_gotofrontback` | ✓ | x |
| `looks_hide` | ✓ | x |
| `looks_hideallsprites` | ✓ | x |
| `looks_nextbackdrop` | ✓ | ✓ |
| `looks_nextcostume` | ✓ | x |
| `looks_say` | ✓ | ✓ |
| `looks_sayforsecs` | ✓ | ✓ |
| `looks_seteffectto` | ✓ | x |
| `looks_setsizeto` | ✓ | x |
| `looks_setstretchto` | ✓ | x |
| `looks_show` | ✓ | x |
| `looks_size` | ✓ | - |
| `looks_switchbackdropto` | ✓ | ✓ |
| `looks_switchbackdroptoandwait` | ✓ | x |
| `looks_switchcostumeto` | ✓ | x |
| `looks_think` | ✓ | ✓ |
| `looks_thinkforsecs` | ✓ | ✓ |

### Motion

| Scratch opcode | AIDA text output | Rust runtime |
|---|:---:|:---:|
| `motion_align_scene` | ✓ | x |
| `motion_changexby` | ✓ | ✓ |
| `motion_changeyby` | ✓ | ✓ |
| `motion_direction` | ✓ | - |
| `motion_glidesecstoxy` | ✓ | ✓ |
| `motion_glideto` | ✓ | ✓ |
| `motion_goto` | ✓ | ✓ |
| `motion_gotoxy` | ✓ | ✓ |
| `motion_ifonedgebounce` | ✓ | x |
| `motion_movesteps` | ✓ | x |
| `motion_pointindirection` | ✓ | ✓ |
| `motion_pointtowards` | ✓ | ✓ |
| `motion_scroll_right` | ✓ | x |
| `motion_scroll_up` | ✓ | x |
| `motion_setrotationstyle` | ✓ | x |
| `motion_setx` | ✓ | ✓ |
| `motion_sety` | ✓ | ✓ |
| `motion_turnleft` | ✓ | x |
| `motion_turnright` | ✓ | x |
| `motion_xposition` | ✓ | - |
| `motion_xscroll` | ✓ | x |
| `motion_yposition` | ✓ | - |
| `motion_yscroll` | ✓ | x |

### Operators

| Scratch opcode | AIDA text output | Rust runtime |
|---|:---:|:---:|
| `operator_add` | ✓ | ✓ |
| `operator_and` | ✓ | ✓ |
| `operator_contains` | ✓ | ✓ |
| `operator_divide` | ✓ | ✓ |
| `operator_equals` | ✓ | ✓ |
| `operator_gt` | ✓ | ✓ |
| `operator_join` | ✓ | ✓ |
| `operator_length` | ✓ | ✓ |
| `operator_letter_of` | ✓ | ✓ |
| `operator_lt` | ✓ | ✓ |
| `operator_mathop` | ✓ | ✓ |
| `operator_mod` | ✓ | ✓ |
| `operator_multiply` | ✓ | ✓ |
| `operator_not` | ✓ | ✓ |
| `operator_or` | ✓ | ✓ |
| `operator_random` | ✓ | ✓ |
| `operator_round` | ✓ | ✓ |
| `operator_subtract` | ✓ | ✓ |

### Sensing

| Scratch opcode | AIDA text output | Rust runtime |
|---|:---:|:---:|
| `sensing_answer` | ✓ | x |
| `sensing_askandwait` | ✓ | x |
| `sensing_coloristouchingcolor` | ✓ | x |
| `sensing_current` | ✓ | ✓ |
| `sensing_dayssince2000` | ✓ | ✓ |
| `sensing_distanceto` | ✓ | x |
| `sensing_keyoptions` | ✓ | - |
| `sensing_keypressed` | ✓ | ✓ |
| `sensing_loud` | ✓ | x |
| `sensing_loudness` | ✓ | ✓ |
| `sensing_mousedown` | ✓ | ✓ |
| `sensing_mousex` | ✓ | ✓ |
| `sensing_mousey` | ✓ | ✓ |
| `sensing_online` | ✓ | ✓ |
| `sensing_of` | ✓ | x |
| `sensing_resettimer` | ✓ | ✓ |
| `sensing_setdragmode` | ✓ | x |
| `sensing_timer` | ✓ | ✓ |
| `sensing_touchingcolor` | ✓ | x |
| `sensing_touchingobject` | ✓ | x |
| `sensing_userid` | ✓ | x |
| `sensing_username` | ✓ | ✓ |

### Sound

| Scratch opcode | AIDA text output | Rust runtime |
|---|:---:|:---:|
| `sound_changeeffectby` | ✓ | x |
| `sound_changevolumeby` | ✓ | x |
| `sound_cleareffects` | ✓ | x |
| `sound_play` | ✓ | x |
| `sound_playuntildone` | ✓ | x |
| `sound_seteffectto` | ✓ | x |
| `sound_setvolumeto` | ✓ | x |
| `sound_stopallsounds` | ✓ | x |
| `sound_volume` | ✓ | x |

### Custom blocks and arguments

| Scratch opcode | AIDA text output | Rust runtime |
|---|:---:|:---:|
| `argument_editor_boolean` | ✓ | - |
| `argument_editor_string_number` | ✓ | - |
| `argument_reporter_boolean` | ✓ | ✓ |
| `argument_reporter_string_number` | ✓ | ✓ |
| `procedures_call` | ✓ | ✓ |
| `procedures_declaration` | ✓ | - |
| `procedures_definition` | ✓ | - |
| `procedures_prototype` | ✓ | - |

### Literals and editor fields

| Scratch opcode | AIDA text output | Rust runtime |
|---|:---:|:---:|
| `colour_picker` | ✓ | - |
| `math_angle` | ✓ | ✓ |
| `math_integer` | ✓ | ✓ |
| `math_number` | ✓ | ✓ |
| `math_positive_number` | ✓ | ✓ |
| `math_whole_number` | ✓ | ✓ |
| `matrix` | ✓ | - |
| `note` | ✓ | - |
| `text` | ✓ | ✓ |

### Pen extension

These opcodes are recognized and rendered by AIDA's text generator. Their Rust
runtime status remains unsupported (`x`); this table does not claim that the
headless runtime draws or changes pen state.

| Scratch opcode | AIDA text output | Rust runtime |
|---|:---:|:---:|
| `pen_changePenColorParamBy` | ✓ | x |
| `pen_changePenHueBy` | ✓ | x |
| `pen_changePenShadeBy` | ✓ | x |
| `pen_changePenSizeBy` | ✓ | x |
| `pen_clear` | ✓ | x |
| `pen_penDown` | ✓ | x |
| `pen_penUp` | ✓ | x |
| `pen_setPenColorParamTo` | ✓ | x |
| `pen_setPenColorToColor` | ✓ | x |
| `pen_setPenHueToNumber` | ✓ | x |
| `pen_setPenShadeToNumber` | ✓ | x |
| `pen_setPenSizeTo` | ✓ | x |
| `pen_stamp` | ✓ | x |

## Live monitor sources

Monitor display support does not imply that a reporter can be used by a script or that it is a Scratch variable. Only `data_variable` refers to a project variable; variable monitors resolve the same ID and owner scope as that reporter.

| Monitor | Live value source |
|---|---|
| `data_variable`, `data_listcontents` | Canonical variable or complete list state. Lists render only the visible row range. |
| `motion_xposition`, `motion_yposition`, `motion_direction`, `looks_size` | Snapshot for the monitor's named sprite. These remain monitor-only reporter support. |
| `looks_backdropnumbername` | Selected Stage backdrop name or one-based number. |
| `sensing_current`, `sensing_dayssince2000`, `sensing_mousex`, `sensing_mousey`, `sensing_mousedown` | Current runtime values, refreshed independently from script reporter evaluation. |
| `sensing_timer` | Renderer clock, updated while the monitor is visible. |
| `sensing_keypressed`, `sensing_online`, `sensing_username`, `sensing_loudness` | Runtime reporter values; online is `false`, username defaults to `playerNNNN`, and loudness is `-1` without microphone input. |

Other reporter monitors can display their saved project value, but are not live unless a source is listed above. Monitor data is batched at the renderer cadence; do not add monitor polling to generated blocks or variable-read hot paths.
