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

use std::collections::{HashMap, HashSet};

use serde_json::Value;

use crate::parser::{InputValue, ScratchBlock, ScratchProject, ScratchTarget, parse_input};

/// Generate Rust source that uses aida-runtime's ScratchValue operator API.
pub fn generate(project: &ScratchProject) -> String {
    let variables = allocate_variables(project);
    let mut output = String::from(
        "#[allow(unused_imports)]\nuse crate::runtime::{\n    scratch_acos_degrees, scratch_add, scratch_and, scratch_asin_degrees, scratch_atan_degrees,\n    scratch_contains, scratch_cos, scratch_divide, scratch_equals, scratch_greater_than,\n    scratch_join, scratch_length, scratch_less_than, scratch_letter, scratch_mod, scratch_multiply,\n    scratch_not, scratch_or, scratch_random, scratch_round, scratch_sin, scratch_subtract,\n    scratch_tan, scratch_to_number, ScratchValue,\n};\n\n#[allow(unused_macros)]\nmacro_rules! print_value {\n    ($name:expr, $value:expr) => {\n        println!(\"{}: {:?}\", $name, $value);\n    };\n}\n\n#[allow(unused_mut, unused_variables, unused_assignments)]\npub fn run() {\n",
    );

    for target in &project.targets {
        let mut declarations: Vec<_> = target.variables.iter().collect();
        declarations.sort_by(|(left_id, left), (right_id, right)| {
            left.name.cmp(&right.name).then(left_id.cmp(right_id))
        });
        for (id, variable) in declarations {
            if let Some(name) = variables.get(id) {
                line(
                    &mut output,
                    1,
                    &format!(
                        "let mut {name}: ScratchValue = ({}).into();",
                        literal(&variable.value)
                    ),
                );
            }
        }
    }

    for target in &project.targets {
        let mut hats: Vec<_> = target
            .blocks
            .iter()
            .filter(|(_, block)| {
                block.top_level
                    && matches!(
                        block.opcode.as_str(),
                        "event_whenflagclicked"
                            | "event_whenthisspriteclicked"
                            | "event_whenstageclicked"
                            | "event_whenkeypressed"
                            | "event_whenbroadcastreceived"
                    )
            })
            .collect();
        hats.sort_by(|(left_id, _), (right_id, _)| left_id.cmp(right_id));
        for (_, hat) in hats {
            generate_statements(target, hat.next.as_deref(), &variables, &mut output);
        }
    }

    output.push_str("}\n");
    output
}

fn generate_statements(
    target: &ScratchTarget,
    first: Option<&str>,
    variables: &HashMap<String, String>,
    output: &mut String,
) {
    let mut current = first;
    let mut visited = HashSet::new();
    while let Some(id) = current {
        if !visited.insert(id.to_owned()) {
            line(
                output,
                1,
                &format!("todo!({});", quote(&format!("block cycle: {id}"))),
            );
            break;
        }
        let Some(block) = target.blocks.get(id) else {
            line(
                output,
                1,
                &format!("todo!({});", quote(&format!("missing block: {id}"))),
            );
            break;
        };
        match block.opcode.as_str() {
            "data_setvariableto" => {
                if let Some(name) = variable_field(block, "VARIABLE", variables) {
                    let value = expression(
                        target,
                        block.inputs.get("VALUE"),
                        variables,
                        &mut HashSet::new(),
                    );
                    line(output, 1, &format!("{name} = ({value}).into();"));
                } else {
                    line(output, 1, "todo!(\"missing variable reference\");");
                }
            }
            "data_changevariableby" => {
                if let Some(name) = variable_field(block, "VARIABLE", variables) {
                    let value = expression(
                        target,
                        block.inputs.get("VALUE"),
                        variables,
                        &mut HashSet::new(),
                    );
                    line(
                        output,
                        1,
                        &format!("{name} = scratch_add({name}.clone(), {value}).into();"),
                    );
                } else {
                    line(output, 1, "todo!(\"missing variable reference\");");
                }
            }
            "looks_say" => {
                let value = expression(
                    target,
                    block.inputs.get("MESSAGE"),
                    variables,
                    &mut HashSet::new(),
                );
                let name = say_variable_name(target, block.inputs.get("MESSAGE"))
                    .unwrap_or_else(|| "say".to_owned());
                let label = quote(&name);
                line(output, 1, &format!("print_value!({label}, {value});"));
            }
            opcode => line(
                output,
                1,
                &format!(
                    "todo!({});",
                    quote(&format!("unsupported statement: {opcode}"))
                ),
            ),
        }
        current = block.next.as_deref();
    }
}

fn expression(
    target: &ScratchTarget,
    input: Option<&Value>,
    variables: &HashMap<String, String>,
    active: &mut HashSet<String>,
) -> String {
    match input.and_then(parse_input) {
        Some(InputValue::Number(value)) => number_literal(value),
        Some(InputValue::String(value)) => text_literal(value),
        Some(InputValue::VariableReference(reference)) => variables
            .get(reference.id)
            .map(|name| format!("{name}.clone()"))
            .unwrap_or_else(|| unsupported_expression("unknown variable")),
        Some(InputValue::BlockReference(id)) => {
            if !active.insert(id.to_owned()) {
                return unsupported_expression(&format!("expression cycle: {id}"));
            }
            let result = target
                .blocks
                .get(id)
                .map(|block| block_expression(target, block, variables, active))
                .unwrap_or_else(|| unsupported_expression(&format!("missing block: {id}")));
            active.remove(id);
            result
        }
        Some(InputValue::Empty) => unsupported_expression("empty input"),
        Some(InputValue::ListReference(_)) => {
            unsupported_expression("list values are not implemented")
        }
        None => unsupported_expression("missing or invalid input"),
    }
}

fn block_expression(
    target: &ScratchTarget,
    block: &ScratchBlock,
    variables: &HashMap<String, String>,
    active: &mut HashSet<String>,
) -> String {
    let input = |name: &str, active: &mut HashSet<String>| {
        expression(target, block.inputs.get(name), variables, active)
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
        "operator_equals" => binary("scratch_equals", "OPERAND1", "OPERAND2", active),
        "operator_gt" => binary("scratch_greater_than", "OPERAND1", "OPERAND2", active),
        "operator_lt" => binary("scratch_less_than", "OPERAND1", "OPERAND2", active),
        "operator_and" => format!(
            "scratch_and({}, {})",
            boolean_input(target, block, "OPERAND1", variables, active),
            boolean_input(target, block, "OPERAND2", variables, active)
        ),
        "operator_or" => format!(
            "scratch_or({}, {})",
            boolean_input(target, block, "OPERAND1", variables, active),
            boolean_input(target, block, "OPERAND2", variables, active)
        ),
        "operator_join" => binary("scratch_join", "STRING1", "STRING2", active),
        "operator_letter_of" => binary("scratch_letter", "LETTER", "STRING", active),
        "operator_length" => format!("scratch_length({})", input("STRING", active)),
        "operator_contains" => binary("scratch_contains", "STRING1", "STRING2", active),
        "operator_round" => format!("scratch_round({})", input("NUM", active)),
        "operator_not" => format!(
            "scratch_not({})",
            boolean_input(target, block, "OPERAND", variables, active)
        ),
        "operator_random" => format!(
            "scratch_random({}, {})",
            random_argument(target, block.inputs.get("FROM"), variables, active),
            random_argument(target, block.inputs.get("TO"), variables, active)
        ),
        "operator_mathop" => math_expression(target, block, variables, active),
        "data_variable" => variable_field(block, "VARIABLE", variables)
            .map(|name| format!("{name}.clone()"))
            .unwrap_or_else(|| unsupported_expression("missing variable reference")),
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

fn math_expression(
    target: &ScratchTarget,
    block: &ScratchBlock,
    variables: &HashMap<String, String>,
    active: &mut HashSet<String>,
) -> String {
    let value = expression(target, block.inputs.get("NUM"), variables, active);
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

// Scratch's random reporter distinguishes integer and decimal literal text.
// Keep the original text for number inputs so `10` and `10.0` reach the
// runtime as distinct Scratch values.
fn random_argument(
    target: &ScratchTarget,
    input: Option<&Value>,
    variables: &HashMap<String, String>,
    active: &mut HashSet<String>,
) -> String {
    match input.and_then(parse_input) {
        Some(InputValue::Number(value)) => value
            .as_str()
            .map(|text| quote(text))
            .or_else(|| value.as_number().map(|number| quote(&number.to_string())))
            .unwrap_or_else(|| expression(target, input, variables, active)),
        Some(InputValue::BlockReference(id)) => {
            let Some(block) = target.blocks.get(id) else {
                return expression(target, input, variables, active);
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
            expression(target, input, variables, active)
        }
        _ => expression(target, input, variables, active),
    }
}

fn unsupported_expression(reason: &str) -> String {
    format!("ScratchValue::Number(todo!({}))", quote(reason))
}

fn boolean_input(
    target: &ScratchTarget,
    block: &ScratchBlock,
    name: &str,
    variables: &HashMap<String, String>,
    active: &mut HashSet<String>,
) -> String {
    match block.inputs.get(name).and_then(parse_input) {
        Some(InputValue::Empty) | None => "false".to_owned(),
        Some(_) => expression(target, block.inputs.get(name), variables, active),
    }
}

fn allocate_variables(project: &ScratchProject) -> HashMap<String, String> {
    let mut entries: Vec<_> = project
        .targets
        .iter()
        .flat_map(|target| {
            target
                .variables
                .iter()
                .map(|(id, variable)| (id.as_str(), variable.name.as_str()))
        })
        .collect();
    entries.sort_unstable();
    let mut used = HashSet::new();
    let mut output = HashMap::new();
    for (id, name) in entries {
        let base = rust_identifier(name);
        let mut candidate = base.clone();
        let mut suffix = 2;
        while !used.insert(candidate.clone()) {
            candidate = format!("{base}_{suffix}");
            suffix += 1;
        }
        output.insert(id.to_owned(), candidate);
    }
    output
}

fn rust_identifier(name: &str) -> String {
    let mut output = String::new();
    for (index, character) in name.chars().enumerate() {
        if character.is_ascii_alphanumeric() || character == '_' {
            if index == 0 && character.is_ascii_digit() {
                output.push('_');
            }
            output.push(character);
        } else if !output.ends_with('_') {
            output.push('_');
        }
    }
    let output = output.trim_matches('_');
    let output = if output.is_empty() {
        "variable"
    } else {
        output
    };
    let output = if output.as_bytes()[0].is_ascii_digit() {
        format!("_{output}")
    } else {
        output.to_owned()
    };
    if is_keyword(&output) {
        format!("_{output}")
    } else {
        output
    }
}

fn is_keyword(name: &str) -> bool {
    matches!(
        name,
        "as" | "break"
            | "const"
            | "continue"
            | "crate"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "Self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
            | "async"
            | "await"
            | "dyn"
    )
}

fn variable_field(
    block: &ScratchBlock,
    field: &str,
    variables: &HashMap<String, String>,
) -> Option<String> {
    let data = block.fields.get(field)?.as_array()?;
    variables.get(data.get(1)?.as_str()?).cloned()
}

fn say_variable_name(target: &ScratchTarget, input: Option<&Value>) -> Option<String> {
    match input.and_then(parse_input)? {
        InputValue::VariableReference(reference) => Some(reference.name.to_owned()),
        InputValue::BlockReference(id) => {
            let reporter = target.blocks.get(id)?;
            if reporter.opcode != "data_variable" {
                return None;
            }
            field_value(reporter, "VARIABLE")?
                .as_str()
                .map(str::to_owned)
        }
        _ => None,
    }
}

fn field_value<'a>(block: &'a ScratchBlock, field: &str) -> Option<&'a Value> {
    block.fields.get(field)?.as_array()?.first()
}

fn literal(value: &Value) -> String {
    match value {
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value
            .as_f64()
            .map(number_literal_f64)
            .unwrap_or_else(|| "0.0_f64".to_owned()),
        Value::String(value) => quote(value),
        _ => "0.0_f64".to_owned(),
    }
}

fn number_literal(value: &Value) -> String {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|text| text.parse::<f64>().ok()))
        .map(number_literal_f64)
        .unwrap_or_else(|| "0.0_f64".to_owned())
}

fn number_literal_f64(value: f64) -> String {
    format!("{value:?}_f64")
}

fn text_literal(value: &Value) -> String {
    value.as_str().map(quote).unwrap_or_else(|| quote(""))
}

fn quote(value: &str) -> String {
    format!("{value:?}")
}

fn line(output: &mut String, indent: usize, text: &str) {
    output.push_str(&"    ".repeat(indent));
    output.push_str(text);
    output.push('\n');
}

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
