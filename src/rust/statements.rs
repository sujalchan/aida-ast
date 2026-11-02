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

//! traverses Scratch statement chains and emits their Rust equivalents.
//!
//! handles variable assignments, increments, and say output while marking
//! cycles, missing blocks, and unsupported statements in generated code.

use std::collections::{HashMap, HashSet};

use serde_json::Value;

use crate::parser::{InputValue, ScratchBlock, ScratchTarget, parse_input};

use super::{
    expressions::{boolean_input, expression},
    literals::{field_value, line, quote},
    scripts::ProcedureSignature,
};

// follow a statement's next links and lower each supported block in order.
pub(super) fn generate_statements(
    target: &ScratchTarget,
    first: Option<&str>,
    variables: &HashMap<String, String>,
    arguments: &HashMap<String, String>,
    procedures: &[ProcedureSignature],
    output: &mut String,
    indent: usize,
) {
    let mut current = first;
    let mut visited = HashSet::new();
    while let Some(id) = current {
        if !visited.insert(id.to_owned()) {
            line(
                output,
                indent,
                &format!("todo!({});", quote(&format!("block cycle: {id}"))),
            );
            break;
        }
        let Some(block) = target.blocks.get(id) else {
            line(
                output,
                indent,
                &format!("todo!({});", quote(&format!("missing block: {id}"))),
            );
            break;
        };
        match block.opcode.as_str() {
            "control_repeat" => {
                let times = expression(
                    target,
                    block.inputs.get("TIMES"),
                    variables,
                    arguments,
                    &mut HashSet::new(),
                );
                line(
                    output,
                    indent,
                    &format!(
                        "for _scratch_repeat_index in 0..(scratch_round({times}).max(0.0) as usize) {{"
                    ),
                );
                generate_substack(
                    target,
                    block,
                    "SUBSTACK",
                    variables,
                    arguments,
                    procedures,
                    output,
                    indent + 1,
                );
                line(output, indent, "}");
            }
            "control_repeat_until" | "control_while" => {
                let condition = boolean_input(
                    target,
                    block,
                    "CONDITION",
                    variables,
                    arguments,
                    &mut HashSet::new(),
                );
                let condition = if block.opcode == "control_repeat_until" {
                    format!("!({condition})")
                } else {
                    condition
                };
                line(output, indent, &format!("while {condition} {{"));
                generate_substack(
                    target,
                    block,
                    "SUBSTACK",
                    variables,
                    arguments,
                    procedures,
                    output,
                    indent + 1,
                );
                line(output, indent, "}");
            }
            "data_setvariableto" => {
                if let Some(name) = variable_field(block, "VARIABLE", variables) {
                    let input = block.inputs.get("VALUE");
                    let value =
                        expression(target, input, variables, arguments, &mut HashSet::new());
                    if let Some(source) = variable_input(target, input, variables, arguments) {
                        line(output, indent, &format!("{name} = {source}.clone();"));
                    } else {
                        line(output, indent, &format!("{name} = ({value}).into();"));
                    }
                } else {
                    line(output, indent, "todo!(\"missing variable reference\");");
                }
            }
            "data_changevariableby" => {
                if let Some(name) = variable_field(block, "VARIABLE", variables) {
                    let value = expression(
                        target,
                        block.inputs.get("VALUE"),
                        variables,
                        arguments,
                        &mut HashSet::new(),
                    );
                    line(
                        output,
                        indent,
                        &format!("{name} = scratch_add(&{name}, {value}).into();"),
                    );
                } else {
                    line(output, indent, "todo!(\"missing variable reference\");");
                }
            }
            "looks_say" => {
                let value = expression(
                    target,
                    block.inputs.get("MESSAGE"),
                    variables,
                    arguments,
                    &mut HashSet::new(),
                );
                let name = say_variable_name(target, block.inputs.get("MESSAGE"))
                    .unwrap_or_else(|| "say".to_owned());
                let label = quote(&name);
                line(output, indent, &format!("print_value!({label}, {value});"));
            }
            "procedures_call" => {
                if let Some(procedure) = procedure_for_call(block, procedures) {
                    let mut call_arguments = Vec::new();
                    for (index, argument_id) in procedure.argument_ids.iter().enumerate() {
                        let input = block.inputs.get(argument_id);
                        let argument = if let Some(source) =
                            variable_input(target, input, variables, arguments)
                        {
                            format!("{source}.clone()")
                        } else if input.is_some() {
                            let value = expression(
                                target,
                                input,
                                variables,
                                arguments,
                                &mut HashSet::new(),
                            );
                            format!("({value}).into()")
                        } else {
                            format!(
                                "todo!({})",
                                quote(&format!("missing procedure argument {index}"))
                            )
                        };
                        call_arguments.push(argument);
                    }
                    call_arguments.extend(
                        procedure
                            .state_variables
                            .iter()
                            .map(|(_, name)| format!("&mut {name}")),
                    );
                    line(
                        output,
                        indent,
                        &format!(
                            "{}({});",
                            procedure.function_name,
                            call_arguments.join(", ")
                        ),
                    );
                } else {
                    line(
                        output,
                        indent,
                        &format!(
                            "todo!({});",
                            quote("unsupported or missing custom procedure")
                        ),
                    );
                }
            }
            opcode => line(
                output,
                indent,
                &format!(
                    "todo!({});",
                    quote(&format!("unsupported statement: {opcode}"))
                ),
            ),
        }
        current = block.next.as_deref();
    }
}

fn procedure_for_call<'a>(
    block: &ScratchBlock,
    procedures: &'a [ProcedureSignature],
) -> Option<&'a ProcedureSignature> {
    let proccode = block.mutation.as_object()?.get("proccode")?.as_str()?;
    procedures
        .iter()
        .find(|procedure| procedure.proccode == proccode)
}

// resolve direct variable inputs so assignments copy the current dynamic value.
fn variable_input(
    target: &ScratchTarget,
    input: Option<&Value>,
    variables: &HashMap<String, String>,
    arguments: &HashMap<String, String>,
) -> Option<String> {
    match input.and_then(parse_input)? {
        InputValue::VariableReference(reference) => variables.get(reference.id).cloned(),
        InputValue::BlockReference(id) => {
            let reporter = target.blocks.get(id)?;
            if reporter.opcode == "data_variable" {
                variable_field(reporter, "VARIABLE", variables)
            } else if matches!(
                reporter.opcode.as_str(),
                "argument_reporter_string_number" | "argument_reporter_boolean"
            ) {
                field_value(reporter, "VALUE")
                    .and_then(Value::as_str)
                    .and_then(|name| arguments.get(name).cloned())
            } else {
                None
            }
        }
        _ => None,
    }
}

// lower the first statement connected to a loop's substack.
fn generate_substack(
    target: &ScratchTarget,
    block: &ScratchBlock,
    input_name: &str,
    variables: &HashMap<String, String>,
    arguments: &HashMap<String, String>,
    procedures: &[ProcedureSignature],
    output: &mut String,
    indent: usize,
) {
    let Some(InputValue::BlockReference(first)) =
        block.inputs.get(input_name).and_then(parse_input)
    else {
        return;
    };
    generate_statements(
        target,
        Some(first),
        variables,
        arguments,
        procedures,
        output,
        indent,
    );
}

// resolve a block's variable field ID to its allocated Rust identifier.
pub(super) fn variable_field(
    block: &ScratchBlock,
    field: &str,
    variables: &HashMap<String, String>,
) -> Option<String> {
    let data = block.fields.get(field)?.as_array()?;
    variables.get(data.get(1)?.as_str()?).cloned()
}

// get the original Scratch variable name displayed by a say reporter.
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
