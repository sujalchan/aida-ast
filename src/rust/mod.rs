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
        assert!(output.contains("scratch_to_number(amount.clone()).floor()"));
        assert!(output.contains("scratch_equals(\"5\", 5.0_f64)"));
    }
}
