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

//! lowers Scratch inputs and reporter blocks into Rust expressions.
//!
//! follows reporter references recursively, selects a lowering by opcode, and
//! uses the sibling literal helpers to preserve Scratch input representations.

use std::collections::{HashMap, HashSet};

use serde_json::Value;

use crate::parser::{InputValue, ScratchBlock, ScratchTarget, parse_input};

use super::{
    literals::{field_value, number_literal, quote, text_literal},
    project::{ListInfo, VariableInfo},
    statements::variable_field,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LocalValueKind {
    Number,
    Text,
    Boolean,
}

pub(super) struct LocalExpression {
    pub(super) code: String,
    pub(super) kind: LocalValueKind,
}

fn list_field(
    block: &ScratchBlock,
    field: &str,
    lists: &HashMap<String, ListInfo>,
) -> Option<ListInfo> {
    let data = block.fields.get(field)?.as_array()?;
    lists.get(data.get(1)?.as_str()?).cloned()
}

// convert a scratch input into a rust value expression, following reporter links recursively.
pub(super) fn expression(
    target: &ScratchTarget,
    input: Option<&Value>,
    variables: &HashMap<String, VariableInfo>,
    lists: &HashMap<String, ListInfo>,
    arguments: &HashMap<String, String>,
    active: &mut HashSet<String>,
) -> String {
    match input.and_then(parse_input) {
        Some(InputValue::Number(value)) => number_literal(value),
        Some(InputValue::String(value)) => text_literal(value),
        Some(InputValue::VariableReference(reference)) => variables
            .get(reference.id)
            .map(|variable| format!("state.get({})", variable.slot))
            .unwrap_or_else(|| unsupported_expression("unknown variable")),
        Some(InputValue::BlockReference(id)) => {
            if !active.insert(id.to_owned()) {
                return unsupported_expression(&format!("expression cycle: {id}"));
            }
            let result = target
                .blocks
                .get(id)
                .map(|block| block_expression(target, block, variables, lists, arguments, active))
                .unwrap_or_else(|| unsupported_expression(&format!("missing block: {id}")));
            active.remove(id);
            result
        }
        Some(InputValue::Empty) => unsupported_expression("empty input"),
        Some(InputValue::ListReference(reference)) => lists
            .get(reference.id)
            .map(|list| format!("state.list({}).contents()", list.slot))
            .unwrap_or_else(|| unsupported_expression("unknown list")),
        None => unsupported_expression("missing or invalid input"),
    }
}

// choose the rust lowering for a reporter block based on its scratch opcode.
fn block_expression(
    target: &ScratchTarget,
    block: &ScratchBlock,
    variables: &HashMap<String, VariableInfo>,
    lists: &HashMap<String, ListInfo>,
    arguments: &HashMap<String, String>,
    active: &mut HashSet<String>,
) -> String {
    let input = |name: &str, active: &mut HashSet<String>| {
        expression(
            target,
            block.inputs.get(name),
            variables,
            lists,
            arguments,
            active,
        )
    };
    let binary = |function: &str, left: &str, right: &str, active: &mut HashSet<String>| {
        format!(
            "{function}({}, {})",
            input(left, active),
            input(right, active)
        )
    };
    match block.opcode.as_str() {
        "operator_add" => binary("scratch_add", "NUM1", "NUM2", active),
        "operator_subtract" => binary("scratch_subtract", "NUM1", "NUM2", active),
        "operator_multiply" => binary("scratch_multiply", "NUM1", "NUM2", active),
        "operator_divide" => binary("scratch_divide", "NUM1", "NUM2", active),
        "operator_mod" => binary("scratch_mod", "NUM1", "NUM2", active),
        "operator_equals" if identical_literal_inputs(block) => "true".to_owned(),
        "operator_equals" => binary("scratch_equals", "OPERAND1", "OPERAND2", active),
        "operator_gt" => binary("scratch_greater_than", "OPERAND1", "OPERAND2", active),
        "operator_lt" => binary("scratch_less_than", "OPERAND1", "OPERAND2", active),
        "operator_and" => format!(
            "scratch_and({}, {})",
            boolean_input(
                target, block, "OPERAND1", variables, lists, arguments, active
            ),
            boolean_input(
                target, block, "OPERAND2", variables, lists, arguments, active
            )
        ),
        "operator_or" => format!(
            "scratch_or({}, {})",
            boolean_input(
                target, block, "OPERAND1", variables, lists, arguments, active
            ),
            boolean_input(
                target, block, "OPERAND2", variables, lists, arguments, active
            )
        ),
        "operator_join" => binary("scratch_join", "STRING1", "STRING2", active),
        "operator_letter_of" => binary("scratch_letter", "LETTER", "STRING", active),
        "operator_length" => format!("scratch_length({})", input("STRING", active)),
        "operator_contains" => binary("scratch_contains", "STRING1", "STRING2", active),
        "operator_round" => format!("scratch_round({})", input("NUM", active)),
        "operator_not" => format!(
            "scratch_not({})",
            boolean_input(
                target, block, "OPERAND", variables, lists, arguments, active
            )
        ),
        "operator_random" => format!(
            "scratch_random({}, {})",
            random_argument(
                target,
                block.inputs.get("FROM"),
                variables,
                lists,
                arguments,
                active,
            ),
            random_argument(
                target,
                block.inputs.get("TO"),
                variables,
                lists,
                arguments,
                active,
            )
        ),
        "operator_mathop" => math_expression(target, block, variables, lists, arguments, active),
        "data_variable" => variable_field(block, "VARIABLE", variables)
            .map(|variable| format!("state.get({})", variable.slot))
            .unwrap_or_else(|| unsupported_expression("missing variable reference")),
        "data_listcontents" => list_field(block, "LIST", lists)
            .map(|list| format!("state.list({}).contents()", list.slot))
            .unwrap_or_else(|| unsupported_expression("missing list reference")),
        "data_itemoflist" => list_field(block, "LIST", lists)
            .map(|list| format!("state.list({}).get({})", list.slot, input("INDEX", active)))
            .unwrap_or_else(|| unsupported_expression("missing list reference")),
        "data_lengthoflist" => list_field(block, "LIST", lists)
            .map(|list| format!("state.list({}).length() as f64", list.slot))
            .unwrap_or_else(|| unsupported_expression("missing list reference")),
        "data_itemnumoflist" => list_field(block, "LIST", lists)
            .map(|list| {
                format!(
                    "state.list({}).item_number({}) as f64",
                    list.slot,
                    input("ITEM", active)
                )
            })
            .unwrap_or_else(|| unsupported_expression("missing list reference")),
        "data_listcontainsitem" => list_field(block, "LIST", lists)
            .map(|list| {
                format!(
                    "state.list({}).contains({})",
                    list.slot,
                    input("ITEM", active)
                )
            })
            .unwrap_or_else(|| unsupported_expression("missing list reference")),
        "argument_reporter_string_number" | "argument_reporter_boolean" => {
            field_value(block, "VALUE")
                .and_then(Value::as_str)
                .and_then(|name| arguments.get(name))
                .map(|name| format!("&{name}"))
                .unwrap_or_else(|| unsupported_expression("missing procedure argument"))
        }
        "math_number"
        | "math_integer"
        | "math_whole_number"
        | "math_positive_number"
        | "math_angle" => field_value(block, "NUM")
            .map(number_literal)
            .unwrap_or_else(|| unsupported_expression("missing numeric field")),
        "text" => field_value(block, "TEXT")
            .map(text_literal)
            .unwrap_or_else(|| unsupported_expression("missing text field")),
        opcode => unsupported_expression(&format!("unsupported reporter: {opcode}")),
    }
}

fn identical_literal_inputs(block: &ScratchBlock) -> bool {
    let left = block.inputs.get("OPERAND1").and_then(parse_input);
    let right = block.inputs.get("OPERAND2").and_then(parse_input);
    match (left, right) {
        (Some(InputValue::Number(left)), Some(InputValue::Number(right))) => {
            number_literal(left) == number_literal(right)
        }
        (Some(InputValue::String(left)), Some(InputValue::String(right))) => left == right,
        _ => false,
    }
}

// collect variables that a native numeric loop can safely keep in f64 locals.
pub(super) fn collect_numeric_local_slots(
    target: &ScratchTarget,
    input: Option<&Value>,
    variables: &HashMap<String, VariableInfo>,
    slots: &mut HashSet<usize>,
    active: &mut HashSet<String>,
) -> bool {
    match input.and_then(parse_input) {
        Some(InputValue::Number(_) | InputValue::String(_)) => true,
        Some(InputValue::VariableReference(reference)) => variables
            .get(reference.id)
            .filter(|variable| variable.numeric)
            .is_some_and(|variable| {
                slots.insert(variable.slot);
                true
            }),
        Some(InputValue::BlockReference(id)) => {
            if !active.insert(id.to_owned()) {
                return false;
            }
            let result = target.blocks.get(id).is_some_and(|block| {
                if !matches!(
                    block.opcode.as_str(),
                    "operator_add"
                        | "operator_subtract"
                        | "operator_multiply"
                        | "operator_divide"
                        | "operator_mod"
                        | "operator_equals"
                        | "operator_gt"
                        | "operator_lt"
                        | "operator_and"
                        | "operator_or"
                        | "operator_not"
                        | "operator_round"
                        | "operator_mathop"
                        | "data_variable"
                        | "math_number"
                        | "math_integer"
                        | "math_whole_number"
                        | "math_positive_number"
                        | "math_angle"
                        | "text"
                ) {
                    return false;
                }
                if block.opcode == "data_variable" {
                    return variable_field(block, "VARIABLE", variables)
                        .filter(|variable| variable.numeric)
                        .is_some_and(|variable| {
                            slots.insert(variable.slot);
                            true
                        });
                }
                block.inputs.values().all(|input| {
                    collect_numeric_local_slots(target, Some(input), variables, slots, active)
                })
            });
            active.remove(id);
            result
        }
        _ => false,
    }
}

// lower an expression using f64 locals for variables proven numeric project-wide.
pub(super) fn local_expression(
    target: &ScratchTarget,
    input: Option<&Value>,
    variables: &HashMap<String, VariableInfo>,
    locals: &HashMap<usize, String>,
    active: &mut HashSet<String>,
) -> Option<LocalExpression> {
    match input.and_then(parse_input)? {
        InputValue::Number(value) => Some(LocalExpression {
            code: number_literal(value),
            kind: LocalValueKind::Number,
        }),
        InputValue::String(value) => Some(LocalExpression {
            code: text_literal(value),
            kind: LocalValueKind::Text,
        }),
        InputValue::VariableReference(reference) => {
            let variable = variables.get(reference.id)?;
            Some(LocalExpression {
                code: locals.get(&variable.slot)?.clone(),
                kind: LocalValueKind::Number,
            })
        }
        InputValue::BlockReference(id) => {
            if !active.insert(id.to_owned()) {
                return None;
            }
            let result = target
                .blocks
                .get(id)
                .and_then(|block| local_block_expression(target, block, variables, locals, active));
            active.remove(id);
            result
        }
        _ => None,
    }
}

fn local_block_expression(
    target: &ScratchTarget,
    block: &ScratchBlock,
    variables: &HashMap<String, VariableInfo>,
    locals: &HashMap<usize, String>,
    active: &mut HashSet<String>,
) -> Option<LocalExpression> {
    let input = |name: &str, active: &mut HashSet<String>| {
        local_expression(target, block.inputs.get(name), variables, locals, active)
    };
    let numeric_binary = |function: &str, left: &str, right: &str, active: &mut HashSet<String>| {
        let left = input(left, active)?;
        let right = input(right, active)?;
        Some(LocalExpression {
            code: format!("{function}({}, {})", left.code, right.code),
            kind: LocalValueKind::Number,
        })
    };
    let comparison = |function: &str, left: &str, right: &str, active: &mut HashSet<String>| {
        let left = input(left, active)?;
        let right = input(right, active)?;
        Some(LocalExpression {
            code: format!("{function}({}, {})", left.code, right.code),
            kind: LocalValueKind::Boolean,
        })
    };

    match block.opcode.as_str() {
        "operator_add" => numeric_binary("scratch_add", "NUM1", "NUM2", active),
        "operator_subtract" => numeric_binary("scratch_subtract", "NUM1", "NUM2", active),
        "operator_multiply" => numeric_binary("scratch_multiply", "NUM1", "NUM2", active),
        "operator_divide" => numeric_binary("scratch_divide", "NUM1", "NUM2", active),
        "operator_mod" => numeric_binary("scratch_mod", "NUM1", "NUM2", active),
        "operator_equals" if identical_literal_inputs(block) => Some(LocalExpression {
            code: "true".to_owned(),
            kind: LocalValueKind::Boolean,
        }),
        "operator_equals" => comparison("scratch_equals", "OPERAND1", "OPERAND2", active),
        "operator_gt" => comparison("scratch_greater_than", "OPERAND1", "OPERAND2", active),
        "operator_lt" => comparison("scratch_less_than", "OPERAND1", "OPERAND2", active),
        "operator_and" | "operator_or" => {
            let left = local_to_bool(input("OPERAND1", active)?);
            let right = local_to_bool(input("OPERAND2", active)?);
            let operator = if block.opcode == "operator_and" {
                "&&"
            } else {
                "||"
            };
            Some(LocalExpression {
                code: format!("({left} {operator} {right})"),
                kind: LocalValueKind::Boolean,
            })
        }
        "operator_not" => Some(LocalExpression {
            code: format!("!({})", local_to_bool(input("OPERAND", active)?)),
            kind: LocalValueKind::Boolean,
        }),
        "operator_round" => Some(LocalExpression {
            code: format!("scratch_round({})", local_to_number(input("NUM", active)?)),
            kind: LocalValueKind::Number,
        }),
        "operator_mathop" => {
            let value = input("NUM", active)?;
            let numeric = format!("scratch_to_number({})", value.code);
            let operator = field_value(block, "OPERATOR")?.as_str()?;
            let code = match operator {
                "sin" => format!("scratch_sin({})", value.code),
                "cos" => format!("scratch_cos({})", value.code),
                "tan" => format!("scratch_tan({})", value.code),
                "asin" => format!("scratch_asin_degrees({})", value.code),
                "acos" => format!("scratch_acos_degrees({})", value.code),
                "atan" => format!("scratch_atan_degrees({})", value.code),
                "abs" => format!("({numeric}).abs()"),
                "floor" => format!("({numeric}).floor()"),
                "ceiling" => format!("({numeric}).ceil()"),
                "sqrt" => format!("({numeric}).sqrt()"),
                "ln" => format!("({numeric}).ln()"),
                "log" => format!("({numeric}).log10()"),
                "e ^" => format!("({numeric}).exp()"),
                "10 ^" => format!("10.0_f64.powf({numeric})"),
                _ => return None,
            };
            Some(LocalExpression {
                code,
                kind: LocalValueKind::Number,
            })
        }
        "data_variable" => {
            let variable = variable_field(block, "VARIABLE", variables)?;
            Some(LocalExpression {
                code: locals.get(&variable.slot)?.clone(),
                kind: LocalValueKind::Number,
            })
        }
        "math_number"
        | "math_integer"
        | "math_whole_number"
        | "math_positive_number"
        | "math_angle" => Some(LocalExpression {
            code: number_literal(field_value(block, "NUM")?),
            kind: LocalValueKind::Number,
        }),
        "text" => Some(LocalExpression {
            code: text_literal(field_value(block, "TEXT")?),
            kind: LocalValueKind::Text,
        }),
        _ => None,
    }
}

pub(super) fn local_to_number(value: LocalExpression) -> String {
    match value.kind {
        LocalValueKind::Number => value.code,
        LocalValueKind::Boolean => format!("if {} {{ 1.0_f64 }} else {{ 0.0_f64 }}", value.code),
        LocalValueKind::Text => format!("scratch_to_number({})", value.code),
    }
}

pub(super) fn local_to_bool(value: LocalExpression) -> String {
    match value.kind {
        LocalValueKind::Boolean => value.code,
        LocalValueKind::Number | LocalValueKind::Text => format!("scratch_bool({})", value.code),
    }
}

// lower math reporters, keeping literal numbers direct and converting dynamic inputs at runtime.
fn math_expression(
    target: &ScratchTarget,
    block: &ScratchBlock,
    variables: &HashMap<String, VariableInfo>,
    lists: &HashMap<String, ListInfo>,
    arguments: &HashMap<String, String>,
    active: &mut HashSet<String>,
) -> String {
    let value = expression(
        target,
        block.inputs.get("NUM"),
        variables,
        lists,
        arguments,
        active,
    );
    let numeric = numeric_literal(target, block.inputs.get("NUM"))
        .map(|literal| {
            if literal.starts_with('-') {
                format!("({literal})")
            } else {
                literal
            }
        })
        .unwrap_or_else(|| format!("scratch_to_number({value})"));
    let Some(operator) = field_value(block, "OPERATOR").and_then(Value::as_str) else {
        return unsupported_expression("missing math operator");
    };
    match operator {
        "sin" => return format!("scratch_sin({value})"),
        "cos" => return format!("scratch_cos({value})"),
        "tan" => return format!("scratch_tan({value})"),
        "asin" => return format!("scratch_asin_degrees({value})"),
        "acos" => return format!("scratch_acos_degrees({value})"),
        "atan" => return format!("scratch_atan_degrees({value})"),
        _ => {}
    }
    let operation = match operator {
        "abs" => "abs()",
        "floor" => "floor()",
        "ceiling" => "ceil()",
        "sqrt" => "sqrt()",
        "ln" => "ln()",
        "log" => "log10()",
        "e ^" => "exp()",
        "10 ^" => return format!("10.0_f64.powf({numeric})"),
        _ => {
            return unsupported_expression(&format!("unsupported math operator: {operator}"));
        }
    };
    format!("{numeric}.{operation}")
}

// return a direct numeric literal when the input is a number primitive or number reporter.
fn numeric_literal(target: &ScratchTarget, input: Option<&Value>) -> Option<String> {
    match input.and_then(parse_input)? {
        InputValue::Number(value) => Some(number_literal(value)),
        InputValue::BlockReference(id) => {
            let block = target.blocks.get(id)?;
            matches!(
                block.opcode.as_str(),
                "math_number"
                    | "math_integer"
                    | "math_whole_number"
                    | "math_positive_number"
                    | "math_angle"
            )
            .then(|| field_value(block, "NUM").map(number_literal))?
        }
        _ => None,
    }
}

// preserve numeric spelling for random inputs because scratch distinguishes 10 from 10.0.
fn random_argument(
    target: &ScratchTarget,
    input: Option<&Value>,
    variables: &HashMap<String, VariableInfo>,
    lists: &HashMap<String, ListInfo>,
    arguments: &HashMap<String, String>,
    active: &mut HashSet<String>,
) -> String {
    match input.and_then(parse_input) {
        Some(InputValue::Number(value)) => value
            .as_str()
            .map(|text| quote(text))
            .or_else(|| value.as_number().map(|number| quote(&number.to_string())))
            .unwrap_or_else(|| expression(target, input, variables, lists, arguments, active)),
        Some(InputValue::BlockReference(id)) => {
            let Some(block) = target.blocks.get(id) else {
                return expression(target, input, variables, lists, arguments, active);
            };
            if matches!(
                block.opcode.as_str(),
                "math_number"
                    | "math_integer"
                    | "math_whole_number"
                    | "math_positive_number"
                    | "math_angle"
            ) {
                if let Some(value) = field_value(block, "NUM") {
                    if let Some(text) = value.as_str() {
                        return quote(text);
                    }
                    if let Some(number) = value.as_number() {
                        return quote(&number.to_string());
                    }
                }
            }
            expression(target, input, variables, lists, arguments, active)
        }
        _ => expression(target, input, variables, lists, arguments, active),
    }
}

// emit a typed placeholder that reports why this expression could not be lowered.
fn unsupported_expression(reason: &str) -> String {
    format!("ScratchValue::Number(todo!({}))", quote(reason))
}

// lower a boolean socket, using false when the Scratch socket is empty.
pub(super) fn boolean_input(
    target: &ScratchTarget,
    block: &ScratchBlock,
    name: &str,
    variables: &HashMap<String, VariableInfo>,
    lists: &HashMap<String, ListInfo>,
    arguments: &HashMap<String, String>,
    active: &mut HashSet<String>,
) -> String {
    match block.inputs.get(name).and_then(parse_input) {
        Some(InputValue::Empty) | None => "false".to_owned(),
        Some(InputValue::BlockReference(id))
            if target.blocks.get(id).is_some_and(|reporter| {
                matches!(
                    reporter.opcode.as_str(),
                    "operator_equals"
                        | "operator_gt"
                        | "operator_lt"
                        | "operator_and"
                        | "operator_or"
                        | "operator_not"
                        | "operator_contains"
                        | "data_listcontainsitem"
                )
            }) =>
        {
            expression(
                target,
                block.inputs.get(name),
                variables,
                lists,
                arguments,
                active,
            )
        }
        Some(_) => format!(
            "scratch_bool({})",
            expression(
                target,
                block.inputs.get(name),
                variables,
                lists,
                arguments,
                active
            )
        ),
    }
}
