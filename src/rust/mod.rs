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

mod expressions;
mod literals;
mod project;
mod scripts;
mod statements;

pub use project::generate;

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::generate;
    use crate::parser::parse_project;

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
        assert!(output.contains("fn new_clone_scripts(target: usize, instance: usize)"));
        assert!(output.contains("new_clone_start_script_sprite1_2(instance)"));
        assert!(output.contains("new_clone_start_script_sprite1_5(instance)"));
        assert!(output.contains("ScratchVariableMetadata::for_target(\"`jEk@4|i[#Fk?(8x)AV.-my variable\", \"my variable\", \"Stage\", 0, true)"));
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
        assert!(output.contains("0 => {\n                    let scratch_new_value"));
        assert!(output.contains(
            "state.set(0, scratch_new_value);\n                    let scratch_new_value"
        ));
        assert!(!output.contains("script.frames.last_mut().unwrap().pc = 1;"));
        assert!(output.contains("script.frames.last_mut().unwrap().pc = usize::MAX;"));
    }

    #[test]
    fn numeric_loops_use_native_locals_and_commit_before_safeguard_yields() {
        let project = parse_project(
            &json!({
                "targets": [{
                    "name": "Sprite1",
                    "isStage": false,
                    "variables": {"count_id": ["count", 0]},
                    "blocks": {
                        "hat": {"opcode": "event_whenflagclicked", "next": "repeat", "topLevel": true, "inputs": {}, "fields": {}},
                        "repeat": {"opcode": "control_repeat", "next": null, "inputs": {"TIMES": [1, [4, "10"]], "SUBSTACK": [2, "change"]}, "fields": {}},
                        "change": {"opcode": "data_changevariableby", "next": null, "inputs": {"VALUE": [3, "multiply", [4, "0"]]}, "fields": {"VARIABLE": ["count", "count_id"]}},
                        "multiply": {"opcode": "operator_multiply", "next": null, "inputs": {"NUM1": [1, [4, "1"]], "NUM2": [3, "equal", [4, "0"]]}, "fields": {}},
                        "equal": {"opcode": "operator_equals", "next": null, "inputs": {"OPERAND1": [1, [10, "same"]], "OPERAND2": [1, [10, "same"]]}, "fields": {}}
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
        assert!(output.contains("arguments: vec![(\"hello\").to_owned().into()]"));
        assert!(output.contains("script.frames.push(ProcedureFrame"));
        assert!(output.contains("function: 1"));
        assert!(output.contains("new_clone_start_script_sprite1_1(instance)"));
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
        assert!(output.contains("fn new_clone_scripts(target: usize, instance: usize)"));
        assert!(output.contains("new_clone_start_script_sprite1_1(instance)"));
        assert!(output.contains("warp_mode: inherited_warp || true"));
        assert!(output.contains("warp_mode: inherited_warp || false"));
        assert!(output.contains("if !(scratch_bool(state.get(0)))"));
        assert!(output.contains("script.frames.last_mut().unwrap().pc = 1;"));
        assert!(
            output.contains("if script.yield_at_safe_point(clock) { return ScriptStatus::Yield; }")
        );
        assert!(output.contains("_ => { script.frames.pop(); continue; }"));
        assert!(output.contains("print_value!(\"say\", \"returned\");"));
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
        assert!(output.contains("scratch_local_0 = 1.0_f64;"));
        assert!(output.contains("scratch_local_0 = 2.0_f64;"));
        assert!(output.contains("fn new_scripts() -> Vec<GeneratedScript> { vec![new_green_flag_script_sprite1_1(), new_green_flag_script_sprite1_2()] }"));
        assert!(output.contains("pub fn run_until_complete(&mut self)"));
        assert!(output.contains("pub fn step(&mut self) -> bool"));
        assert!(output.contains("pub fn restart(&mut self)"));
        assert_eq!(output.matches("return ScriptStatus::Yield;").count(), 2);
    }

    #[test]
    fn wait_block_keeps_its_deadline_in_the_active_procedure_frame() {
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
        assert!(output.contains("wait_deadlines: vec![None; 1]"));
        assert!(output.contains("ScriptStatus::WaitUntil(wait_deadline)"));
        assert!(output.contains("clock.now_millis().saturating_add(duration_ms)"));
        assert!(output.contains("wait_deadlines[0] = None"));
        assert!(output.contains("warp_mode: inherited_warp || true"));
        assert!(output.contains("fn new_clone_scripts(target: usize, instance: usize)"));
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
        assert!(output.contains("repeat_counts: vec![0; 2]"));
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
        assert!(output.contains("state.set(0, scratch_new_value);"));
        assert!(output.contains("state.set(1, scratch_new_value);"));
    }
}
