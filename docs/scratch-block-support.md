# Scratch block support

This is a snapshot of AIDA's Scratch 3 block support. The inventory contains the 176 concrete opcodes in [`src/aida/block_cases.tsv`](../src/aida/block_cases.tsv) plus the four custom-procedure opcodes covered by procedure tests. It includes core blocks, editor menus and literals, legacy opcodes, AIDA's listed extension examples, and Pen blocks. It does not attempt to list third-party extension opcodes that are not in this repository's catalog.

The opcode inventory follows Scratch VM's standard block packages: [control](https://github.com/scratchfoundation/scratch-vm/blob/develop/src/blocks/scratch3_control.js), [data](https://github.com/scratchfoundation/scratch-vm/blob/develop/src/blocks/scratch3_data.js), [events](https://github.com/scratchfoundation/scratch-vm/blob/develop/src/blocks/scratch3_event.js), [looks](https://github.com/scratchfoundation/scratch-vm/blob/develop/src/blocks/scratch3_looks.js), [motion](https://github.com/scratchfoundation/scratch-vm/blob/develop/src/blocks/scratch3_motion.js), [operators](https://github.com/scratchfoundation/scratch-vm/blob/develop/src/blocks/scratch3_operators.js), [procedures](https://github.com/scratchfoundation/scratch-vm/blob/develop/src/blocks/scratch3_procedures.js), [sensing](https://github.com/scratchfoundation/scratch-vm/blob/develop/src/blocks/scratch3_sensing.js), and [sound](https://github.com/scratchfoundation/scratch-vm/blob/develop/src/blocks/scratch3_sound.js). AIDA's explicit text mappings are in [`src/aida/blocks.rs`](../src/aida/blocks.rs); control-flow and procedure rendering is also implemented in [`src/aida.rs`](../src/aida.rs) and [`src/aida/procedures.rs`](../src/aida/procedures.rs).

## Status key

- **✓**: AIDA generates Scratch-specific AIDA text, or the Rust backend implements the block's runtime behavior.
- **-**: AIDA renders the block, but Rust support is structural, partial, or compile-time only. This includes menu blocks, event hats that are not started by the runtime, and list visibility without a visual monitor.
- **x**: the Rust runtime backend has no implementation; generated code reaches an unsupported-block placeholder if the block executes.

`AIDA text output` is a translation format, not an execution engine. A check in that column means the block has a text representation; it does not imply Scratch VM-equivalent runtime behavior. Runtime status is based on the generated Rust implementation in [`src/rust/expressions.rs`](../src/rust/expressions.rs), [`src/rust/statements.rs`](../src/rust/statements.rs), and [`src/rust/scripts.rs`](../src/rust/scripts.rs). These marks describe implemented code paths, not a guarantee of parity for every Scratch edge case.

| Scratch opcode | AIDA text output | Rust runtime |
|---|:---:|:---:|
| **Catalog** | **176 block cases + 4 procedure opcodes** | **52 implemented ✓, 33 partial -, 95 not implemented x** |

### Control

| Scratch opcode | AIDA text output | Rust runtime |
|---|:---:|:---:|
| `control_all_at_once` | ✓ | x |
| `control_clear_counter` | ✓ | x |
| `control_create_clone_of` | ✓ | ✓ |
| `control_create_clone_of_menu` | ✓ | - |
| `control_delete_this_clone` | ✓ | x |
| `control_for_each` | ✓ | x |
| `control_forever` | ✓ | ✓ |
| `control_get_counter` | ✓ | x |
| `control_if` | ✓ | ✓ |
| `control_if_else` | ✓ | ✓ |
| `control_incr_counter` | ✓ | x |
| `control_repeat` | ✓ | ✓ |
| `control_repeat_until` | ✓ | ✓ |
| `control_start_as_clone` | ✓ | ✓ |
| `control_stop` | ✓ | x |
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
| `data_hidelist` | ✓ | - |
| `data_hidevariable` | ✓ | x |
| `data_insertatlist` | ✓ | ✓ |
| `data_itemnumoflist` | ✓ | ✓ |
| `data_itemoflist` | ✓ | ✓ |
| `data_lengthoflist` | ✓ | ✓ |
| `data_listcontainsitem` | ✓ | ✓ |
| `data_listcontents` | ✓ | ✓ |
| `data_listindexall` | ✓ | - |
| `data_listindexrandom` | ✓ | - |
| `data_replaceitemoflist` | ✓ | ✓ |
| `data_setvariableto` | ✓ | ✓ |
| `data_showlist` | ✓ | - |
| `data_showvariable` | ✓ | x |
| `data_variable` | ✓ | ✓ |

### Events

| Scratch opcode | AIDA text output | Rust runtime |
|---|:---:|:---:|
| `event_broadcast` | ✓ | x |
| `event_broadcast_menu` | ✓ | - |
| `event_broadcastandwait` | ✓ | x |
| `event_touchingobjectmenu` | ✓ | - |
| `event_whenbackdropswitchesto` | ✓ | - |
| `event_whenbroadcastreceived` | ✓ | - |
| `event_whenflagclicked` | ✓ | ✓ |
| `event_whengreaterthan` | ✓ | - |
| `event_whenkeypressed` | ✓ | - |
| `event_whenstageclicked` | ✓ | - |
| `event_whenthisspriteclicked` | ✓ | - |
| `event_whentouchingobject` | ✓ | - |

### Looks

| Scratch opcode | AIDA text output | Rust runtime |
|---|:---:|:---:|
| `looks_backdropnumbername` | ✓ | x |
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
| `looks_nextbackdrop` | ✓ | x |
| `looks_nextcostume` | ✓ | x |
| `looks_say` | ✓ | ✓ |
| `looks_sayforsecs` | ✓ | x |
| `looks_seteffectto` | ✓ | x |
| `looks_setsizeto` | ✓ | x |
| `looks_setstretchto` | ✓ | x |
| `looks_show` | ✓ | x |
| `looks_size` | ✓ | x |
| `looks_switchbackdropto` | ✓ | x |
| `looks_switchbackdroptoandwait` | ✓ | x |
| `looks_switchcostumeto` | ✓ | x |
| `looks_think` | ✓ | x |
| `looks_thinkforsecs` | ✓ | x |

### Motion

| Scratch opcode | AIDA text output | Rust runtime |
|---|:---:|:---:|
| `motion_align_scene` | ✓ | x |
| `motion_changexby` | ✓ | x |
| `motion_changeyby` | ✓ | x |
| `motion_direction` | ✓ | x |
| `motion_glidesecstoxy` | ✓ | - |
| `motion_glideto` | ✓ | x |
| `motion_glideto_menu` | ✓ | - |
| `motion_goto` | ✓ | x |
| `motion_goto_menu` | ✓ | - |
| `motion_gotoxy` | ✓ | x |
| `motion_ifonedgebounce` | ✓ | x |
| `motion_movesteps` | ✓ | x |
| `motion_pointindirection` | ✓ | x |
| `motion_pointtowards` | ✓ | x |
| `motion_pointtowards_menu` | ✓ | - |
| `motion_scroll_right` | ✓ | x |
| `motion_scroll_up` | ✓ | x |
| `motion_setrotationstyle` | ✓ | x |
| `motion_setx` | ✓ | x |
| `motion_sety` | ✓ | x |
| `motion_turnleft` | ✓ | x |
| `motion_turnright` | ✓ | x |
| `motion_xposition` | ✓ | x |
| `motion_xscroll` | ✓ | x |
| `motion_yposition` | ✓ | x |
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
| `sensing_current` | ✓ | x |
| `sensing_dayssince2000` | ✓ | x |
| `sensing_distanceto` | ✓ | x |
| `sensing_distancetomenu` | ✓ | - |
| `sensing_keyoptions` | ✓ | - |
| `sensing_keypressed` | ✓ | x |
| `sensing_loud` | ✓ | x |
| `sensing_loudness` | ✓ | x |
| `sensing_mousedown` | ✓ | x |
| `sensing_mousex` | ✓ | x |
| `sensing_mousey` | ✓ | x |
| `sensing_of` | ✓ | x |
| `sensing_of_object_menu` | ✓ | - |
| `sensing_resettimer` | ✓ | x |
| `sensing_setdragmode` | ✓ | x |
| `sensing_timer` | ✓ | x |
| `sensing_touchingcolor` | ✓ | x |
| `sensing_touchingobject` | ✓ | x |
| `sensing_touchingobjectmenu` | ✓ | - |
| `sensing_userid` | ✓ | x |
| `sensing_username` | ✓ | x |

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
| `sound_sounds_menu` | ✓ | - |
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

### Literals and menus

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

### Extension examples

| Scratch opcode | AIDA text output | Rust runtime |
|---|:---:|:---:|
| `extension_microbit_display` | ✓ | x |
| `extension_music_drum` | ✓ | x |
| `extension_music_play_note` | ✓ | x |
| `extension_music_reporter` | ✓ | x |
| `extension_pen_down` | ✓ | x |
| `extension_wedo_boolean` | ✓ | x |
| `extension_wedo_hat` | ✓ | x |
| `extension_wedo_motor` | ✓ | x |
| `extension_wedo_tilt_menu` | ✓ | x |
| `extension_wedo_tilt_reporter` | ✓ | x |

## Reading this checklist

The Rust runtime is headless. For example, `looks_say` writes to the console, while unsupported visual, sensing, audio, and extension blocks are not simulated. Some menus and procedure definitions are data used by their parent/call blocks rather than runnable commands.

The counts and marks are a manually reviewed snapshot. Update this file when the Rust block generator or runtime gains or loses support.
