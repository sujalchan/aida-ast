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

//! handles custom procedure metadata and renders it as aida source.
//!
//! resolves definitions to their prototypes, reads argument names, IDs, types,
//! defaults, and warp settings, then generates procedure headers, calls, and
//! references to arguments in their enclosing procedure scope.

use super::*;

pub(super) struct Procedure {
    pub procedure_name: String,
    argument_ids: Vec<String>,
    argument_names: Vec<String>,
    argument_defaults: Vec<Value>,
    argument_types: Vec<&'static str>,
    warp_enabled: bool,
}

impl TargetGenerator<'_> {
    // resolve a procedure definition to its prototype block.
    fn resolve_prototype<'a>(
        &'a self,
        block: &'a ScratchBlock,
    ) -> Result<&'a ScratchBlock, String> {
        if block.opcode != "procedures_definition" {
            return Ok(block);
        }
        let Some(InputValue::BlockReference(prototype_id)) =
            block.inputs.get("custom_block").and_then(parse_input)
        else {
            return Err("missing procedure prototype".to_owned());
        };
        self.target
            .blocks
            .get(prototype_id)
            .filter(|prototype_block| {
                matches!(
                    prototype_block.opcode.as_str(),
                    "procedures_prototype" | "procedures_declaration"
                )
            })
            .ok_or_else(|| format!("missing procedure prototype: {prototype_id}"))
    }

    // decode the procedure's mutation data and validate its argument IDs.
    pub(super) fn procedure(&self, block: &ScratchBlock) -> Result<Procedure, String> {
        let prototype_block = self.resolve_prototype(block)?;

        let mutation = &prototype_block.mutation;

        let procedure_code = mutation
            .get("proccode")
            .and_then(Value::as_str)
            .ok_or_else(|| "missing procedure code".to_owned())?;

        let argument_ids = string_array(mutation, "argumentids", true)?;

        let argument_names = string_array(mutation, "argumentnames", false)?;

        let argument_defaults = mutation_array(mutation, "argumentdefaults", false)?;

        let argument_types = parse_procedure_code(procedure_code).1;
        if argument_ids.len() != argument_types.len()
            || argument_ids.iter().collect::<HashSet<_>>().len() != argument_ids.len()
        {
            return Err("invalid procedure argument IDs".to_owned());
        }

        let warp_enabled = match mutation.get("warp") {
            None | Some(Value::Bool(false)) => false,
            Some(Value::Bool(true)) => true,
            Some(Value::String(value)) if value == "true" => true,
            Some(Value::String(value)) if value == "false" => false,
            _ => return Err("invalid procedure warp flag".to_owned()),
        };

        Ok(Procedure {
            procedure_name: self
                .procedure_names
                .get(procedure_code)
                .cloned()
                .unwrap_or_else(|| identifier(procedure_code)),
            argument_ids,
            argument_names,
            argument_defaults,
            argument_types,
            warp_enabled,
        })
    }

    // render a definition or declaration from its signature and defaults.
    pub(super) fn procedure_header(&self, block: &ScratchBlock) -> Result<String, String> {
        let procedure = self.procedure(block)?;

        // saved projects can retain defaults for removed parameters. scratch
        // iterates argument IDs and ignores these trailing defaults.
        if procedure.argument_names.len() != procedure.argument_ids.len()
            || procedure.argument_defaults.len() < procedure.argument_ids.len()
        {
            return Err("invalid procedure argument names or defaults".to_owned());
        }

        let parameter_names = allocated_parameter_names(&procedure);
        let formatted_arguments = procedure
            .argument_ids
            .iter()
            .enumerate()
            .map(|(index, argument_id)| {
                format!(
                    "{}: {} = {}",
                    parameter_names[argument_id],
                    procedure.argument_types[index],
                    scalar(&procedure.argument_defaults[index])
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        let declaration_kind = if block.opcode == "procedures_declaration" {
            "declare"
        } else {
            "define"
        };

        let warp_suffix = if procedure.warp_enabled { " warp" } else { "" };

        Ok(format!(
            "{declaration_kind}: {}({formatted_arguments}){warp_suffix}",
            procedure.procedure_name
        ))
    }

    // render arguments in the order recorded by the prototype mutation.
    pub(super) fn procedure_call(&self, block: &ScratchBlock) -> Result<String, String> {
        let procedure = self.procedure(block)?;
        let formatted_arguments = procedure
            .argument_ids
            .iter()
            .zip(&procedure.argument_types)
            .map(|(argument_id, argument_type)| {
                let mut active = HashSet::new();
                if *argument_type == "bool" {
                    self.boolean_input(block, argument_id, &mut active).text
                } else {
                    self.input_expression(block.inputs.get(argument_id), &mut active)
                        .text
                }
            })
            .collect::<Vec<_>>()
            .join(", ");
        Ok(format!(
            "call: {}({formatted_arguments})",
            procedure.procedure_name
        ))
    }

    // find the enclosing procedure so an argument reporter uses its allocated name.
    pub(super) fn argument_name(&self, block: &ScratchBlock) -> String {
        let Some(argument_name) = field_value(block, "VALUE").and_then(Value::as_str) else {
            return Expression::unknown("missing argument name").text;
        };

        // parent links identify the procedure scope even for nested reporters.
        let mut parent_id = block.parent.as_deref();
        let mut visited_parent_ids = HashSet::new();

        while let Some(parent_id_value) = parent_id {
            if !visited_parent_ids.insert(parent_id_value) {
                break;
            }
            let Some(parent_block) = self.target.blocks.get(parent_id_value) else {
                break;
            };
            if parent_block.opcode == "procedures_definition"
                || parent_block.opcode == "procedures_prototype"
            {
                if let Ok(procedure) = self.procedure(parent_block)
                    && let Some(index) = procedure
                        .argument_names
                        .iter()
                        .position(|name| name == argument_name)
                    && let Some(argument_id) = procedure.argument_ids.get(index)
                    && let Some(allocated_name) =
                        allocated_parameter_names(&procedure).get(argument_id)
                {
                    return allocated_name.clone();
                }
                break;
            }
            parent_id = parent_block.parent.as_deref();
        }
        identifier(argument_name)
    }
}

// allocate stable identifiers for parameter names within one procedure.
fn allocated_parameter_names(procedure: &Procedure) -> HashMap<String, String> {
    allocate_names(
        procedure
            .argument_ids
            .iter()
            .zip(&procedure.argument_names)
            .map(|(argument_id, argument_name)| (argument_id.as_str(), argument_name.as_str())),
    )
}

// split placeholders from the procedure label while keeping escaped percent signs literal.
pub(super) fn parse_procedure_code(procedure_code: &str) -> (String, Vec<&'static str>) {
    let mut procedure_label = String::new();
    let mut argument_types = Vec::new();
    let mut code_characters = procedure_code.chars().peekable();
    while let Some(character) = code_characters.next() {
        if character == '\\' && code_characters.peek() == Some(&'%') {
            procedure_label.push('%');
            code_characters.next();
        } else if character == '%' && matches!(code_characters.peek(), Some('s' | 'n' | 'b')) {
            argument_types.push(match code_characters.next() {
                Some('b') => "bool",
                Some('n') => "number",
                _ => "value",
            });
            procedure_label.push(' ');
        } else {
            procedure_label.push(character);
        }
    }
    (
        procedure_label
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" "),
        argument_types,
    )
}

// mutation arrays may be stored as arrays or as serialized json strings.
fn mutation_array(
    mutation: &Value,
    mutation_key: &str,
    required: bool,
) -> Result<Vec<Value>, String> {
    match mutation.get(mutation_key) {
        Some(Value::Array(values)) => Ok(values.clone()),
        Some(Value::String(serialized_values)) => serde_json::from_str(serialized_values)
            .map_err(|_| format!("invalid procedure {mutation_key}")),
        None if !required => Ok(Vec::new()),
        _ => Err(format!("missing or invalid procedure {mutation_key}")),
    }
}

fn string_array(
    mutation: &Value,
    mutation_key: &str,
    required: bool,
) -> Result<Vec<String>, String> {
    mutation_array(mutation, mutation_key, required)?
        .into_iter()
        .map(|argument| {
            argument
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("invalid procedure {mutation_key}"))
        })
        .collect()
}
