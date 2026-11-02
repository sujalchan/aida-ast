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
    expressions::expression,
    literals::{field_value, line, quote},
};

// follow a statement's next links and lower each supported block in order.
pub(super) fn generate_statements(
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
