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

//! finds supported Scratch event scripts and sends their bodies to statement lowering.
//!
//! sorting hat block IDs keeps script output stable when the project is generated
//! more than once.

use std::collections::{HashMap, HashSet};

use serde_json::Value;

use crate::parser::{InputValue, ScratchTarget, parse_input};

use super::{literals::line, project::rust_identifier, statements::generate_statements};

#[derive(Clone)]
pub(super) struct ProcedureSignature {
    pub(super) definition_id: String,
    pub(super) function_name: String,
    pub(super) proccode: String,
    pub(super) argument_ids: Vec<String>,
    pub(super) argument_names: Vec<String>,
    pub(super) arguments: HashMap<String, String>,
    pub(super) state_variables: Vec<(String, String)>,
}

// emit Scratch custom block definitions as Rust functions.
pub(super) fn generate_procedures(
    target: &ScratchTarget,
    variables: &HashMap<String, String>,
    output: &mut String,
) {
    let procedures = procedure_signatures(target, variables);
    for procedure in &procedures {
        let Some(definition) = target.blocks.get(&procedure.definition_id) else {
            continue;
        };
        let mut parameters: Vec<_> = procedure
            .argument_names
            .iter()
            .map(|name| {
                let rust_name = procedure
                    .arguments
                    .get(name)
                    .cloned()
                    .unwrap_or_else(|| rust_identifier(name));
                format!("{rust_name}: ScratchValue")
            })
            .collect();
        parameters.extend(
            procedure
                .state_variables
                .iter()
                .map(|(_, name)| format!("state_{name}: &mut ScratchValue")),
        );

        output.push_str("#[allow(dead_code, unused_variables, unused_mut, unused_assignments)]\n");
        line(
            output,
            0,
            &format!(
                "fn {}({}) {{",
                procedure.function_name,
                parameters.join(", ")
            ),
        );
        for (_, name) in &procedure.state_variables {
            line(
                output,
                1,
                &format!("let mut {name} = state_{name}.clone();"),
            );
        }

        generate_statements(
            target,
            definition.next.as_deref(),
            variables,
            &procedure.arguments,
            &procedures,
            output,
            1,
        );

        for (_, name) in &procedure.state_variables {
            line(output, 1, &format!("*state_{name} = {name};"));
        }
        line(output, 0, "}");
        output.push('\n');
    }
}

pub(super) fn procedure_signatures(
    target: &ScratchTarget,
    variables: &HashMap<String, String>,
) -> Vec<ProcedureSignature> {
    let mut definitions: Vec<_> = target
        .blocks
        .iter()
        .filter(|(_, block)| block.opcode == "procedures_definition")
        .collect();
    definitions.sort_by(|(left_id, _), (right_id, _)| left_id.cmp(right_id));

    let mut used_names = HashSet::from(["run".to_owned()]);
    let mut procedures = Vec::new();
    for (definition_id, definition) in definitions {
        let Some(InputValue::BlockReference(prototype_id)) =
            definition.inputs.get("custom_block").and_then(parse_input)
        else {
            continue;
        };
        let Some(prototype) = target.blocks.get(prototype_id) else {
            continue;
        };
        let Some(mutation) = prototype.mutation.as_object() else {
            continue;
        };
        let Some(proccode) = mutation.get("proccode").and_then(Value::as_str) else {
            continue;
        };

        let base_name = rust_identifier(
            &proccode
                .replace("%s", " ")
                .replace("%b", " ")
                .replace("%n", " "),
        );
        let mut function_name = base_name.clone();
        let mut suffix = 2;
        while !used_names.insert(function_name.clone()) {
            function_name = format!("{base_name}_{suffix}");
            suffix += 1;
        }

        let argument_names = parse_string_array(mutation.get("argumentnames"));
        let argument_ids = parse_string_array(mutation.get("argumentids"));
        let state_variables = procedure_variables(target, definition.next.as_deref(), variables);
        let reserved_names = state_variables
            .iter()
            .flat_map(|(_, name)| [name.clone(), format!("state_{name}")])
            .collect::<HashSet<_>>();
        let arguments = allocate_argument_names(&argument_names, &reserved_names);
        procedures.push(ProcedureSignature {
            definition_id: definition_id.to_owned(),
            function_name,
            proccode: proccode.to_owned(),
            argument_ids,
            argument_names,
            arguments,
            state_variables,
        });
    }
    procedures
}

fn parse_string_array(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_str)
        .and_then(|encoded| serde_json::from_str::<Vec<String>>(encoded).ok())
        .unwrap_or_default()
}

fn allocate_argument_names(
    names: &[String],
    reserved: &HashSet<String>,
) -> HashMap<String, String> {
    let mut used = reserved.clone();
    let mut output = HashMap::new();
    for name in names {
        let base = rust_identifier(name);
        let mut candidate = base.clone();
        let mut suffix = 2;
        while !used.insert(candidate.clone()) {
            candidate = format!("{base}_{suffix}");
            suffix += 1;
        }
        output.insert(name.clone(), candidate);
    }
    output
}

fn procedure_variables(
    target: &ScratchTarget,
    first: Option<&str>,
    variables: &HashMap<String, String>,
) -> Vec<(String, String)> {
    let mut pending = first.into_iter().map(str::to_owned).collect::<Vec<_>>();
    let mut visited = HashSet::new();
    let mut found = HashSet::new();
    while let Some(id) = pending.pop() {
        if !visited.insert(id.clone()) {
            continue;
        }
        let Some(block) = target.blocks.get(&id) else {
            continue;
        };

        if let Some(data) = block.fields.get("VARIABLE").and_then(Value::as_array) {
            if let Some(variable_id) = data.get(1).and_then(Value::as_str) {
                if variables.contains_key(variable_id) {
                    found.insert(variable_id.to_owned());
                }
            }
        }
        if let Some(next) = &block.next {
            pending.push(next.clone());
        }
        for input in block.inputs.values() {
            match parse_input(input) {
                Some(InputValue::VariableReference(reference)) => {
                    if variables.contains_key(reference.id) {
                        found.insert(reference.id.to_owned());
                    }
                }
                Some(InputValue::BlockReference(id)) => pending.push(id.to_owned()),
                _ => {}
            }
        }
    }

    let mut output: Vec<_> = found
        .into_iter()
        .filter_map(|id| variables.get(&id).map(|name| (id, name.clone())))
        .collect();
    output.sort_by(|(left_id, left_name), (right_id, right_name)| {
        left_name.cmp(right_name).then(left_id.cmp(right_id))
    });
    output
}

// lower each supported top-level event hat in stable block-ID order.
pub(super) fn generate_scripts(
    target: &ScratchTarget,
    variables: &HashMap<String, String>,
    output: &mut String,
) {
    let procedures = procedure_signatures(target, variables);
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
        generate_statements(
            target,
            hat.next.as_deref(),
            variables,
            &HashMap::new(),
            &procedures,
            output,
            1,
        );
    }
}
