// Copyright (c) 2026 AIDA AST contributers (see AUTHORS.md)
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

mod direct_numeric;
mod expressions;
mod literals;
mod project;
mod scripts;
mod statements;

pub use project::generate_with_list_data;
#[cfg(test)]
pub use project::{generate, generate_with_options};

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{generate, generate_with_list_data};
    use crate::parser::{ScratchBlock, parse_project};

    #[test]
    fn fixture_without_event_scripts_generates_one_broadcast_dispatcher() {
        let input = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/src/svg-bitmap.sb3");
        let json = crate::extractor::read_project_json(input).unwrap();
        let project = parse_project(&json).unwrap();
        let output = generate(&project);

        assert_eq!(output.matches("fn new_broadcast_scripts(").count(), 1);
    }

    #[test]
    fn wireframe_scripts_only_emit_reachable_procedure_bodies() {
        let input = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/src/wireframe-engine.sb3"
        );
        let json = crate::extractor::read_project_json(input).unwrap();
        let project = parse_project(&json).unwrap();
        let (output, _) = generate_with_list_data(&project, false, "wireframe.rs.lists.json");
        let (first, rest) = output
            .split_once("fn step_green_flag_script_engine_from_nowhere_2(")
            .unwrap();
        let (second, third) = rest
            .split_once("fn step_green_flag_script_engine_from_nowhere_3(")
            .unwrap();
        let third = third
            .split_once("fn new_green_flag_script_engine_from_nowhere_3_for(")
            .unwrap()
            .0;
        assert!(first.contains("fn direct_procedure_1_4("));
        assert!(first.contains("fn direct_procedure_1_4_numbers("));
        assert!(first.contains("cached_numeric_item_"));
        assert!(first.contains("script.warp_budget.expired(clock)"));
        assert!(!first.contains("fn direct_procedure_1_5("));
        assert!(first.contains("ProcedureFrame::new(8,"));
        assert!(first.contains("ProcedureFrame::new(4,"));
        assert!(!second.contains("ProcedureFrame::new(4,"));
        assert!(!third.contains("ProcedureFrame::new(4,"));
        assert!(!second.contains("script.frames.pop(); continue;"));
        assert!(!third.contains("script.frames.pop(); continue;"));
    }

    #[test]
    fn large_list_fixture_uses_compact_initializer_and_preserves_all_items() {
        let input = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/src/monitor-tests-sensing.sb3"
        );
        let json = crate::extractor::read_project_json(input).unwrap();
        let project = parse_project(&json).unwrap();
        let list = project
            .targets
            .iter()
            .flat_map(|target| target.lists.values())
            .find(|list| list.items.len() == 200_000)
            .expect("fixture should contain its complete 200,000-item list");
        assert!(list.items[0].is_number());
        assert!(list.items[100_000].is_number());
        assert!(list.items[199_999].is_number());

        let (output, list_data) = generate_with_list_data(&project, false, "large.rs.lists.json");
        let serialized_items = serde_json::to_string(&list.items).unwrap();
        assert!(output.contains("include_bytes!(\"large.rs.lists.json\")"));
        assert!(output.contains("ScratchList::from_json_bytes(&initial_list_data["));
        let list_data = list_data.expect("large list data should be externalized");
        assert!(
            list_data
                .windows(serialized_items.len())
                .any(|window| window == serialized_items.as_bytes())
        );
        assert!(
            output.len() < 2_000_000,
            "generated module was {} bytes",
            output.len()
        );
    }

    #[test]
    fn small_list_fixture_keeps_inline_initializers() {
        let input = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/src/lists.sb3");
        let json = crate::extractor::read_project_json(input).unwrap();
        let project = parse_project(&json).unwrap();
        let (output, list_data) = generate_with_list_data(&project, false, "lists.rs.lists.json");

        assert!(list_data.is_none());
        assert!(!output.contains("include_bytes!("));
        assert!(output.contains("ScratchList::from_json("));
    }

    #[test]
    fn pen_fixture_generates_pen_commands_and_costume_state_without_todos() {
        let input = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/src/pen-tests.sb3");
        let json = crate::extractor::read_project_json(input).unwrap();
        let project = parse_project(&json).unwrap();
        let output = generate(&project);

        for generated in [
            "state.clear_pen();",
            "state.pen_down();",
            "state.pen_up();",
            "state.stamp();",
            "state.set_pen_rgb_color(scratch_pen_color);",
            "state.set_pen_color_parameter(crate::runtime::PenColorParameter::Color,",
            "state.set_pen_size(",
            "state.switch_costume_to(",
            "state.set_visible(true);",
            "state.set_visible(false);",
            "state.costume_number_name(true)",
        ] {
            assert!(
                output.contains(generated),
                "missing generated pen path: {generated}"
            );
        }
        assert!(!output.contains("unsupported statement: pen_"));
        assert!(!output.contains("unsupported reporter: looks_costume"));
        for parameter in ["color", "saturation", "brightness", "transparency"] {
            let variant = match parameter {
                "color" => "Color",
                "saturation" => "Saturation",
                "brightness" => "Brightness",
                "transparency" => "Transparency",
                _ => unreachable!(),
            };
            assert!(output.contains(&format!(
                "state.set_pen_color_parameter(crate::runtime::PenColorParameter::{variant},"
            )));
        }
    }

    // lower the fixture's motion blocks to the shared runtime state methods.
    #[test]
    fn rotation_direction_fixture_generates_motion_and_rotation_style_paths() {
        let input = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/src/motion-rotation-direction.sb3"
        );
        let json = crate::extractor::read_project_json(input).unwrap();
        let project = parse_project(&json).unwrap();
        let output = generate(&project);

        for generated in [
            "state.move_steps(scratch_to_number(",
            "state.set_rotation_style(\"left-right\");",
            "state.set_rotation_style(\"don't rotate\");",
            "state.set_rotation_style(\"all around\");",
            "state.bounce_on_edge();",
            "state.x_position()",
            "state.y_position()",
        ] {
            assert!(
                output.contains(generated),
                "missing generated motion path: {generated}"
            );
        }
        assert!(!output.contains("unsupported statement: motion_"));
        assert!(!output.contains("unsupported reporter: motion_"));
    }

    // specialize rgb assignment only when that target cannot use hsv controls.
    #[test]
    fn pen_rgb_generation_skips_unused_hsv_state_per_target() {
        let make_project = |include_hsv_control: bool| {
            let mut blocks = json!({
                "hat": { "opcode": "event_whenflagclicked", "next": "set_color", "topLevel": true },
                "set_color": { "opcode": "pen_setPenColorToColor", "next": null, "parent": "hat", "inputs": { "COLOR": [1, [10, "16711680"]] } }
            });
            if include_hsv_control {
                blocks["set_color"]["next"] = json!("set_saturation");
                blocks["set_saturation"] = json!({
                    "opcode": "pen_setPenColorParamTo",
                    "next": null,
                    "parent": "set_color",
                    "fields": { "COLOR_PARAM": ["saturation", null] },
                    "inputs": { "VALUE": [1, [4, "50"]] }
                });
            }
            json!({
                "targets": [
                    { "name": "Stage", "isStage": true, "variables": {}, "lists": {}, "blocks": {} },
                    { "name": "Sprite1", "isStage": false, "variables": {}, "lists": {}, "blocks": blocks }
                ]
            })
        };

        let direct_project = parse_project(&make_project(false).to_string()).unwrap();
        let direct_output = generate(&direct_project);
        assert!(direct_output.contains("state.set_pen_rgb_color_without_hsv(scratch_pen_color);"));

        let hsv_project = parse_project(&make_project(true).to_string()).unwrap();
        let hsv_output = generate(&hsv_project);
        assert!(hsv_output.contains("state.set_pen_rgb_color(scratch_pen_color);"));
        assert!(hsv_output.contains("state.set_pen_color_parameter("));
    }

    #[test]
    fn pen_smoke_loop_uses_enum_menu_values_in_its_hot_path() {
        let input = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/src/pen-smoke-test.sb3");
        let json = crate::extractor::read_project_json(input).unwrap();
        let project = parse_project(&json).unwrap();
        let output = generate(&project);

        assert!(
            output.contains(
                "state.set_pen_color_parameter(crate::runtime::PenColorParameter::Color,"
            )
        );
        assert!(!output.contains("scratch_pen_parameter"));
    }

    #[test]
    fn reporter_connected_pen_color_parameter_keeps_dynamic_conversion() {
        let input = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/src/pen-tests.sb3");
        let json = crate::extractor::read_project_json(input).unwrap();
        let mut project = parse_project(&json).unwrap();
        let (variable_id, variable_name) = project
            .targets
            .iter()
            .find(|target| target.is_stage)
            .and_then(|stage| stage.variables.iter().next())
            .map(|(id, variable)| (id.clone(), variable.name.clone()))
            .unwrap();
        let target = project
            .targets
            .iter_mut()
            .find(|target| target.name == "Sprite1")
            .unwrap();
        target.blocks.insert(
            "dynamic_pen_parameter".to_owned(),
            ScratchBlock {
                opcode: "data_variable".to_owned(),
                fields: std::collections::HashMap::from([(
                    "VARIABLE".to_owned(),
                    json!([variable_name, variable_id]),
                )]),
                ..Default::default()
            },
        );
        let pen_block = target
            .blocks
            .values_mut()
            .find(|block| block.opcode == "pen_changePenColorParamBy")
            .unwrap();
        pen_block.inputs.insert(
            "COLOR_PARAM".to_owned(),
            json!([1, "dynamic_pen_parameter"]),
        );

        let output = generate(&project);
        assert!(output.contains("state.set_pen_color_parameter(scratch_string(state.get_"));
    }

    #[test]
    fn sensing_fixture_lowers_reporters_as_values_and_emits_live_monitor_snapshots() {
        let input = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/src/monitor-tests-sensing.sb3"
        );
        let json = crate::extractor::read_project_json(input).unwrap();
        let project = parse_project(&json).unwrap();
        let output = generate(&project);

        assert!(output.contains("state.set_variable_visible(0, true);"));
        assert!(output.contains("state.set_variable_visible(0, false);"));
        assert!(output.contains("state.days_since_2000()"));
        assert!(output.contains("state.username()"));
        assert!(output.contains("state.current_year()"));
        assert!(output.contains("state.current_day_of_week()"));
        assert!(output.contains("state.mouse_x()"));
        assert!(output.contains("state.mouse_y()"));
        assert!(output.contains("state.mouse_down()"));
        assert!(output.contains("state.loudness()"));
        assert!(output.contains("state.project_timer_seconds(clock.now_millis())"));
        for method in [
            "current_year",
            "current_month",
            "current_date",
            "current_day_of_week",
            "current_hour",
            "current_minute",
            "current_second",
        ] {
            assert!(
                output.contains(&format!("state.{method}()")),
                "missing {method}"
            );
            assert!(
                output.contains(&format!("scratch_string(state.{method}())")),
                "{method} is not used through the shared reporter path in a block input"
            );
        }
        for reporter in [
            "state.days_since_2000()",
            "state.mouse_x()",
            "state.mouse_y()",
            "state.project_timer_seconds(clock.now_millis())",
            "state.username()",
            "state.mouse_down()",
        ] {
            assert!(
                output.contains(&format!("scratch_string({reporter})")),
                "{reporter} is not used through the shared reporter path"
            );
        }
        assert!(output.contains("scratch_subtract(state.days_since_2000(), state.get_shared(0))"));
        for monitor_key in [
            "sensing_dayssince2000/sensing_dayssince2000",
            "sensing_current/current_year",
            "sensing_current/current_month",
            "sensing_current/current_date",
            "sensing_current/current_dayofweek",
            "sensing_current/current_hour",
            "sensing_current/current_minute",
            "sensing_current/current_second",
            "sensing_online/online",
            "sensing_username/sensing_username",
            "sensing_mousex/sensing_mousex",
            "sensing_mousey/sensing_mousey",
            "sensing_mousedown/sensing_mousedown",
        ] {
            assert!(
                output.contains(&format!("__reporter/Stage/{monitor_key}")),
                "missing monitor snapshot for {monitor_key}"
            );
        }
        assert!(output.contains("sensing_online"));
        assert!(output.contains("value: ScratchValue = (false).into()"));
        // assert the generated reporter key and local binding separately to avoid formatting coupling.
        assert!(output.contains(
            "__reporter/Stage/sensing_current/current_year\".to_owned()"
        ));
        assert!(output.contains(
            "value: ScratchValue = (self.state.current_year()).into()"
        ));
        assert!(output.contains(
            "__reporter/Stage/sensing_username/sensing_username\".to_owned()"
        ));
        assert!(output.contains(
            "value: ScratchValue = (self.state.username()).into()"
        ));
        assert!(output.contains("ReporterSnapshot"));
        assert!(output.contains("pub fn has_live_sensing_monitors(&self) -> bool { true }"));
        assert!(!output.contains("unsupported reporter: sensing_dayssince2000"));
        assert!(!output.contains("unsupported reporter: sensing_username"));
        assert!(!output.contains("unsupported reporter: sensing_online"));
        assert!(!output.contains("unsupported statement: data_showvariable"));
        assert!(!output.contains("unsupported statement: data_hidevariable"));
    }

    #[test]
    fn clone_fixture_lowers_clone_creation_and_clone_start_hats() {
        let input = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/src/clone-with-variables.sb3"
        );
        let json = crate::extractor::read_project_json(input).unwrap();
        let project = parse_project(&json).unwrap();
        let output = generate(&project);

        assert!(output.contains("state.create_clone(source, target, 300);"));
        assert!(output.contains("fn new_clone_scripts(target: usize, _instance: usize)"));
        assert!(output.contains("new_clone_start_script_sprite1_2(_instance)"));
        assert!(output.contains("new_clone_start_script_sprite1_5(_instance)"));
        assert!(output.contains("take_pending_clones"));
        assert!(output.contains("remove_target_scripts"));
        assert!(output.contains("ScratchVariableMetadata::for_target(\"`jEk@4|i[#Fk?(8x)AV.-my variable\", \"my variable\", \"Stage\", 0, true)"));
    }

    // exercise selector strings through list mutation and reporter code generation.
    #[test]
    fn list_index_dropdowns_flow_into_list_mutations_and_item_reporters() {
        let project = parse_project(
            r#"{
                "targets": [{
                    "name": "Stage",
                    "isStage": true,
                    "lists": { "list-id": ["items", ["a", "b"]] },
                    "blocks": {
                        "hat": { "opcode": "event_whenflagclicked", "next": "delete", "topLevel": true },
                        "delete": { "opcode": "data_deleteoflist", "next": "insert", "parent": "hat", "fields": { "LIST": ["items", "list-id"] }, "inputs": { "INDEX": [3, "all", [10, ""]] } },
                        "all": { "opcode": "data_listindexall", "shadow": true, "fields": { "INDEX": ["all", null] } },
                        "insert": { "opcode": "data_insertatlist", "next": "replace", "parent": "delete", "fields": { "LIST": ["items", "list-id"] }, "inputs": { "ITEM": [1, [10, "x"]], "INDEX": [3, "random", [10, ""]] } },
                        "random": { "opcode": "data_listindexrandom", "shadow": true, "fields": { "INDEX": ["random", null] } },
                        "replace": { "opcode": "data_replaceitemoflist", "next": "say", "parent": "insert", "fields": { "LIST": ["items", "list-id"] }, "inputs": { "INDEX": [3, "last", [10, ""]], "ITEM": [1, [10, "y"]] } },
                        "last": { "opcode": "data_listindexall", "shadow": true, "fields": { "INDEX": ["last", null] } },
                        "say": { "opcode": "looks_say", "parent": "replace", "inputs": { "MESSAGE": [3, "item", [10, ""]] } },
                        "item": { "opcode": "data_itemoflist", "parent": "say", "fields": { "LIST": ["items", "list-id"] }, "inputs": { "INDEX": [3, "get_index", [10, ""]] } },
                        "get_index": { "opcode": "data_listindexrandom", "shadow": true, "fields": { "INDEX": ["last", null] } }
                    }
                }]
            }"#,
        )
        .unwrap();
        let output = generate(&project);

        assert!(output.contains("let scratch_list_index: ScratchValue = (\"all\").into();"));
        assert!(output.contains("let scratch_list_index: ScratchValue = (\"random\").into();"));
        assert!(output.contains("let scratch_list_index: ScratchValue = (\"last\").into();"));
        assert!(output.contains("state.list(0).get(\"last\")"));
        assert!(
            output.contains(
                "let scratch_list_changed = state.list_mut(0).delete(scratch_list_index);"
            )
        );
        assert!(output.contains("state.list_changed_result(0, scratch_list_changed);"));
        assert!(!output.contains("list_before_change"));
        assert!(!output.contains("unsupported reporter: data_listindex"));
    }

    #[test]
    fn item_replace_fixture_uses_guarded_numeric_list_paths() {
        let input = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/src/item-replace-smoke.sb3"
        );
        let json = crate::extractor::read_project_json(input).unwrap();
        let project = parse_project(&json).unwrap();
        let list = project
            .targets
            .iter()
            .flat_map(|target| target.lists.values())
            .find(|list| list.items.len() == 172_800)
            .expect("replacement fixture should retain all 172,800 initial list items");
        // confirm project parsing retains the beginning, middle, and final items.
        assert!(list.items[0].is_number());
        assert!(list.items[86_400].is_number());
        assert!(list.items[172_799].is_number());
        let output = generate(&project);

        // numeric inputs use one validated replacement and read-back operation.
        assert!(output.contains("replace_number_and_equals"));
        assert!(output.contains("with_stage_numeric_list_batch"));
        assert!(output.contains("scratch_batch_value_"));
        assert!(output.contains("scratch_list_replace_matches"));
        assert!(output.contains("state.change_shared_number(1, scratch_to_number(1.0_f64));"));
        assert!(output.contains("repeat_remaining -= 1;"));
        assert!(output.contains("repeat_counts[0] = repeat_remaining;"));
        assert!(output.contains("if script.yield_at_safe_point(clock)"));
        assert!(output.contains("return ScriptStatus::StopAll;"));
        assert!(output.contains("with_scratch_random(|loop_random|"));
        assert!(output.contains("loop_random.between_with_integer_bounds"));
    }

    #[test]
    fn replacement_batch_rejects_branch_that_reads_the_list() {
        let input = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/src/item-replace-smoke.sb3"
        );
        let json = crate::extractor::read_project_json(input).unwrap();
        let mut project = parse_project(&json).unwrap();
        let sprite = project
            .targets
            .iter_mut()
            .find(|target| target.blocks.contains_key("i"))
            .unwrap();
        sprite
            .blocks
            .get_mut("i")
            .unwrap()
            .inputs
            .insert("MESSAGE".into(), serde_json::json!([2, "u"]));
        let output = generate(&project);
        assert!(!output.contains("with_stage_numeric_list_batch"));
        assert!(output.contains("scratch_list_replace_result.unwrap_or_else"));
        assert!(output.contains("state.list(0).get("));
    }

    // verify simple warp list fills use a bounded generated batch path and retain resumable state.
    #[test]
    fn list_smoke_warp_append_loop_is_fused_with_watchdog_chunks() {
        let input = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/src/list-smoke-test.sb3");
        let json = crate::extractor::read_project_json(input).unwrap();
        let project = parse_project(&json).unwrap();
        let output = generate(&project);

        assert!(output.contains("add_generated_numbers(repeat_chunk"));
        assert!(output.contains("with_scratch_random(|scratch_random|"));
        assert!(output.contains("between_with_integer_bounds(11.0_f64, 20.0_f64, true)"));
        assert!(output.contains("scratch_random_constant(11.0_f64, 20.0_f64, true)"));
        assert!(output.contains("script.warp_budget.expired_now(clock)"));
        assert!(output.contains("repeat_counts[0] = repeat_remaining"));
        assert!(!output.contains("list_before_change"));
    }

    #[test]
    fn broadcast_fixture_resolves_message_ids_and_emits_restartable_receiver_hats() {
        let input = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/src/broadcast-test.sb3");
        let json = crate::extractor::read_project_json(input).unwrap();
        let project = parse_project(&json).unwrap();
        let output = generate(&project);

        assert!(
            output.contains(
                "message: broadcast_message_key(scratch_string(\"message1\")), wait: false"
            )
        );
        assert!(
            output.contains(
                "message: broadcast_message_key(scratch_string(\"message1\")), wait: true"
            )
        );
        assert!(
            output.contains(
                "message: broadcast_message_key(scratch_string(\"message2\")), wait: false"
            )
        );
        assert!(output.contains("1 => { for instance in state.instances_for_target(2)"));
        assert!(output.contains("2 => { for instance in state.instances_for_target(1)"));
        assert!(output.contains("fn broadcast_receiver(&self) -> Option<(usize, usize, usize)>"));
        assert!(!output.contains(
            "self.scheduler.append_new_script_ids_since(&mut self.frame_order, next_new_id)"
        ));
        assert!(!output.contains("unsupported statement: event_broadcast"));
        assert!(!output.contains("unsupported statement: event_broadcastandwait"));
    }

    #[test]
    fn backdrop_switch_fixture_routes_events_to_stage_backdrop_hats() {
        let input = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/src/think.sb3");
        let json = crate::extractor::read_project_json(input).unwrap();
        let mut project = parse_project(&json).unwrap();
        let stage = project
            .targets
            .iter_mut()
            .find(|target| target.is_stage)
            .unwrap();
        stage.blocks.insert(
            "backdrop_hat".to_owned(),
            ScratchBlock {
                opcode: "event_whenbackdropswitchesto".to_owned(),
                top_level: true,
                fields: [("BACKDROP".to_owned(), json!(["backdrop1", null]))].into(),
                ..ScratchBlock::default()
            },
        );

        let output = generate(&project);

        assert!(output.contains("fn new_backdrop_scripts(backdrop: &str, state: &ScratchState)"));
        assert!(output.contains("\"backdrop1\" => { for instance in state.instances_for_target("));
        assert!(output.contains("backdrop_receiver_script_stage_"));
        // redraw stops another scheduler round but does not interrupt the current script turn.
        assert!(!output.contains("if state.redraw_requested() { return ScriptStatus::Yield; }"));
    }

    #[test]
    fn legacy_vm_noops_keep_their_command_and_undefined_reporter_behavior() {
        let project = parse_project(
            &json!({
                "targets": [{
                    "name": "Sprite1", "isStage": false,
                    "blocks": {
                        "hat": {"opcode":"event_whenflagclicked", "next":"hide_all", "topLevel":true},
                        "hide_all": {"opcode":"looks_hideallsprites", "next":"change_stretch", "parent":"hat"},
                        "change_stretch": {"opcode":"looks_changestretchby", "next":"set_stretch", "parent":"hide_all"},
                        "set_stretch": {"opcode":"looks_setstretchto", "next":"scroll_right", "parent":"change_stretch"},
                        "scroll_right": {"opcode":"motion_scroll_right", "next":"scroll_up", "parent":"set_stretch"},
                        "scroll_up": {"opcode":"motion_scroll_up", "next":"align_scene", "parent":"scroll_right"},
                        "align_scene": {"opcode":"motion_align_scene", "next":"say_userid", "parent":"scroll_up"},
                        "say_userid": {"opcode":"looks_say", "next":"say_xscroll", "parent":"align_scene", "inputs":{"MESSAGE":[2,"reporter_userid"]}},
                        "reporter_userid": {"opcode":"sensing_userid", "parent":"say_userid"},
                        "say_xscroll": {"opcode":"looks_say", "next":"say_yscroll", "parent":"say_userid", "inputs":{"MESSAGE":[2,"reporter_xscroll"]}},
                        "reporter_xscroll": {"opcode":"motion_xscroll", "parent":"say_xscroll"},
                        "say_yscroll": {"opcode":"looks_say", "parent":"say_xscroll", "inputs":{"MESSAGE":[2,"reporter_yscroll"]}},
                        "reporter_yscroll": {"opcode":"motion_yscroll", "parent":"say_yscroll"}
                    }
                }]
            })
            .to_string(),
        )
        .unwrap();
        let output = generate(&project);

        assert_eq!(output.matches("let _ = ();\n").count(), 6);
        assert_eq!(output.matches("ScratchValue::Undefined").count(), 3);
        for opcode in [
            "looks_hideallsprites",
            "looks_changestretchby",
            "looks_setstretchto",
            "motion_scroll_right",
            "motion_scroll_up",
            "motion_align_scene",
            "sensing_userid",
            "motion_xscroll",
            "motion_yscroll",
        ] {
            assert!(!output.contains(&format!("unsupported statement: {opcode}")));
            assert!(!output.contains(&format!("unsupported reporter: {opcode}")));
        }
    }

    #[test]
    fn stop_loop_fixture_lowers_this_script_inside_a_custom_block_and_stop_all() {
        let input = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/src/stop-loop-test.sb3");
        let json = crate::extractor::read_project_json(input).unwrap();
        let project = parse_project(&json).unwrap();
        let output = generate(&project);

        assert!(output.contains("if script.frames.len() > 1 { script.frames.pop(); continue; }"));
        assert!(output.contains("return ScriptStatus::StopAll;"));
        assert!(!output.contains("unsupported statement: control_stop"));
    }

    #[test]
    fn clone_stop_fixture_lowers_motion_for_target_instances() {
        let input = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/src/clone-stop-test.sb3");
        let json = crate::extractor::read_project_json(input).unwrap();
        let project = parse_project(&json).unwrap();
        let output = generate(&project);

        assert!(
            output.contains(
                "state.set_xy(scratch_to_number(-240.0_f64), scratch_to_number(0.0_f64));"
            )
        );
        assert!(output.contains("state.change_x_by(scratch_to_number(1.0_f64));"));
        assert!(!output.contains("state.request_redraw();"));
        assert!(
            output.contains("self.scheduler.script_ids_in_frame_order_into(&mut self.frame_order)")
        );
        assert!(output.contains("self.scheduler.advance_script_id(id, &mut self.state)"));
        assert!(!output.contains(
            "self.scheduler.append_new_script_ids_since(&mut self.frame_order, next_new_id)"
        ));
        assert!(!output.contains("unsupported statement: motion_gotoxy"));
        assert!(!output.contains("unsupported statement: motion_changexby"));
    }

    #[test]
    fn go_to_fixture_lowers_random_moves_and_resumable_glides() {
        let input = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/src/go-to-tests.sb3");
        let json = crate::extractor::read_project_json(input).unwrap();
        let project = parse_project(&json).unwrap();
        let output = generate(&project);

        assert!(output.contains("scratch_random(-240.5_f64, 240.5_f64)"));
        assert!(output.contains("scratch_random(-179.5_f64, 180.5_f64)"));
        assert!(output.contains("state.change_x_by(scratch_to_number(1.0_f64));"));
        assert!(output.contains("state.change_y_by(scratch_to_number(1.0_f64));"));
        assert!(output.contains("GlideState::new(clock.now_millis()"));
        assert!(output.contains("motion_glide.position_at(clock.now_millis())"));
        assert!(output.contains("return ScriptStatus::Yield;"));
        assert!(output.contains("state.set_xy(motion_x, motion_y);"));
        assert!(!output.contains("state.request_redraw();"));
        // statically numeric sprite updates use the allocation-free runtime entry point.
        assert!(output.contains("state.change_local_number(1, scratch_change);"));
        assert!(!output.contains(
            "self.scheduler.append_new_script_ids_since(&mut self.frame_order, next_new_id)"
        ));
        let first_glide = output.find("GlideState::new(clock.now_millis()").unwrap();
        let second_glide = output[first_glide + 1..]
            .find("GlideState::new(clock.now_millis()")
            .map(|offset| first_glide + 1 + offset)
            .unwrap();
        assert!(first_glide < second_glide);
        for opcode in [
            "motion_goto",
            "motion_changeyby",
            "motion_glideto",
            "motion_glidesecstoxy",
        ] {
            assert!(!output.contains(&format!("unsupported statement: {opcode}")));
        }
    }

    #[test]
    fn motion_point_fixture_lowers_set_x_y_and_clone_direction_blocks() {
        let input = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/src/motion-point-test.sb3"
        );
        let json = crate::extractor::read_project_json(input).unwrap();
        let project = parse_project(&json).unwrap();
        let output = generate(&project);
        assert!(output.contains("state.set_xy(scratch_to_number(0.0_f64), motion_y);"));
        assert!(output.contains("state.set_xy(motion_x, scratch_to_number(0.0_f64));"));
        assert!(output.contains("state.set_direction(scratch_to_number(90.0_f64));"));
        assert!(output.contains("scratch_random(-180.0_f64, 180.0_f64)"));
        for opcode in [
            "motion_setx",
            "motion_sety",
            "motion_pointindirection",
            "motion_pointtowards",
        ] {
            assert!(!output.contains(&format!("unsupported statement: {opcode}")));
        }
    }

    #[test]
    fn motion_position_reporters_read_the_scheduled_instance() {
        let input = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/src/pen-smoke-test-two.sb3"
        );
        let json = crate::extractor::read_project_json(input).unwrap();
        let project = parse_project(&json).unwrap();
        let output = generate(&project);

        // position reporters now use the target-aware state accessors directly.
        assert!(
            output.contains(
                "state.x_position()"
            )
        );
        assert!(
            output.contains(
                "state.y_position()"
            )
        );
    }

    #[test]
    fn position_direction_clone_fixture_lowers_turns_and_direction_reporter() {
        let input = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/src/position-direction-clone.sb3"
        );
        let json = crate::extractor::read_project_json(input).unwrap();
        let project = parse_project(&json).unwrap();
        let output = generate(&project);

        assert!(
            output
                .contains("state.set_direction(state.direction() + scratch_to_number(15.0_f64));")
        );
        assert!(
            output
                .contains("state.set_direction(state.direction() - scratch_to_number(15.0_f64));")
        );
        assert!(output.contains("scratch_string(state.direction())"));
        for opcode in ["motion_turnright", "motion_turnleft", "motion_direction"] {
            assert!(!output.contains(&format!("unsupported statement: {opcode}")));
            assert!(!output.contains(&format!("unsupported reporter: {opcode}")));
        }
    }

    #[test]
    fn say_think_fixture_generates_bubbles_and_timed_waits() {
        let input = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/src/say-think.sb3");
        let json = crate::extractor::read_project_json(input).unwrap();
        let project = parse_project(&json).unwrap();
        let output = generate(&project);

        assert!(output.contains("state.set_speech_bubble(BubbleKind::Say"));
        assert!(output.contains("state.set_speech_bubble(BubbleKind::Think"));
        assert!(output.contains("return ScriptStatus::WaitUntil(wait_deadline);"));
        assert!(output.contains("self.state.expire_speech_bubbles(self.scheduler.now_millis());"));
        for opcode in [
            "looks_say",
            "looks_sayforsecs",
            "looks_think",
            "looks_thinkforsecs",
        ] {
            assert!(!output.contains(&format!("unsupported statement: {opcode}")));
        }
    }

    #[test]
    fn consecutive_timed_think_blocks_resume_at_each_block_and_reach_following_work() {
        let input = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/src/think.sb3");
        let json = crate::extractor::read_project_json(input).unwrap();
        let project = parse_project(&json).unwrap();
        let output = generate(&project);

        assert!(
            output
                .contains("\n                1 => {\n                    let scratch_bubble_text")
        );
        assert!(
            output
                .contains("\n                2 => {\n                    let scratch_bubble_text")
        );
        assert_eq!(
            output
                .matches("state.set_speech_bubble(BubbleKind::Think")
                .count(),
            2
        );
        assert!(output.contains("state.set_shared(0, scratch_new_value);"));
    }

    #[test]
    fn generated_program_exposes_tick_redraw_and_input_state_to_compiled_code() {
        let project = parse_project(
            &json!({
                "targets": [{
                    "name": "Sprite1", "isStage": false, "variables": {},
                    "blocks": {
                        "hat": {"opcode":"event_whenflagclicked", "next":"say_key", "topLevel":true, "inputs":{}, "fields":{}},
                        "say_key": {"opcode":"looks_say", "next":"say_x", "parent":"hat", "inputs":{"MESSAGE":[3,"key",[10,""]]}, "fields":{}},
                        "key": {"opcode":"sensing_keypressed", "next":null, "parent":"say_key", "inputs":{}, "fields":{"KEY_OPTION":["space",null]}},
                        "say_x": {"opcode":"looks_say", "next":null, "parent":"say_key", "inputs":{"MESSAGE":[3,"mouse_x",[10,""]]}, "fields":{}},
                        "mouse_x": {"opcode":"sensing_mousex", "next":null, "parent":"say_x", "inputs":{}, "fields":{}}
                    }
                }]
            }).to_string(),
        ).unwrap();
        let output = generate(&project);
        assert!(output.contains("pub fn begin_tick(&mut self)"));
        assert!(output.contains("pub fn redraw_requested(&self)"));
        assert!(output.contains("pub fn presentation_consumed(&mut self)"));
        assert!(output.contains("pub fn apply_input(&mut self, event: RuntimeInputEvent)"));
        assert!(output.contains("state.key_pressed(\"space\")"));
        assert!(output.contains("state.mouse_x()"));
    }

    #[test]
    fn keypress_hats_start_generated_scripts_from_input_edges() {
        let project = parse_project(
            &json!({
                "targets": [{
                    "name": "Sprite1", "isStage": false, "variables": {},
                    "blocks": {
                        "hat": {"opcode":"event_whenkeypressed", "next":"say", "topLevel":true, "inputs":{}, "fields":{"KEY_OPTION":["space",null]}},
                        "say": {"opcode":"looks_say", "next":null, "parent":"hat", "inputs":{"MESSAGE":[1,[10,"key"]]}, "fields":{}}
                    }
                }]
            }).to_string(),
        ).unwrap();
        let output = generate(&project);
        assert!(output.contains("fn new_keypress_scripts(key: &str, state: &ScratchState)"));
        assert!(output.contains("\"space\" => { for instance in state.instances_for_target(0)"));
        assert!(output.contains("self.scheduler.start_key_hats(scripts)"));
    }

    // check that the special any selector is emitted before key-specific hats.
    #[test]
    fn any_key_hat_is_started_for_each_recognized_key_event() {
        let project = parse_project(
            &json!({
                "targets": [{
                    "name":"Stage", "isStage":true, "variables":{}, "blocks":{}
                }, {
                    "name":"Sprite1", "isStage":false, "variables":{},
                    "blocks": {
                        "any_hat": {"opcode":"event_whenkeypressed", "next":"any_say", "topLevel":true, "inputs":{}, "fields":{"KEY_OPTION":["any",null]}},
                        "any_say": {"opcode":"looks_say", "next":null, "parent":"any_hat", "inputs":{"MESSAGE":[1,[10,"any"]]}, "fields":{}},
                        "letter_hat": {"opcode":"event_whenkeypressed", "next":"letter_say", "topLevel":true, "inputs":{}, "fields":{"KEY_OPTION":["a",null]}},
                        "letter_say": {"opcode":"looks_say", "next":null, "parent":"letter_hat", "inputs":{"MESSAGE":[1,[10,"letter"]]}, "fields":{}}
                    }
                }]
            }).to_string(),
        ).unwrap();
        let output = generate(&project);
        let start = output.find("fn new_keypress_scripts").unwrap();
        let body = &output[start..];
        let match_at = body.find("match key").unwrap();
        assert!(body[..match_at].contains("state.instances_for_target(1)"));
        assert!(body[match_at..].contains("\"a\" =>"));
    }

    // verify the supplied project keeps its active costume reporter and shadow selector distinct.
    #[test]
    fn detect_keys_fixture_uses_connected_reporter_and_shadow_fallback() {
        let input = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/src/detect-keys.sb3");
        let json = crate::extractor::read_project_json(input).unwrap();
        let project = parse_project(&json).unwrap();
        let sprite = project
            .targets
            .iter()
            .find(|target| target.name == "Sprite1")
            .unwrap();
        assert_eq!(sprite.costumes[0].name, "w");

        let output = generate(&project);
        assert!(output.contains("state.key_pressed(state.costume_number_name(false))"));
        assert!(output.contains("state.key_pressed(\"up arrow\")"));
        assert!(!output.contains("state.key_pressed(\"w\")"));
        assert!(output.contains("\"up arrow\" => { for instance in state.instances_for_target(1)"));
        assert!(output.contains("\"w\" => { for instance in state.instances_for_target(1)"));
        assert!(output.contains("fn key_hat_key(&self) -> Option<(usize, usize)>"));
        assert!(output.contains("self.scheduler.start_key_hats(scripts)"));
        let key_hat_impl = output
            .split("fn key_hat_key(&self) -> Option<(usize, usize)>")
            .nth(1)
            .unwrap()
            .split("fn restart_for_broadcast")
            .next()
            .unwrap();
        assert!(!key_hat_impl.contains("_ => None"));
    }

    // ensure an input-bearing reporter monitor evaluates in the monitor target's state context.
    #[test]
    fn input_reporter_monitor_uses_its_block_input_and_target_context() {
        let project = parse_project(
            &json!({
                "targets": [{
                    "name": "Stage", "isStage": true, "variables": {}, "blocks": {}
                }, {
                    "name": "Sprite1", "isStage": false, "variables": {},
                    "blocks": {
                        "key": {"opcode":"sensing_keypressed", "inputs":{"KEY_OPTION":[3,"costume",[10,"w"]]}, "fields":{}, "parent":null, "next":null},
                        "costume": {"opcode":"looks_costumenumbername", "inputs":{}, "fields":{"NUMBER_NAME":["name",null]}, "parent":"key", "next":null}
                    }
                }],
                "monitors": [{
                    "id":"key", "opcode":"sensing_keypressed", "params":{"KEY_OPTION":"space"},
                    "spriteName":"Sprite1", "visible":true
                }]
            }).to_string(),
        ).unwrap();
        let output = generate(&project);
        assert!(output.contains("state.key_pressed(state.costume_number_name(false))"));
        assert!(output.contains("self.state.original_instance(1)"));
        assert!(output.contains("self.state.set_current_instance(instance)"));
    }

    #[test]
    fn stop_other_scripts_in_sprite_keeps_the_current_script_continuation() {
        let project = parse_project(
            &json!({
                "targets": [{
                    "name": "Sprite1",
                    "isStage": false,
                    "variables": {},
                    "blocks": {
                        "hat": {"opcode": "event_whenflagclicked", "next": "stop", "topLevel": true, "inputs": {}, "fields": {}},
                        "stop": {"opcode": "control_stop", "next": "say", "parent": "hat", "inputs": {}, "fields": {"STOP_OPTION": ["other scripts in sprite", null]}},
                        "say": {"opcode": "looks_say", "next": null, "parent": "stop", "inputs": {"MESSAGE": [1, [10, "after stop"]]}, "fields": {}}
                    }
                }]
            })
            .to_string(),
        )
        .unwrap();
        let output = generate(&project);

        assert!(output.contains("return ScriptStatus::StopOtherScripts;"));
        assert!(output.contains("pc = 1;"));
        assert!(!output.contains("unsupported statement: control_stop"));
    }

    #[test]
    fn generated_helpers_and_imports_follow_project_usage() {
        let project_json = json!({
            "targets": [{
                "name": "Stage",
                "isStage": true,
                "variables": {},
                "lists": {},
                "blocks": {}
            }]
        });
        let project = parse_project(&project_json.to_string()).unwrap();
        let output = generate(&project);

        assert!(output.contains("use crate::runtime::{"));
        assert!(!output.contains("#[allow(unused_imports)]"));
        assert!(!output.contains("print_value"));
        assert!(!output.contains("WarpBudget"));
        assert!(!output.contains("warp_budget_expired"));
        assert!(!output.contains("yield_at_safe_point"));
        assert!(output.contains("fn new_clone_scripts(target: usize, _instance: usize)"));
        assert!(!output.contains("take_pending_clones"));
        assert!(!output.contains("remove_target_scripts"));
        assert!(!output.contains("state.set_current_instance(*instance)"));
        assert!(output.contains("#[allow(dead_code)]\npub fn run()"));
    }

    #[test]
    fn generated_warp_guards_only_include_used_helpers() {
        let input = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/src/clone-with-variables.sb3"
        );
        let json = crate::extractor::read_project_json(input).unwrap();
        let project = parse_project(&json).unwrap();
        let output = generate(&project);

        assert!(output.contains("fn warp_budget_expired(&mut self"));
        assert!(!output.contains("fn yield_at_safe_point(&mut self"));
        assert!(output.contains("WarpBudget"));
    }

    #[test]
    fn if_else_fixture_lowers_conditional_branches() {
        let input = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/src/if-else-test.sb3");
        let json = crate::extractor::read_project_json(input).unwrap();
        let project = parse_project(&json).unwrap();
        let output = generate(&project);

        assert!(output.contains("if scratch_and("));
        assert!(output.contains("} else {"));
        assert!(!output.contains("unsupported statement: control_if"));
        assert!(!output.contains("unsupported statement: control_if_else"));
    }

    #[test]
    fn lowers_dynamic_math_with_scratch_conversion_and_keeps_literal_types() {
        let project = parse_project(
            &json!({
                "targets": [{
                    "name": "Sprite1",
                    "isStage": false,
                    "variables": {
                        "amount_id": ["amount", 5.5],
                        "same_id": ["same", false]
                    },
                    "blocks": {
                        "hat": {
                            "opcode": "event_whenflagclicked",
                            "next": "math_set",
                            "topLevel": true,
                            "inputs": {},
                            "fields": {}
                        },
                        "math_set": {
                            "opcode": "data_setvariableto",
                            "next": "compare_set",
                            "parent": "hat",
                            "inputs": {"VALUE": [1, "math"]},
                            "fields": {"VARIABLE": ["amount", "amount_id"]}
                        },
                        "math": {
                            "opcode": "operator_mathop",
                            "parent": "math_set",
                            "inputs": {"NUM": [3, "variable"]},
                            "fields": {"OPERATOR": ["floor", null]}
                        },
                        "variable": {
                            "opcode": "data_variable",
                            "parent": "math",
                            "inputs": {},
                            "fields": {"VARIABLE": ["amount", "amount_id"]}
                        },
                        "compare_set": {
                            "opcode": "data_setvariableto",
                            "parent": "math_set",
                            "inputs": {"VALUE": [1, "compare"]},
                            "fields": {"VARIABLE": ["same", "same_id"]}
                        },
                        "compare": {
                            "opcode": "operator_equals",
                            "parent": "compare_set",
                            "inputs": {
                                "OPERAND1": [1, [10, "5"]],
                                "OPERAND2": [1, [4, "5"]]
                            },
                            "fields": {}
                        }
                    }
                }]
            })
            .to_string(),
        )
        .unwrap();

        let output = generate(&project);
        assert!(output.contains("scratch_to_number(state.get(0)).floor()"));
        assert!(output.contains("scratch_equals(\"5\", 5.0_f64)"));
    }

    #[test]
    fn straight_line_blocks_share_one_resumable_dispatch_arm() {
        let project = parse_project(
            &json!({
                "targets": [{
                    "name": "Sprite1",
                    "isStage": false,
                    "variables": {"first": ["first", 0], "second": ["second", 0]},
                    "blocks": {
                        "hat": {"opcode": "event_whenflagclicked", "next": "set_first", "topLevel": true, "inputs": {}, "fields": {}},
                        "set_first": {"opcode": "data_setvariableto", "next": "set_second", "inputs": {"VALUE": [1, [4, "1"]]}, "fields": {"VARIABLE": ["first", "first"]}},
                        "set_second": {"opcode": "data_setvariableto", "next": null, "inputs": {"VALUE": [1, [4, "2"]]}, "fields": {"VARIABLE": ["second", "second"]}}
                    }
                }]
            })
            .to_string(),
        )
        .unwrap();

        let output = generate(&project);
        // straight-line numeric assignments avoid temporary scratch values.
        assert!(output.contains("state.set_local_number(0, 1.0_f64);"));
        assert!(output.contains("state.set_local_number(1, 2.0_f64);"));
        assert!(!output.contains("script.frames.last_mut().unwrap().pc = 1;"));
        assert!(output.contains("script.frames.last_mut().unwrap().pc = usize::MAX;"));
    }

    #[test]
    fn numeric_loops_use_native_locals_and_commit_before_safeguard_yields() {
        // repeat-until avoids the separate simple-repeat fusion path in this test.
        let project = parse_project(
            &json!({
                "targets": [{
                    "name": "Sprite1",
                    "isStage": false,
                    "variables": {"count_id": ["count", 0]},
                    "blocks": {
                        "hat": {"opcode": "event_whenflagclicked", "next": "repeat", "topLevel": true, "inputs": {}, "fields": {}},
                        "repeat": {"opcode": "control_repeat_until", "next": null, "inputs": {"CONDITION": [2, "equal"], "SUBSTACK": [2, "change"]}, "fields": {}},
                        "change": {"opcode": "data_changevariableby", "next": null, "inputs": {"VALUE": [1, [4, "1"]]}, "fields": {"VARIABLE": ["count", "count_id"]}},
                        "equal": {"opcode": "operator_equals", "next": null, "inputs": {"OPERAND1": [3, [12, "count", "count_id"], [4, "10"]], "OPERAND2": [1, [4, "10"]]}, "fields": {}}
                    }
                }]
            })
            .to_string(),
        )
        .unwrap();

        let output = generate(&project);
        assert!(output.contains("let mut scratch_local_0 = scratch_to_number(state.get(0));"));
        assert!(output.contains("scratch_local_0 += 1.0_f64;"));
        assert!(!output.contains("scratch_equals(\"same\", \"same\")"));
        let safeguard = output
            .find("if !scratch_warp_mode || script.warp_budget.expired(clock)")
            .unwrap();
        let commit = output[safeguard..]
            .find("state.set(0, scratch_local_0.into());")
            .unwrap()
            + safeguard;
        let yielded = output[commit..]
            .find("return ScriptStatus::Yield;")
            .unwrap()
            + commit;
        assert!(safeguard < commit && commit < yielded);
    }

    #[test]
    fn fast_math_only_changes_proven_numeric_loop_expressions() {
        let input = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/src/hard-math-10-seconds.sb3"
        );
        let json = crate::extractor::read_project_json(input).unwrap();
        let project = parse_project(&json).unwrap();
        let compatible = generate(&project);
        let fast = super::generate_with_options(&project, true);

        assert!(compatible.contains("scratch_number_operand(scratch_local_0)"));
        assert!(compatible.contains("scratch_sin("));
        assert!(!compatible.contains("fast_sin("));
        assert!(fast.contains("fast_sin("));
        assert!(fast.contains("fast_cos("));
        assert!(fast.contains("fast_asin_degrees("));
        assert!(!fast.contains("scratch_number_operand(scratch_local_0)"));

        let dynamic_input = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/src/operators.sb3");
        let json = crate::extractor::read_project_json(dynamic_input).unwrap();
        let dynamic_project = parse_project(&json).unwrap();
        let dynamic_fast = super::generate_with_options(&dynamic_project, true);
        assert!(dynamic_fast.contains("scratch_add(state.get("));
        assert!(dynamic_fast.contains("scratch_to_number(state.get_shared("));
    }

    #[test]
    fn stage_changes_use_the_shared_numeric_update_path() {
        let input = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/src/addition-loop.sb3");
        let json = crate::extractor::read_project_json(input).unwrap();
        let project = parse_project(&json).unwrap();
        let output = generate(&project);

        // numeric shared-variable updates preserve the runtime monitor path.
        assert!(output.contains("let scratch_change = scratch_to_number(1.0_f64);"));
        assert!(output.contains("state.change_shared_number(0, scratch_change);"));
    }

    #[test]
    fn numeric_loops_specialize_non_nan_comparisons_and_literal_increments() {
        let project = parse_project(
            &json!({
                "targets": [{
                    "name": "Sprite1",
                    "isStage": false,
                    "variables": {
                        "count_id": ["count", 0],
                        "running_id": ["running", 1]
                    },
                    "blocks": {
                        "hat": {"opcode": "event_whenflagclicked", "next": "loop", "topLevel": true, "inputs": {}, "fields": {}},
                        "loop": {"opcode": "control_repeat_until", "next": null, "inputs": {"CONDITION": [2, "equal"], "SUBSTACK": [2, "change"]}, "fields": {}},
                        "equal": {"opcode": "operator_equals", "next": null, "inputs": {"OPERAND1": [3, [12, "running", "running_id"], [4, "0"]], "OPERAND2": [1, [4, "0"]]}, "fields": {}},
                        "change": {"opcode": "data_changevariableby", "next": null, "inputs": {"VALUE": [1, [4, "1"]]}, "fields": {"VARIABLE": ["count", "count_id"]}}
                    }
                }]
            })
            .to_string(),
        )
        .unwrap();

        let output = generate(&project);
        assert!(output.contains("if scratch_local_1 == 0.0_f64 { break; }"));
        assert!(output.contains("scratch_local_0 += 1.0_f64;"));
        assert!(!output.contains("scratch_equals(scratch_local_1, 0.0_f64)"));
        assert!(!output.contains("scratch_add(scratch_local_0, 1.0_f64)"));
    }

    #[test]
    fn numeric_loops_keep_dynamic_scratch_semantics() {
        let project = parse_project(
            &json!({
                "targets": [{
                    "name": "Sprite1",
                    "isStage": false,
                    "variables": {
                        "count_id": ["count", 0],
                        "change_id": ["change", 1]
                    },
                    "blocks": {
                        "hat": {"opcode": "event_whenflagclicked", "next": "set", "topLevel": true, "inputs": {}, "fields": {}},
                        "set": {"opcode": "data_setvariableto", "next": "loop", "inputs": {"VALUE": [3, "divide", [4, "0"]]}, "fields": {"VARIABLE": ["count", "count_id"]}},
                        "divide": {"opcode": "operator_divide", "next": null, "inputs": {"NUM1": [1, [4, "0"]], "NUM2": [1, [4, "0"]]}, "fields": {}},
                        "loop": {"opcode": "control_repeat_until", "next": null, "inputs": {"CONDITION": [2, "equal"], "SUBSTACK": [2, "change"]}, "fields": {}},
                        "equal": {"opcode": "operator_equals", "next": null, "inputs": {"OPERAND1": [3, [12, "count", "count_id"], [4, "0"]], "OPERAND2": [1, [4, "0"]]}, "fields": {}},
                        "change": {"opcode": "data_changevariableby", "next": null, "inputs": {"VALUE": [3, [12, "change", "change_id"], [4, "0"]]}, "fields": {"VARIABLE": ["count", "count_id"]}}
                    }
                }]
            })
            .to_string(),
        )
        .unwrap();

        let output = generate(&project);
        assert!(output.contains("scratch_equals(scratch_local_1, 0.0_f64)"));
        assert!(output.contains("scratch_add(scratch_local_1, scratch_local_0)"));
    }

    #[test]
    fn loop_lowering_borrows_variables_and_converts_only_dynamic_conditions() {
        let project = parse_project(
            &json!({
                "targets": [{
                    "name": "Sprite1",
                    "isStage": false,
                    "variables": {
                        "count_id": ["count", 3],
                        "text_id": ["text", "3"],
                        "flag_id": ["flag", false]
                    },
                    "blocks": {
                        "hat": {
                            "opcode": "event_whenflagclicked",
                            "next": "repeat",
                            "topLevel": true,
                            "inputs": {},
                            "fields": {}
                        },
                        "repeat": {
                            "opcode": "control_repeat",
                            "next": "while",
                            "parent": "hat",
                            "inputs": {"TIMES": [3, "text_reporter", [10, "3"]], "SUBSTACK": [2, "set_text"]},
                            "fields": {}
                        },
                        "while": {
                            "opcode": "control_while",
                            "next": null,
                            "parent": "repeat",
                            "inputs": {"CONDITION": [2, "compare"], "SUBSTACK": [2, "set_flag"]},
                            "fields": {}
                        },
                        "text_reporter": {
                            "opcode": "data_variable",
                            "parent": "repeat",
                            "inputs": {},
                            "fields": {"VARIABLE": ["text", "text_id"]}
                        },
                        "set_text": {
                            "opcode": "data_setvariableto",
                            "next": "set_flag",
                            "parent": "repeat",
                            "inputs": {"VALUE": [1, [10, "5"]]},
                            "fields": {"VARIABLE": ["count", "count_id"]}
                        },
                        "set_flag": {
                            "opcode": "data_setvariableto",
                            "next": null,
                            "parent": "set_text",
                            "inputs": {"VALUE": [3, "count_reporter", [10, ""]]},
                            "fields": {"VARIABLE": ["flag", "flag_id"]}
                        },
                        "compare": {
                            "opcode": "operator_equals",
                            "parent": "set_flag",
                            "inputs": {"OPERAND1": [3, "count_reporter", [10, ""]], "OPERAND2": [1, [4, "5"]]},
                            "fields": {}
                        },
                        "count_reporter": {
                            "opcode": "data_variable",
                            "parent": "compare",
                            "inputs": {},
                            "fields": {"VARIABLE": ["count", "count_id"]}
                        }
                    }
                }]
            })
            .to_string(),
        )
        .unwrap();

        let output = generate(&project);
        assert!(output.contains("scratch_round(state.get(2))"));
        assert!(output.contains("scratch_equals(state.get(0), 5.0_f64)"));
        assert!(output.contains("let scratch_new_value: ScratchValue = (state.get(0)).clone();"));
        assert!(!output.contains("scratch_bool(scratch_equals"));
        assert!(!output.contains("scratch_equals((state.get(0)).clone()"));
        assert!(!output.contains("scratch_round((state.get(2)).clone()"));
        assert_eq!(output.matches("scratch_round(state.get(2))").count(), 1);
        assert!(output.contains("return ScriptStatus::Yield;"));
    }

    #[test]
    fn emits_custom_blocks_as_resumable_argument_frames() {
        let project = parse_project(
            &json!({
                "targets": [{
                    "name": "Sprite1",
                    "isStage": false,
                    "variables": {"result_id": ["result", 0]},
                    "blocks": {
                        "hat": {
                            "opcode": "control_start_as_clone",
                            "next": "call",
                            "topLevel": true,
                            "inputs": {},
                            "fields": {}
                        },
                        "call": {
                            "opcode": "procedures_call",
                            "parent": "hat",
                            "inputs": {"argument_id": [1, [10, "hello"]]},
                            "fields": {},
                            "mutation": {
                                "proccode": "echo %s",
                                "argumentids": "[\"argument_id\"]",
                                "warp": "false"
                            }
                        },
                        "definition": {
                            "opcode": "procedures_definition",
                            "next": "set_result",
                            "topLevel": true,
                            "inputs": {"custom_block": [1, "prototype"]},
                            "fields": {}
                        },
                        "prototype": {
                            "opcode": "procedures_prototype",
                            "parent": "definition",
                            "inputs": {"argument_id": [1, "argument_default"]},
                            "fields": {},
                            "mutation": {
                                "proccode": "echo %s",
                                "argumentnames": "[\"value\"]",
                                "argumentids": "[\"argument_id\"]",
                                "argumentdefaults": "[\"\"]",
                                "warp": "false"
                            }
                        },
                        "argument_default": {
                            "opcode": "argument_reporter_string_number",
                            "parent": "prototype",
                            "shadow": true,
                            "inputs": {},
                            "fields": {"VALUE": ["value", null]}
                        },
                        "set_result": {
                            "opcode": "data_setvariableto",
                            "parent": "definition",
                            "inputs": {"VALUE": [3, "argument", [10, ""]]},
                            "fields": {"VARIABLE": ["result", "result_id"]}
                        },
                        "argument": {
                            "opcode": "argument_reporter_string_number",
                            "parent": "set_result",
                            "inputs": {},
                            "fields": {"VALUE": ["value", null]}
                        }
                    }
                }]
            })
            .to_string(),
        )
        .unwrap();

        let output = generate(&project);
        assert!(output.contains("arguments[0]).clone()"));
        assert!(output.contains("\"hello\").to_owned().into()"));
        assert!(output.contains("script.frames.push(ProcedureFrame"));
        assert!(output.contains("ProcedureFrame::new(1"));
        assert!(output.contains("new_clone_start_script_sprite1_1(_instance)"));
    }

    #[test]
    fn clone_nested_custom_calls_preserve_warp_mode_and_the_callers_pc() {
        let project = parse_project(
            &json!({
                "targets": [{
                    "name": "Sprite1",
                    "isStage": false,
                    "variables": {"done_id": ["done", false], "progress_id": ["progress", 0]},
                    "blocks": {
                        "hat": {"opcode": "control_start_as_clone", "next": "call_outer", "topLevel": true, "inputs": {}, "fields": {}},
                        "call_outer": {"opcode": "procedures_call", "next": "after_call", "parent": "hat", "inputs": {}, "fields": {}, "mutation": {"proccode": "warp outer", "argumentids": "[]", "warp": "false"}},
                        "after_call": {"opcode": "looks_say", "parent": "call_outer", "inputs": {"MESSAGE": [1, [10, "returned"]]}, "fields": {}},
                        "inner_definition": {"opcode": "procedures_definition", "next": "inner_change", "topLevel": true, "inputs": {"custom_block": [1, "inner_prototype"]}, "fields": {}},
                        "inner_prototype": {"opcode": "procedures_prototype", "parent": "inner_definition", "inputs": {}, "fields": {}, "mutation": {"proccode": "normal inner", "argumentnames": "[]", "argumentids": "[]", "argumentdefaults": "[]", "warp": "false"}},
                        "inner_change": {"opcode": "data_changevariableby", "parent": "inner_definition", "inputs": {"VALUE": [1, [4, "1"]]}, "fields": {"VARIABLE": ["progress", "progress_id"]}},
                        "outer_definition": {"opcode": "procedures_definition", "next": "outer_repeat", "topLevel": true, "inputs": {"custom_block": [1, "outer_prototype"]}, "fields": {}},
                        "outer_prototype": {"opcode": "procedures_prototype", "parent": "outer_definition", "inputs": {}, "fields": {}, "mutation": {"proccode": "warp outer", "argumentnames": "[]", "argumentids": "[]", "argumentdefaults": "[]", "warp": "true"}},
                        "outer_repeat": {"opcode": "control_repeat", "next": "wait_until", "parent": "outer_definition", "inputs": {"TIMES": [1, [4, "2"]], "SUBSTACK": [2, "call_inner"]}, "fields": {}},
                        "call_inner": {"opcode": "procedures_call", "next": "outer_change", "parent": "outer_repeat", "inputs": {}, "fields": {}, "mutation": {"proccode": "normal inner", "argumentids": "[]", "warp": "false"}},
                        "outer_change": {"opcode": "data_changevariableby", "parent": "call_inner", "inputs": {"VALUE": [1, [4, "10"]]}, "fields": {"VARIABLE": ["progress", "progress_id"]}},
                        "wait_until": {"opcode": "control_wait_until", "parent": "outer_repeat", "inputs": {"CONDITION": [2, "done_reporter"]}, "fields": {}},
                        "done_reporter": {"opcode": "data_variable", "parent": "wait_until", "inputs": {}, "fields": {"VARIABLE": ["done", "done_id"]}}
                    }
                }]
            })
            .to_string(),
        )
        .unwrap();

        let output = generate(&project);
        assert!(output.contains("fn new_clone_scripts(target: usize, _instance: usize)"));
        assert!(output.contains("new_clone_start_script_sprite1_1(_instance)"));
        assert!(output.contains("inherited_warp || true"));
        assert!(output.contains("inherited_warp || false"));
        assert!(output.contains("if !(scratch_bool(state.get(0)))"));
        assert!(output.contains("script.frames.last_mut().unwrap().pc = 1;"));
        assert!(
            output.contains("if script.yield_at_safe_point(clock) { return ScriptStatus::Yield; }")
        );
        assert!(output.contains("_ => { script.frames.pop(); continue; }"));
        assert!(output.contains("print_value!(state, \"say\", scratch_bubble_text.clone());"));
    }

    #[test]
    fn emits_and_invokes_each_green_flag_script_separately() {
        let project = parse_project(
            &json!({
                "targets": [{
                    "name": "Sprite1",
                    "isStage": false,
                    "variables": {"count_id": ["count", 0]},
                    "blocks": {
                        "first_hat": {
                            "opcode": "event_whenflagclicked",
                            "next": "first_repeat",
                            "topLevel": true,
                            "inputs": {},
                            "fields": {}
                        },
                        "first_repeat": {
                            "opcode": "control_repeat",
                            "next": null,
                            "parent": "first_hat",
                            "inputs": {"TIMES": [1, [4, "1"]], "SUBSTACK": [2, "first_set"]},
                            "fields": {}
                        },
                        "first_set": {
                            "opcode": "data_setvariableto",
                            "parent": "first_repeat",
                            "inputs": {"VALUE": [1, [4, "1"]]},
                            "fields": {"VARIABLE": ["count", "count_id"]}
                        },
                        "second_hat": {
                            "opcode": "event_whenflagclicked",
                            "next": "second_repeat",
                            "topLevel": true,
                            "inputs": {},
                            "fields": {}
                        },
                        "second_repeat": {
                            "opcode": "control_repeat",
                            "next": null,
                            "parent": "second_hat",
                            "inputs": {"TIMES": [1, [4, "1"]], "SUBSTACK": [2, "second_set"]},
                            "fields": {}
                        },
                        "second_set": {
                            "opcode": "data_setvariableto",
                            "parent": "second_repeat",
                            "inputs": {"VALUE": [1, [4, "2"]]},
                            "fields": {"VARIABLE": ["count", "count_id"]}
                        }
                    }
                }]
            })
            .to_string(),
        )
        .unwrap();

        let output = generate(&project);
        assert!(output.contains("fn step_green_flag_script_sprite1_1"));
        assert!(output.contains("fn step_green_flag_script_sprite1_2"));
        // each flag script keeps its own native write while sharing the variable slot.
        assert!(output.contains("state.set_local_number(0, 1.0_f64);"));
        assert!(output.contains("state.set_local_number(0, 2.0_f64);"));
        assert!(output.contains("fn new_scripts() -> Vec<GeneratedScript> { vec![new_green_flag_script_sprite1_1(), new_green_flag_script_sprite1_2()] }"));
        assert!(output.contains("pub fn run_until_complete(&mut self)"));
        assert!(output.contains("pub fn step(&mut self) -> bool"));
        assert!(output.contains("pub fn restart(&mut self)"));
        // generated repeat branches may emit more than one valid yield site per script.
        assert!(output.matches("return ScriptStatus::Yield;").count() >= 2);
    }

    #[test]
    fn wait_in_custom_block_advances_its_frame_before_scheduler_suspension() {
        let project = parse_project(
            &json!({
                "targets": [{
                    "name": "Sprite1",
                    "isStage": false,
                    "variables": {},
                    "blocks": {
                        "hat": {"opcode": "control_start_as_clone", "next": "call", "topLevel": true, "inputs": {}, "fields": {}},
                        "call": {"opcode": "procedures_call", "next": null, "parent": "hat", "inputs": {}, "fields": {}, "mutation": {"proccode": "delay", "argumentids": "[]", "warp": "true"}},
                        "definition": {"opcode": "procedures_definition", "next": "wait", "topLevel": true, "inputs": {"custom_block": [1, "prototype"]}, "fields": {}},
                        "prototype": {"opcode": "procedures_prototype", "parent": "definition", "inputs": {}, "fields": {}, "mutation": {"proccode": "delay", "argumentnames": "[]", "argumentids": "[]", "argumentdefaults": "[]", "warp": "true"}},
                        "wait": {"opcode": "control_wait", "next": "say", "parent": "definition", "inputs": {"DURATION": [1, [4, "0.25"]]}, "fields": {}},
                        "say": {"opcode": "looks_say", "next": null, "parent": "wait", "inputs": {"MESSAGE": [1, [10, "done"]]}, "fields": {}}
                    }
                }]
            }).to_string(),
        ).unwrap();

        let output = generate(&project);
        assert!(!output.contains("wait_deadlines"));
        assert!(output.contains("script.frames.last_mut().unwrap().pc = 1;"));
        assert!(output.contains("ScriptStatus::WaitUntil(wait_deadline)"));
        assert!(output.contains("clock.now_millis().saturating_add(duration_ms)"));
        assert!(!output.contains("clock.now_millis() < wait_deadline"));
        assert!(output.contains("inherited_warp || true"));
        assert!(output.contains("fn new_clone_scripts(target: usize, _instance: usize)"));
        assert!(!output.contains("if script.warp_budget_expired(clock) { return ScriptStatus::Yield; }\n                            continue;"));
        assert!(output.contains("pub fn advance(&mut self) -> SchedulerStatus"));
    }

    #[test]
    fn lowers_nested_repeat_until_and_forever_with_resumable_loop_state() {
        let project = parse_project(
            &json!({
                "targets": [{
                    "name": "Sprite1",
                    "isStage": false,
                    "variables": {
                        "count_id": ["count", 2],
                        "done_id": ["done", false]
                    },
                    "blocks": {
                        "hat": {
                            "opcode": "event_whenflagclicked",
                            "next": "outer_repeat",
                            "topLevel": true,
                            "inputs": {},
                            "fields": {}
                        },
                        "outer_repeat": {
                            "opcode": "control_repeat",
                            "next": null,
                            "parent": "hat",
                            "inputs": {"TIMES": [1, [4, "2"]], "SUBSTACK": [2, "inner_repeat"]},
                            "fields": {}
                        },
                        "inner_repeat": {
                            "opcode": "control_repeat",
                            "next": null,
                            "parent": "outer_repeat",
                            "inputs": {"TIMES": [3, "count_reporter", [10, "0"]], "SUBSTACK": [2, "until"]},
                            "fields": {}
                        },
                        "count_reporter": {
                            "opcode": "data_variable",
                            "parent": "inner_repeat",
                            "inputs": {},
                            "fields": {"VARIABLE": ["count", "count_id"]}
                        },
                        "until": {
                            "opcode": "control_repeat_until",
                            "next": null,
                            "parent": "inner_repeat",
                            "inputs": {"CONDITION": [2, "done_reporter"], "SUBSTACK": [2, "forever"]},
                            "fields": {}
                        },
                        "done_reporter": {
                            "opcode": "data_variable",
                            "parent": "until",
                            "inputs": {},
                            "fields": {"VARIABLE": ["done", "done_id"]}
                        },
                        "forever": {
                            "opcode": "control_forever",
                            "next": null,
                            "parent": "until",
                            "inputs": {"SUBSTACK": [2, "set_done"]},
                            "fields": {}
                        },
                        "set_done": {
                            "opcode": "data_setvariableto",
                            "next": null,
                            "parent": "forever",
                            "inputs": {"VALUE": [1, [4, "1"]]},
                            "fields": {"VARIABLE": ["done", "done_id"]}
                        }
                    }
                }]
            })
            .to_string(),
        )
        .unwrap();

        let output = generate(&project);
        assert!(output.contains("vec![0; 2]"));
        assert!(
            output.contains("script.frames.last_mut().unwrap().repeat_counts[0] = repeat_count;")
        );
        assert!(
            output.contains("script.frames.last_mut().unwrap().repeat_counts[1] = repeat_count;")
        );
        assert_eq!(
            output.matches("let repeat_count = scratch_round").count(),
            2
        );
        assert!(output.contains("if !(scratch_bool(state.get(1)))"));
        assert!(output.contains("script.frames.last_mut().unwrap().pc = 8;"));
        assert!(output.contains("return ScriptStatus::Yield;"));
    }

    #[test]
    fn shared_state_slots_keep_same_named_target_variables_distinct() {
        let project = parse_project(
            &json!({
                "targets": [
                    {
                        "name": "Stage",
                        "isStage": true,
                        "variables": {"stage_score_id": ["score", 0]},
                        "blocks": {
                            "stage_hat": {"opcode": "event_whenflagclicked", "next": "stage_set", "topLevel": true, "inputs": {}, "fields": {}},
                            "stage_set": {"opcode": "data_setvariableto", "next": null, "parent": "stage_hat", "inputs": {"VALUE": [1, [4, "1"]]}, "fields": {"VARIABLE": ["score", "stage_score_id"]}}
                        }
                    },
                    {
                        "name": "Sprite1",
                        "isStage": false,
                        "variables": {"sprite_score_id": ["score", 0]},
                        "blocks": {
                            "sprite_hat": {"opcode": "event_whenflagclicked", "next": "sprite_set", "topLevel": true, "inputs": {}, "fields": {}},
                            "sprite_set": {"opcode": "data_setvariableto", "next": null, "parent": "sprite_hat", "inputs": {"VALUE": [1, [4, "2"]]}, "fields": {"VARIABLE": ["score", "sprite_score_id"]}}
                        }
                    }
                ]
            })
            .to_string(),
        )
        .unwrap();

        let output = generate(&project);
        assert!(output.contains(
            "ScratchVariableMetadata::for_target(\"stage_score_id\", \"score\", \"Stage\", 0, true)"
        ));
        assert!(output.contains(
            "ScratchVariableMetadata::for_target(\"sprite_score_id\", \"score\", \"Sprite1\", 1, false)"
        ));
        // numeric writes retain stage and sprite ownership in separate fast paths.
        assert!(output.contains("state.set_shared_number(1, 1.0_f64);"));
        assert!(output.contains("state.set_local_number(0, 2.0_f64);"));
    }

    #[test]
    fn sprite_backdrop_event_fixture_generates_scoped_clicks_and_edge_hats() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/src/sprite-backdrop-events.sb3"
        );
        let json = crate::extractor::read_project_json(path).unwrap();
        let project = parse_project(&json).unwrap();
        let output = generate(&project);

        assert!(output.contains("state.loudness() > scratch_to_number(-10.0_f64)"));
        assert!(
            output
                .contains("state.project_timer_seconds(now_millis) > scratch_to_number(10.0_f64)")
        );
        assert!(output.contains("previous.rising_edge("));
        assert!(output.contains("self.scheduler.start_edge_hats(starters)"));
        assert!(output.contains("pointer_click_activates("));
        assert!(output.contains("state.instance_target(instance)"));
        assert!(output.contains("None => { if let Some(instance) = state.original_instance(0)"));
        assert!(output.contains("sprite_clicked_script_sprite1"));
        assert!(output.contains("stage_clicked_script_stage"));
        assert!(output.contains("state.reset_project_timer(clock.now_millis());"));
        assert!(output.contains("state.reset_project_timer(self.scheduler.now_millis());"));
        assert!(output.contains("pub fn restart(&mut self) { self.edge_hat_previous.clear();"));
        assert!(
            output.contains(
                "pub fn stop_all(&mut self) { self.scheduler.stop_all(&mut self.state); }"
            )
        );
        assert!(!output.contains("-> GeneratedScript { new_greater_than_script_"));
        assert!(!output.contains("-> GeneratedScript { new_sprite_clicked_script_"));
    }

    #[test]
    fn greater_than_hat_defaults_match_scratch_block_defaults() {
        let project = parse_project(
            &json!({
                "targets": [{
                    "name": "Stage",
                    "isStage": true,
                    "blocks": {
                        "threshold": {
                            "opcode": "event_whengreaterthan",
                            "topLevel": true,
                            "inputs": {},
                            "fields": {}
                        }
                    }
                }]
            })
            .to_string(),
        )
        .unwrap();
        let output = generate(&project);
        assert!(output.contains("state.loudness() > scratch_to_number(10.0_f64)"));
    }

    #[test]
    fn sound_fixture_generates_resumable_playback_and_tick_yielding_effects() {
        let input = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/src/sound.sb3");
        let json = crate::extractor::read_project_json(input).unwrap();
        let project = parse_project(&json).unwrap();
        let output = generate(&project);

        assert!(output.contains("pub fn uses_sound() -> bool { true }"));
        assert!(output.contains("state.set_original_volume(1, 100_f64);"));
        assert!(output.contains("state.start_sound(scratch_sound_selector, clock.now_millis())"));
        assert!(output.contains("ScriptStatus::WaitForSignal(sound_wait)"));
        assert!(output.contains("ScratchSoundEffect::Pitch"));
        assert!(output.contains("ScratchSoundEffect::Pan"));
        assert!(output.contains("state.set_sound_volume(scratch_to_number("));
        assert!(output.contains("sound_volume_for_target(1)"));
        assert!(output.contains("const REFRESH_SOUND_VOLUME_MONITOR: bool = true;"));
        assert!(
            output.contains("if REFRESH_SOUND_VOLUME_MONITOR { state.request_presentation(); }")
        );
        assert!(output.contains("return ScriptStatus::Yield;"));
        assert!(!output.contains("unsupported statement: sound_"));
        assert!(!output.contains("unsupported reporter: sound_"));
    }
}
