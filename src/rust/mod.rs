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
        assert!(output.contains("scratch_to_number(&amount).floor()"));
        assert!(output.contains("scratch_equals(\"5\", 5.0_f64)"));
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
        assert!(output.contains("scratch_round(&text)"));
        assert!(output.contains("while scratch_equals(&count, 5.0_f64)"));
        assert!(output.contains("flag = count.clone();"));
        assert!(!output.contains("scratch_bool(scratch_equals"));
        assert!(!output.contains("scratch_equals(count.clone()"));
        assert!(!output.contains("scratch_round(text.clone()"));
        assert_eq!(output.matches("scratch_round(&text)").count(), 1);
    }

    #[test]
    fn emits_custom_blocks_as_functions_with_arguments_and_state() {
        let project = parse_project(
            &json!({
                "targets": [{
                    "name": "Sprite1",
                    "isStage": false,
                    "variables": {"result_id": ["result", 0]},
                    "blocks": {
                        "hat": {
                            "opcode": "event_whenflagclicked",
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
        assert!(output.contains("fn echo(value: ScratchValue, state_result: &mut ScratchValue) {"));
        assert!(output.contains("let mut result = state_result.clone();"));
        assert!(output.contains("result = value.clone();"));
        assert!(output.contains("*state_result = result;"));
        assert!(output.contains("echo((\"hello\").into(), &mut result);"));
        assert!(output.find("fn echo").unwrap() < output.find("pub fn run").unwrap());
    }
}
