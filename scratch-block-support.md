# Scratch block support

This is a snapshot of AIDA's Scratch 3 block support. The table lists 180 core, editor-menu, literal, legacy, and Pen opcodes from [`src/aida/block_cases.tsv`](src/aida/block_cases.tsv), plus the four custom-procedure opcodes covered by procedure tests. It intentionally omits the extension-example opcodes and does not list third-party extensions.

The opcode inventory follows Scratch VM's standard block packages: [control](https://github.com/scratchfoundation/scratch-vm/blob/develop/src/blocks/scratch3_control.js), [data](https://github.com/scratchfoundation/scratch-vm/blob/develop/src/blocks/scratch3_data.js), [events](https://github.com/scratchfoundation/scratch-vm/blob/develop/src/blocks/scratch3_event.js), [looks](https://github.com/scratchfoundation/scratch-vm/blob/develop/src/blocks/scratch3_looks.js), [motion](https://github.com/scratchfoundation/scratch-vm/blob/develop/src/blocks/scratch3_motion.js), [operators](https://github.com/scratchfoundation/scratch-vm/blob/develop/src/blocks/scratch3_operators.js), [procedures](https://github.com/scratchfoundation/scratch-vm/blob/develop/src/blocks/scratch3_procedures.js), [sensing](https://github.com/scratchfoundation/scratch-vm/blob/develop/src/blocks/scratch3_sensing.js), and [sound](https://github.com/scratchfoundation/scratch-vm/blob/develop/src/blocks/scratch3_sound.js). AIDA's explicit text mappings are in [`src/aida/blocks.rs`](src/aida/blocks.rs); control-flow and procedure rendering is also implemented in [`src/aida.rs`](src/aida.rs) and [`src/aida/procedures.rs`](src/aida/procedures.rs).

## Status key

- **✓**: AIDA generates Scratch-specific AIDA text, or the Rust backend implements the block's runtime behavior.
- **-**: AIDA renders the block, but it is structural/menu data, only works as a parent block input, or is only supported through a project monitor. This includes procedure definition blocks, menu shadows, click hats, greater-than sensor hats, and motion/size reporters currently exposed as monitors rather than script inputs.
- **x**: the Rust runtime backend has no implementation; generated code reaches an unsupported-block placeholder if the block executes.

`AIDA text output` is a translation format, not an execution engine. A check in that column means the block has a text representation; it does not imply Scratch VM-equivalent runtime behavior. Runtime status is based on the generated Rust implementation in [`src/rust/expressions.rs`](src/rust/expressions.rs), [`src/rust/statements.rs`](src/rust/statements.rs), and [`src/rust/scripts.rs`](src/rust/scripts.rs). These marks describe implemented code paths, not a guarantee of parity for every Scratch edge case.

Broadcast support includes `event_broadcast`, `event_broadcastandwait`, and `event_whenbroadcastreceived`. The scheduler starts matching receivers, restarts an already active receiver when the same message is sent again, and waits for the started receiver runs for broadcast-and-wait. Matching receivers are created for live clones too; deleting a clone removes its scripts and releases senders waiting for them. The `broadcast-test.sb3` fixture exercises these paths, including a receiver that calls a waiting custom block. Keyboard hats and key/mouse reporters work in graphical mode. Backdrop-switch hats run when a different backdrop is selected. Click hats and greater-than sensor hats remain partial because AIDA does not yet generate those event sources.

Project monitors preserve their target ownership and reporter parameters when AIDA builds the renderer's value keys. Sprite-owned reporters are labeled with their owning sprite name; Stage backdrop number/name monitors read the selected Stage backdrop. Motion x/y/direction and Looks size monitors use the matching original sprite's runtime snapshot, so motion changes (including glide updates) refresh the displayed value. The renderer only marks a monitor dirty when its displayed value changes. This monitor support does not imply that the reporter opcode is available as a script input. Generic reporter monitors without a runtime-published value still show their project-provided initial value.

| Scratch opcode | AIDA text output | Rust runtime |
|---|:---:|:---:|
| **Catalog** | **180 block cases + 4 procedure opcodes** | **82 implemented ✓, 31 partial -, 71 not implemented x** |

### Control

| Scratch opcode | AIDA text output | Rust runtime |
|---|:---:|:---:|
| `control_all_at_once` | ✓ | x |
| `control_clear_counter` | ✓ | x |
| `control_create_clone_of` | ✓ | ✓ |
| `control_create_clone_of_menu` | ✓ | - |
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
| `data_showlist` | ✓ | ✓ |
| `data_showvariable` | ✓ | x |
| `data_variable` | ✓ | ✓ |

### Events

| Scratch opcode | AIDA text output | Rust runtime |
|---|:---:|:---:|
| `event_broadcast` | ✓ | ✓ |
| `event_broadcast_menu` | ✓ | - |
| `event_broadcastandwait` | ✓ | ✓ |
| `event_touchingobjectmenu` | ✓ | - |
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
| `motion_glideto_menu` | ✓ | - |
| `motion_goto` | ✓ | ✓ |
| `motion_goto_menu` | ✓ | - |
| `motion_gotoxy` | ✓ | ✓ |
| `motion_ifonedgebounce` | ✓ | x |
| `motion_movesteps` | ✓ | x |
| `motion_pointindirection` | ✓ | ✓ |
| `motion_pointtowards` | ✓ | ✓ |
| `motion_pointtowards_menu` | ✓ | - |
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
| `sensing_current` | ✓ | x |
| `sensing_dayssince2000` | ✓ | x |
| `sensing_distanceto` | ✓ | x |
| `sensing_distancetomenu` | ✓ | - |
| `sensing_keyoptions` | ✓ | - |
| `sensing_keypressed` | ✓ | ✓ |
| `sensing_loud` | ✓ | x |
| `sensing_loudness` | ✓ | x |
| `sensing_mousedown` | ✓ | ✓ |
| `sensing_mousex` | ✓ | ✓ |
| `sensing_mousey` | ✓ | ✓ |
| `sensing_of` | ✓ | x |
| `sensing_of_object_menu` | ✓ | - |
| `sensing_resettimer` | ✓ | x |
| `sensing_setdragmode` | ✓ | x |
| `sensing_timer` | ✓ | ✓ |
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
| `pen_menu_colorParam` | ✓ | x |
| `pen_penDown` | ✓ | x |
| `pen_penUp` | ✓ | x |
| `pen_setPenColorParamTo` | ✓ | x |
| `pen_setPenColorToColor` | ✓ | x |
| `pen_setPenHueToNumber` | ✓ | x |
| `pen_setPenShadeToNumber` | ✓ | x |
| `pen_setPenSizeTo` | ✓ | x |
| `pen_stamp` | ✓ | x |

## Reading this checklist

The Rust runtime can run headless or with the graphical renderer. `looks_say` writes its value to the console and displays a speech bubble in graphical mode; think bubbles work the same way without console output. Motion updates sprite and clone state, and backdrop switching updates the Stage. Variable/list data, clone state, broadcasts, waits, and supported event hats are integrated with the scheduler. The renderer has a separate drawable path for clones, but clone visibility in `motion-point-test.sb3` remains a reported graphical issue and is not marked as verified here. Other visual effects, costume changes, sound, pen blocks, and unsupported sensing remain unsimulated. Some menu blocks and procedure definition blocks are structural data used by their parent or call block rather than runnable commands.

The counts and marks are a manually reviewed snapshot. Update this file when the Rust block generator or runtime gains or loses support.

Motion position updates include `motion_setx` and `motion_sety`; position updates use SVG or bitmap costume bounds for Scratch-style fencing. Motion reporter monitors retain their `spriteName` owner and display that sprite's x, y, or direction state. `motion_pointtowards` handles random direction, mouse, and named sprite targets. Clone script state and transforms are independent in runtime snapshots; graphical clone visibility still needs verification in the reported fixture.
