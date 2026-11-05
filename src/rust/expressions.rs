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
            .map(VariableInfo::read_expression)
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
    // A list selected for cached numeric reads still owns its original dynamic
    // values; the runtime invalidates the cache when the list changes.
    let numeric_input = |name: &str, active: &mut HashSet<String>| {
        let candidate = block.inputs.get(name).and_then(parse_input).and_then(|input| match input {
            InputValue::BlockReference(id) => target.blocks.get(id),
            _ => None,
        }).filter(|reporter| reporter.opcode == "data_itemoflist");
        if let Some(reporter) = candidate {
            if let Some(list) = list_field(reporter, "LIST", lists).filter(|list| list.cache_numeric_reads) {
                let index = expression(target, reporter.inputs.get("INDEX"), variables, lists, arguments, active);
                return format!("state.list({}).get_cached_number({index})", list.slot);
            }
        }
        input(name, active)
    };
    let numeric_binary = |function: &str, active: &mut HashSet<String>| {
        format!("{function}({}, {})", numeric_input("NUM1", active), numeric_input("NUM2", active))
    };
    // Canonical small integer text has exactly the same Scratch comparison
    // text as its numeric value, including when the other operand falls back
    // to a string comparison. Emit a number to avoid parsing it on every test.
    let comparison = |function: &str, active: &mut HashSet<String>| {
        let operand = |name: &str, active: &mut HashSet<String>| {
            let value = block.inputs.get(name);
            let other = if name == "OPERAND1" { "OPERAND2" } else { "OPERAND1" };
            let both_literals = matches!(block.inputs.get(other).and_then(parse_input), Some(InputValue::String(_) | InputValue::Number(_)));
            if let Some(InputValue::String(value)) = value.and_then(parse_input).filter(|_| !both_literals) {
                if let Some(text) = value.as_str() {
                    if let Ok(number) = text.parse::<i32>() {
                        if number.unsigned_abs() <= 1_000_000 && number.to_string() == text {
                            return format!("{number}.0_f64");
                        }
                    }
                }
            }
            input(name, active)
        };
        let left = operand("OPERAND1", active);
        let right = operand("OPERAND2", active);
        let both_literals = ["OPERAND1", "OPERAND2"].iter().all(|name| matches!(block.inputs.get(*name).and_then(parse_input), Some(InputValue::String(_) | InputValue::Number(_))));
        if !both_literals && comparison_input_is_numeric(target, block.inputs.get("OPERAND1"))
            && comparison_input_is_numeric(target, block.inputs.get("OPERAND2"))
        {
            let op = match function { "scratch_equals" => "==", "scratch_greater_than" => ">", "scratch_less_than" => "<", _ => unreachable!() };
            format!("{{ let left = {left}; let right = {right}; if left.is_nan() || right.is_nan() {{ {function}(left, right) }} else {{ left {op} right }} }}")
        } else {
            format!("{function}({left}, {right})")
        }
    };
    match block.opcode.as_str() {
        "operator_add" => numeric_binary("scratch_add", active),
        "operator_subtract" => numeric_binary("scratch_subtract", active),
        "operator_multiply" => numeric_binary("scratch_multiply", active),
        "operator_divide" => numeric_binary("scratch_divide", active),
        "operator_mod" => binary("scratch_mod", "NUM1", "NUM2", active),
        "operator_equals" if identical_literal_inputs(block) => "true".to_owned(),
        "operator_equals" => numeric_list_equality(target, block, variables, lists)
            .unwrap_or_else(|| comparison("scratch_equals", active)),
        "operator_gt" => comparison("scratch_greater_than", active),
        "operator_lt" => comparison("scratch_less_than", active),
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
        "operator_random" => random_expression(target, block, variables, lists, arguments, active),
        "operator_mathop" => math_expression(target, block, variables, lists, arguments, active),
        "data_variable" => variable_field(block, "VARIABLE", variables)
            .map(|variable| variable.read_expression())
            .unwrap_or_else(|| unsupported_expression("missing variable reference")),
        "sound_volume" => "state.sound_volume()".to_owned(),
        "sound_sounds_menu" => field_value(block, "SOUND_MENU")
            .map(text_literal)
            .unwrap_or_else(|| unsupported_expression("missing sound menu value")),
        "data_listcontents" => list_field(block, "LIST", lists)
            .map(|list| format!("state.list({}).contents()", list.slot))
            .unwrap_or_else(|| unsupported_expression("missing list reference")),
        "looks_backdrops" => field_value(block, "BACKDROP")
            .map(text_literal)
            .unwrap_or_else(|| unsupported_expression("missing backdrop menu value")),
        // read each motion reporter from the instance selected by the scheduler.
        "motion_xposition" => "state.x_position()".to_owned(),
        "motion_yposition" => "state.y_position()".to_owned(),
        // report the direction stored for the currently scheduled sprite or clone.
        "motion_direction" => "state.direction()".to_owned(),
        "looks_costume" => field_value(block, "COSTUME")
            .map(text_literal)
            .unwrap_or_else(|| unsupported_expression("missing costume menu value")),
        "pen_menu_colorParam" => field_value(block, "colorParam")
            .map(text_literal)
            .unwrap_or_else(|| unsupported_expression("missing pen color parameter")),
        "looks_costumenumbername" => {
            let number = field_value(block, "NUMBER_NAME")
                .and_then(Value::as_str)
                .is_some_and(|value| value == "number");
            format!("state.costume_number_name({number})")
        }
        "looks_backdropnumbername" => {
            let number = field_value(block, "NUMBER_NAME")
                .and_then(Value::as_str)
                .is_some_and(|value| value == "number");
            format!("state.backdrop_number_name({number})")
        }
        "data_itemoflist" => list_field(block, "LIST", lists)
            .map(|list| format!("state.list({}).get({})", list.slot, input("INDEX", active)))
            .unwrap_or_else(|| unsupported_expression("missing list reference")),
        // forward dropdown selectors unchanged; list operations resolve them when used.
        "data_listindexall" | "data_listindexrandom" => field_value(block, "INDEX")
            .and_then(Value::as_str)
            .map(quote)
            .unwrap_or_else(|| quote("1")),
        "data_lengthoflist" => list_field(block, "LIST", lists)
            .map(|list| format!("state.list({}).length() as f64", list.slot))
            .unwrap_or_else(|| unsupported_expression("missing list reference")),
        "event_broadcast_menu" => block
            .fields
            .get("BROADCAST_OPTION")
            .and_then(Value::as_array)
            .and_then(|field| {
                let name = field.first()?.as_str()?;
                Some(
                    field
                        .get(1)
                        .and_then(Value::as_str)
                        .and_then(|id| target.broadcasts.get(id))
                        .map(String::as_str)
                        .unwrap_or(name),
                )
            })
            .map(quote)
            .unwrap_or_else(|| unsupported_expression("missing broadcast message")),
        "data_itemnumoflist" => list_field(block, "LIST", lists)
            .map(|list| {
                format!(
                    "state.list({}).item_number({}) as f64",
                    list.slot,
                    input("ITEM", active)
                )
            })
            .unwrap_or_else(|| unsupported_expression("missing list reference")),
        // evaluate the current connected reporter, falling back to legacy field data only when absent.
        "sensing_keypressed" => block
            .inputs
            .get("KEY_OPTION")
            .map(|_| format!("state.key_pressed({})", input("KEY_OPTION", active)))
            .or_else(|| {
                field_value(block, "KEY_OPTION")
                    .and_then(Value::as_str)
                    .map(|key| format!("state.key_pressed({})", quote(key)))
            })
            .unwrap_or_else(|| "state.key_pressed(\"\")".to_owned()),
        // preserve the selected serialized dropdown value for reporter and hat inputs.
        "sensing_keyoptions" => field_value(block, "KEY_OPTION")
            .and_then(Value::as_str)
            .map(quote)
            .unwrap_or_else(|| quote("space")),
        "sensing_mousedown" => "state.mouse_down()".to_owned(),
        "sensing_mousex" => "state.mouse_x()".to_owned(),
        "sensing_mousey" => "state.mouse_y()".to_owned(),
        "sensing_timer" => "state.project_timer_seconds(clock.now_millis())".to_owned(),
        "sensing_current" => current_expression(block),
        "sensing_dayssince2000" => "state.days_since_2000()".to_owned(),
        "sensing_online" => "false".to_owned(),
        "sensing_username" => "state.username()".to_owned(),
        "sensing_loudness" => "state.loudness()".to_owned(),
        // legacy vm reporters have no primitive result, so preserve scratch's undefined value.
        "motion_xscroll" | "motion_yscroll" | "sensing_userid" => {
            "ScratchValue::Undefined".to_owned()
        }
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

// Numeric comparison operands can use native operators except for NaN,
// where Scratch deliberately falls back to text ordering.
fn comparison_input_is_numeric(target: &ScratchTarget, input: Option<&Value>) -> bool {
    match input.and_then(parse_input) {
        Some(InputValue::Number(_)) => true,
        Some(InputValue::String(value)) => value.as_str().and_then(|text| text.parse::<i32>().ok().map(|n| (text, n))).is_some_and(|(text, n)| n.unsigned_abs() <= 1_000_000 && n.to_string() == text),
        Some(InputValue::BlockReference(id)) => target.blocks.get(id).is_some_and(|block| matches!(block.opcode.as_str(), "operator_add" | "operator_subtract" | "operator_multiply" | "operator_divide" | "operator_mod" | "operator_round" | "data_lengthoflist" | "data_itemnumoflist" | "motion_xposition" | "motion_yposition" | "motion_direction" | "sound_volume" | "sensing_mousex" | "sensing_mousey" | "sensing_timer" | "sensing_dayssince2000" | "sensing_loudness")),
        _ => false,
    }
}

// specialize direct numeric list comparisons while retaining dynamic fallbacks.
fn numeric_list_equality(
    target: &ScratchTarget,
    block: &ScratchBlock,
    variables: &HashMap<String, VariableInfo>,
    lists: &HashMap<String, ListInfo>,
) -> Option<String> {
    for (list_input, other_input) in [("OPERAND1", "OPERAND2"), ("OPERAND2", "OPERAND1")] {
        let InputValue::BlockReference(id) = parse_input(block.inputs.get(list_input)?)? else {
            continue;
        };
        let reporter = target.blocks.get(id)?;
        if reporter.opcode != "data_itemoflist" {
            continue;
        }
        let list = list_field(reporter, "LIST", lists)?;
        let index = direct_variable_input(target, reporter.inputs.get("INDEX"), variables)?;
        let other = direct_variable_input(target, block.inputs.get(other_input), variables)?;
        if !index.numeric || !other.numeric {
            continue;
        }
        let index_expression = index.read_expression();
        let other_expression = other.read_expression();
        return Some(format!(
            "{{ let scratch_numeric_list_match = match ({index_expression}, {other_expression}) {{ (ScratchValue::Number(scratch_index), ScratchValue::Number(scratch_value)) => state.list({}).equals_number(*scratch_index, *scratch_value), _ => None }}; scratch_numeric_list_match.unwrap_or_else(|| scratch_equals(state.list({}).get({index_expression}), {other_expression})) }}",
            list.slot, list.slot
        ));
    }
    None
}

fn direct_variable_input(
    target: &ScratchTarget,
    input: Option<&Value>,
    variables: &HashMap<String, VariableInfo>,
) -> Option<VariableInfo> {
    match input.and_then(parse_input)? {
        InputValue::VariableReference(reference) => variables.get(reference.id).cloned(),
        InputValue::BlockReference(id) => {
            let reporter = target.blocks.get(id)?;
            if reporter.opcode == "data_variable" {
                variable_field(reporter, "VARIABLE", variables)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn current_expression(block: &ScratchBlock) -> String {
    block
        .fields
        .get("CURRENTMENU")
        .and_then(Value::as_array)
        .and_then(|field| field.first())
        .and_then(Value::as_str)
        .and_then(current_method)
        .map_or_else(|| "0.0".to_owned(), |method| format!("state.{method}()"))
}

fn current_method(menu: &str) -> Option<&'static str> {
    match menu.to_ascii_uppercase().as_str() {
        "YEAR" => Some("current_year"),
        "MONTH" => Some("current_month"),
        "DATE" => Some("current_date"),
        "DAYOFWEEK" => Some("current_day_of_week"),
        "HOUR" => Some("current_hour"),
        "MINUTE" => Some("current_minute"),
        "SECOND" => Some("current_second"),
        _ => None,
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

// evaluate constant arithmetic using scratch's numeric input conversion.
pub(super) fn constant_numeric_input(
    target: &ScratchTarget,
    input: Option<&Value>,
    active: &mut HashSet<String>,
) -> Option<f64> {
    let normalize = |value: f64| if value.is_nan() { 0.0 } else { value };
    match input.and_then(parse_input)? {
        InputValue::Number(value) => Some(normalize(
            value
                .as_f64()
                .or_else(|| value.as_str().and_then(|text| text.parse().ok()))
                .unwrap_or(0.0),
        )),
        InputValue::String(value) => Some(normalize(
            value
                .as_str()
                .map(str::trim)
                .filter(|text| !text.is_empty())
                .and_then(|text| text.parse().ok())
                .unwrap_or(0.0),
        )),
        InputValue::BlockReference(id) => {
            if !active.insert(id.to_owned()) {
                return None;
            }
            let block = target.blocks.get(id)?;
            let result = if block.opcode == "operator_equals" && identical_literal_inputs(block) {
                Some(1.0)
            } else {
                let names = match block.opcode.as_str() {
                    "operator_add" | "operator_subtract" => ("NUM1", "NUM2"),
                    "operator_multiply" | "operator_divide" => ("NUM1", "NUM2"),
                    _ => {
                        active.remove(id);
                        return None;
                    }
                };
                let left = normalize(constant_numeric_input(
                    target,
                    block.inputs.get(names.0),
                    active,
                )?);
                let right = normalize(constant_numeric_input(
                    target,
                    block.inputs.get(names.1),
                    active,
                )?);
                Some(match block.opcode.as_str() {
                    "operator_add" => left + right,
                    "operator_subtract" => left - right,
                    "operator_multiply" => left * right,
                    "operator_divide" => left / right,
                    _ => unreachable!(),
                })
            };
            active.remove(id);
            result
        }
        _ => None,
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
    fast_math: bool,
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
            let result = target.blocks.get(id).and_then(|block| {
                local_block_expression(target, block, variables, locals, fast_math, active)
            });
            active.remove(id);
            result
        }
        _ => None,
    }
}

pub(super) fn expression_argument(expression: &str) -> &str {
    let bytes = expression.as_bytes();
    if bytes.first() != Some(&b'(') || bytes.last() != Some(&b')') {
        return expression;
    }

    let mut depth = 0;
    let mut in_string = false;
    let mut escaped = false;
    for (index, byte) in bytes.iter().copied().enumerate() {
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
            continue;
        }

        match byte {
            b'"' => in_string = true,
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 && index + 1 != bytes.len() {
                    return expression;
                }
            }
            _ => {}
        }
    }

    if depth == 0 {
        &expression[1..expression.len() - 1]
    } else {
        expression
    }
}

fn local_block_expression(
    target: &ScratchTarget,
    block: &ScratchBlock,
    variables: &HashMap<String, VariableInfo>,
    locals: &HashMap<usize, String>,
    fast_math: bool,
    active: &mut HashSet<String>,
) -> Option<LocalExpression> {
    let input = |name: &str, active: &mut HashSet<String>| {
        local_expression(
            target,
            block.inputs.get(name),
            variables,
            locals,
            fast_math,
            active,
        )
    };
    let numeric_binary =
        |function: &str, operator: &str, left: &str, right: &str, active: &mut HashSet<String>| {
            let left = input(left, active)?;
            let right = input(right, active)?;
            let native =
                left.kind == LocalValueKind::Number && right.kind == LocalValueKind::Number;
            let code = if native {
                let left = if fast_math {
                    left.code
                } else {
                    format!(
                        "scratch_number_operand({})",
                        expression_argument(&left.code)
                    )
                };
                let right = if fast_math {
                    right.code
                } else {
                    format!(
                        "scratch_number_operand({})",
                        expression_argument(&right.code)
                    )
                };
                format!("({left} {operator} {right})")
            } else {
                format!(
                    "{function}({}, {})",
                    expression_argument(&left.code),
                    expression_argument(&right.code)
                )
            };
            Some(LocalExpression {
                code,
                kind: LocalValueKind::Number,
            })
        };
    let comparison = |function: &str,
                      operator: &str,
                      left_name: &str,
                      right_name: &str,
                      active: &mut HashSet<String>| {
        let left = input(left_name, active)?;
        let right = input(right_name, active)?;
        let native = left.kind == LocalValueKind::Number
            && right.kind == LocalValueKind::Number
            && local_input_is_non_nan_number(
                target,
                block.inputs.get(left_name),
                variables,
                &mut HashSet::new(),
            )
            && local_input_is_non_nan_number(
                target,
                block.inputs.get(right_name),
                variables,
                &mut HashSet::new(),
            );
        Some(LocalExpression {
            code: if native {
                format!("{} {operator} {}", left.code, right.code)
            } else {
                format!("{function}({}, {})", left.code, right.code)
            },
            kind: LocalValueKind::Boolean,
        })
    };

    match block.opcode.as_str() {
        "operator_add" => numeric_binary("scratch_add", "+", "NUM1", "NUM2", active),
        "operator_subtract" => numeric_binary("scratch_subtract", "-", "NUM1", "NUM2", active),
        "operator_multiply" => numeric_binary("scratch_multiply", "*", "NUM1", "NUM2", active),
        "operator_divide" => numeric_binary("scratch_divide", "/", "NUM1", "NUM2", active),
        "operator_mod" => {
            let left = input("NUM1", active)?;
            let right = input("NUM2", active)?;
            Some(LocalExpression {
                code: format!("scratch_mod({}, {})", left.code, right.code),
                kind: LocalValueKind::Number,
            })
        }
        "operator_equals" if identical_literal_inputs(block) => Some(LocalExpression {
            code: "true".to_owned(),
            kind: LocalValueKind::Boolean,
        }),
        "operator_equals" => comparison("scratch_equals", "==", "OPERAND1", "OPERAND2", active),
        "operator_gt" => comparison("scratch_greater_than", ">", "OPERAND1", "OPERAND2", active),
        "operator_lt" => comparison("scratch_less_than", "<", "OPERAND1", "OPERAND2", active),
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
            let numeric = if value.kind == LocalValueKind::Number {
                if fast_math {
                    value.code.clone()
                } else {
                    format!(
                        "scratch_number_operand({})",
                        expression_argument(&value.code)
                    )
                }
            } else {
                format!("scratch_to_number({})", expression_argument(&value.code))
            };
            let operator = field_value(block, "OPERATOR")?.as_str()?;
            let code = match operator {
                "sin" => format!(
                    "{}({})",
                    if fast_math { "fast_sin" } else { "scratch_sin" },
                    expression_argument(&value.code)
                ),
                "cos" => format!(
                    "{}({})",
                    if fast_math { "fast_cos" } else { "scratch_cos" },
                    expression_argument(&value.code)
                ),
                "tan" => format!(
                    "{}({})",
                    if fast_math { "fast_tan" } else { "scratch_tan" },
                    expression_argument(&value.code)
                ),
                "asin" => format!(
                    "{}({})",
                    if fast_math {
                        "fast_asin_degrees"
                    } else {
                        "scratch_asin_degrees"
                    },
                    expression_argument(&value.code)
                ),
                "acos" => format!(
                    "{}({})",
                    if fast_math {
                        "fast_acos_degrees"
                    } else {
                        "scratch_acos_degrees"
                    },
                    expression_argument(&value.code)
                ),
                "atan" => format!(
                    "{}({})",
                    if fast_math {
                        "fast_atan_degrees"
                    } else {
                        "scratch_atan_degrees"
                    },
                    expression_argument(&value.code)
                ),
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

fn local_input_is_non_nan_number(
    target: &ScratchTarget,
    input: Option<&Value>,
    variables: &HashMap<String, VariableInfo>,
    active: &mut HashSet<String>,
) -> bool {
    match input.and_then(parse_input) {
        Some(InputValue::Number(value)) => value
            .as_f64()
            .or_else(|| value.as_str().and_then(|text| text.parse().ok()))
            .is_some_and(|value: f64| !value.is_nan()),
        Some(InputValue::VariableReference(reference)) => variables
            .get(reference.id)
            .is_some_and(|variable| variable.non_nan),
        Some(InputValue::BlockReference(id)) => {
            if !active.insert(id.to_owned()) {
                return false;
            }
            let result = target.blocks.get(id).is_some_and(|block| {
                if block.opcode == "data_variable" {
                    return variable_field(block, "VARIABLE", variables)
                        .is_some_and(|variable| variable.non_nan);
                }
                matches!(
                    block.opcode.as_str(),
                    "math_number"
                        | "math_integer"
                        | "math_whole_number"
                        | "math_positive_number"
                        | "math_angle"
                ) && field_value(block, "NUM")
                    .and_then(|value| {
                        value
                            .as_f64()
                            .or_else(|| value.as_str().and_then(|text| text.parse().ok()))
                    })
                    .is_some_and(|value: f64| !value.is_nan())
            });
            active.remove(id);
            result
        }
        _ => false,
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
// lower fixed random bounds once while preserving scratch's integer-range decision.
fn random_expression(
    target: &ScratchTarget,
    block: &ScratchBlock,
    variables: &HashMap<String, VariableInfo>,
    lists: &HashMap<String, ListInfo>,
    arguments: &HashMap<String, String>,
    active: &mut HashSet<String>,
) -> String {
    if let Some((from, to, integer_bounds)) = constant_random_bounds(target, block) {
        return format!(
            "scratch_random_constant({from:?}_f64, {to:?}_f64, {})",
            integer_bounds
        );
    }
    format!(
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
    )
}

pub(super) fn constant_random_bounds(
    target: &ScratchTarget,
    block: &ScratchBlock,
) -> Option<(f64, f64, bool)> {
    if block.opcode != "operator_random" {
        return None;
    }
    let (from, from_integer) = constant_random_bound(target, block.inputs.get("FROM"))?;
    let (to, to_integer) = constant_random_bound(target, block.inputs.get("TO"))?;
    Some((from, to, from_integer && to_integer))
}

fn constant_random_bound(target: &ScratchTarget, input: Option<&Value>) -> Option<(f64, bool)> {
    let spelling = match input.and_then(parse_input)? {
        InputValue::Number(value) => random_literal_spelling(&value)?,
        InputValue::BlockReference(id) => {
            let block = target.blocks.get(id)?;
            let field = match block.opcode.as_str() {
                "math_number"
                | "math_integer"
                | "math_whole_number"
                | "math_positive_number"
                | "math_angle" => "NUM",
                "text" => "TEXT",
                _ => return None,
            };
            random_literal_spelling(field_value(block, field)?)?
        }
        _ => return None,
    };
    let text = spelling.trim();
    let number = if text.is_empty() {
        0.0
    } else {
        text.parse::<f64>().unwrap_or(0.0)
    };
    let number = if number.is_nan() { 0.0 } else { number };
    number
        .is_finite()
        .then_some((number, !spelling.contains('.')))
}

fn random_literal_spelling(value: &Value) -> Option<String> {
    value
        .as_str()
        .map(str::to_owned)
        .or_else(|| value.as_number().map(|number| number.to_string()))
}

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

#[cfg(test)]
mod tests {
    use super::expression;
    use crate::parser::ScratchTarget;
    use serde_json::json;
    use std::collections::{HashMap, HashSet};

    #[test]
    fn comparison_only_normalizes_canonical_integer_text() {
        let mut target: ScratchTarget = serde_json::from_value(json!({
            "name": "Stage", "isStage": true, "blocks": {
                "sensor": {"opcode": "sensing_loudness"},
                "compare": {"opcode": "operator_gt", "inputs": {"OPERAND1": [2, "sensor"], "OPERAND2": [1, [10, "240"]]}}
            }
        })).unwrap();
        let root = json!([2, "compare"]);
        let generate = |target: &ScratchTarget| expression(target, Some(&root), &HashMap::new(), &HashMap::new(), &HashMap::new(), &mut HashSet::new());
        let generated = generate(&target);
        assert!(generated.contains("let left = state.loudness()"));
        assert!(generated.contains("let right = 240.0_f64"));
        assert!(generated.contains("scratch_greater_than(left, right)"));
        for value in ["0240", "-0", " 240", "240.0", "1000001"] {
            target.blocks.get_mut("compare").unwrap().inputs.insert("OPERAND2".into(), json!([1, [10, value]]));
            assert!(generate(&target).contains(&format!("{value:?}")));
        }
    }

    #[test]
    fn sensing_reporter_inside_nested_operator_uses_the_shared_input_evaluator() {
        let target: ScratchTarget = serde_json::from_value(json!({
            "name": "Stage",
            "isStage": true,
            "blocks": {
                "loudness": { "opcode": "sensing_loudness" },
                "join": {
                    "opcode": "operator_join",
                    "inputs": {
                        "STRING1": [3, "loudness", [10, "fallback"]],
                        "STRING2": [1, [10, " dB"]]
                    }
                }
            }
        }))
        .unwrap();
        let root = json!([3, "join"]);
        let generated = expression(
            &target,
            Some(&root),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &mut HashSet::new(),
        );

        assert!(generated.contains("state.loudness()"));
        assert!(generated.starts_with("scratch_join("));
        assert!(generated.contains("dB"));
        assert!(!generated.contains("fallback"));
    }

    #[test]
    fn project_sensing_reporters_lower_inside_join_and_comparison_inputs() {
        let mut target: ScratchTarget = serde_json::from_value(json!({
            "name": "Stage",
            "isStage": true,
            "blocks": {
                "current": {
                    "opcode": "sensing_current",
                    "fields": { "CURRENTMENU": ["DAYOFWEEK", null] }
                },
                "days": { "opcode": "sensing_dayssince2000" },
                "online": { "opcode": "sensing_online" },
                "username": { "opcode": "sensing_username" },
                "mousex": { "opcode": "sensing_mousex" },
                "mousey": { "opcode": "sensing_mousey" },
                "mousedown": { "opcode": "sensing_mousedown" },
                "timer": { "opcode": "sensing_timer" },
                "loudness": { "opcode": "sensing_loudness" },
                "consumer": { "opcode": "operator_join" }
            }
        }))
        .unwrap();
        let cases = [
            ("current", "state.current_day_of_week()"),
            ("days", "state.days_since_2000()"),
            ("online", "false"),
            ("username", "state.username()"),
            ("mousex", "state.mouse_x()"),
            ("mousey", "state.mouse_y()"),
            ("mousedown", "state.mouse_down()"),
            ("timer", "state.project_timer_seconds(clock.now_millis())"),
            ("loudness", "state.loudness()"),
        ];

        for (sensor, expected) in cases {
            let consumer = target.blocks.get_mut("consumer").unwrap();
            consumer.opcode = "operator_join".to_owned();
            consumer.inputs = HashMap::from([
                ("STRING1".to_owned(), json!([3, sensor])),
                ("STRING2".to_owned(), json!([1, [10, ""]])),
            ]);
            let join = expression(
                &target,
                Some(&json!([3, "consumer"])),
                &HashMap::new(),
                &HashMap::new(),
                &HashMap::new(),
                &mut HashSet::new(),
            );
            assert!(join.starts_with("scratch_join("));
            assert!(join.contains(expected), "{sensor} missing in {join}");

            let consumer = target.blocks.get_mut("consumer").unwrap();
            consumer.opcode = "operator_equals".to_owned();
            consumer.inputs = HashMap::from([
                ("OPERAND1".to_owned(), json!([3, sensor])),
                ("OPERAND2".to_owned(), json!([1, [10, ""]])),
            ]);
            let comparison = expression(
                &target,
                Some(&json!([3, "consumer"])),
                &HashMap::new(),
                &HashMap::new(),
                &HashMap::new(),
                &mut HashSet::new(),
            );
            assert!(comparison.starts_with("scratch_equals("));
            assert!(
                comparison.contains(expected),
                "{sensor} missing in {comparison}"
            );
        }
    }

    // preserve every selector choice and scratch's default before a list consumer interprets it.
    #[test]
    fn list_index_dropdowns_keep_scratch_values_and_default_to_one() {
        let cases = [
            ("data_listindexall", "1", "1"),
            ("data_listindexall", "last", "last"),
            ("data_listindexall", "all", "all"),
            ("data_listindexrandom", "1", "1"),
            ("data_listindexrandom", "last", "last"),
            ("data_listindexrandom", "random", "random"),
        ];

        for (opcode, selected, expected) in cases {
            let target: ScratchTarget = serde_json::from_value(json!({
                "name": "Stage",
                "isStage": true,
                "blocks": {
                    "selector": {
                        "opcode": opcode,
                        "fields": { "INDEX": [selected, null] }
                    }
                }
            }))
            .unwrap();
            let generated = expression(
                &target,
                Some(&json!([3, "selector"])),
                &HashMap::new(),
                &HashMap::new(),
                &HashMap::new(),
                &mut HashSet::new(),
            );

            assert_eq!(generated, format!("{expected:?}"), "{opcode}: {selected}");
        }

        for opcode in ["data_listindexall", "data_listindexrandom"] {
            let target: ScratchTarget = serde_json::from_value(json!({
                "name": "Stage",
                "isStage": true,
                "blocks": { "selector": { "opcode": opcode } }
            }))
            .unwrap();
            let generated = expression(
                &target,
                Some(&json!([3, "selector"])),
                &HashMap::new(),
                &HashMap::new(),
                &HashMap::new(),
                &mut HashSet::new(),
            );

            assert_eq!(generated, "\"1\"");
        }
    }
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
