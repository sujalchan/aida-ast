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

use super::*;

pub(super) struct Procedure {
    pub name: String,
    ids: Vec<String>,
    names: Vec<String>,
    defaults: Vec<Value>,
    types: Vec<&'static str>,
    warp: bool,
}

impl TargetGenerator<'_> {
    fn prototype<'a>(&'a self, block: &'a ScratchBlock) -> Result<&'a ScratchBlock, String> {
        if block.opcode != "procedures_definition" {
            return Ok(block);
        }
        let Some(InputValue::BlockReference(id)) =
            block.inputs.get("custom_block").and_then(parse_input)
        else {
            return Err("missing procedure prototype".to_owned());
        };
        self.target
            .blocks
            .get(id)
            .filter(|b| {
                matches!(
                    b.opcode.as_str(),
                    "procedures_prototype" | "procedures_declaration"
                )
            })
            .ok_or_else(|| format!("missing procedure prototype: {id}"))
    }

    pub(super) fn procedure(&self, block: &ScratchBlock) -> Result<Procedure, String> {
        let block = self.prototype(block)?;
        let mutation = &block.mutation;
        let code = mutation
            .get("proccode")
            .and_then(Value::as_str)
            .ok_or_else(|| "missing procedure code".to_owned())?;
        let ids = string_array(mutation, "argumentids", true)?;
        let names = string_array(mutation, "argumentnames", false)?;
        let defaults = mutation_array(mutation, "argumentdefaults", false)?;
        let types = parts(code).1;
        if ids.len() != types.len() || ids.iter().collect::<HashSet<_>>().len() != ids.len() {
            return Err("invalid procedure argument IDs".to_owned());
        }
        let warp = match mutation.get("warp") {
            None | Some(Value::Bool(false)) => false,
            Some(Value::Bool(true)) => true,
            Some(Value::String(s)) if s == "true" => true,
            Some(Value::String(s)) if s == "false" => false,
            _ => return Err("invalid procedure warp flag".to_owned()),
        };
        Ok(Procedure {
            name: self
                .procedure_names
                .get(code)
                .cloned()
                .unwrap_or_else(|| identifier(code)),
            ids,
            names,
            defaults,
            types,
            warp,
        })
    }

    pub(super) fn procedure_header(&self, block: &ScratchBlock) -> Result<String, String> {
        let procedure = self.procedure(block)?;
        if procedure.names.len() != procedure.ids.len()
            || procedure.defaults.len() != procedure.ids.len()
        {
            return Err("invalid procedure argument names or defaults".to_owned());
        }
        let names = parameter_names(&procedure);
        let args = procedure
            .ids
            .iter()
            .enumerate()
            .map(|(i, id)| {
                format!(
                    "{}: {} = {}",
                    names[id],
                    procedure.types[i],
                    scalar(&procedure.defaults[i])
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        let kind = if block.opcode == "procedures_declaration" {
            "declare"
        } else {
            "define"
        };
        let warp = if procedure.warp { " warp" } else { "" };
        Ok(format!("{kind}: {}({args}){warp}", procedure.name))
    }

    pub(super) fn procedure_call(&self, block: &ScratchBlock) -> Result<String, String> {
        let procedure = self.procedure(block)?;
        let args = procedure
            .ids
            .iter()
            .zip(&procedure.types)
            .map(|(id, kind)| {
                let mut active = HashSet::new();
                if *kind == "bool" {
                    self.boolean_input(block, id, &mut active).text
                } else {
                    self.input_expression(block.inputs.get(id), &mut active)
                        .text
                }
            })
            .collect::<Vec<_>>()
            .join(", ");
        Ok(format!("call: {}({args})", procedure.name))
    }

    pub(super) fn argument_name(&self, block: &ScratchBlock) -> String {
        let Some(name) = field_value(block, "VALUE").and_then(Value::as_str) else {
            return Expression::unknown("missing argument name").text;
        };
        // Parent links identify the procedure scope even for nested reporters.
        let mut parent = block.parent.as_deref();
        let mut visited = HashSet::new();
        while let Some(id) = parent {
            if !visited.insert(id) {
                break;
            }
            let Some(block) = self.target.blocks.get(id) else {
                break;
            };
            if block.opcode == "procedures_definition" || block.opcode == "procedures_prototype" {
                if let Ok(procedure) = self.procedure(block) {
                    if let Some(index) = procedure.names.iter().position(|n| n == name) {
                        if let Some(id) = procedure.ids.get(index) {
                            if let Some(name) = parameter_names(&procedure).get(id) {
                                return name.clone();
                            }
                        }
                    }
                }
                break;
            }
            parent = block.parent.as_deref();
        }
        identifier(name)
    }
}

fn parameter_names(procedure: &Procedure) -> HashMap<String, String> {
    allocate_names(
        procedure
            .ids
            .iter()
            .zip(&procedure.names)
            .map(|(id, name)| (id.as_str(), name.as_str())),
    )
}

// Scratch permits escaped percent signs in procedure labels. These must not
// become parameters or disappear from the generated procedure name.
pub(super) fn parts(code: &str) -> (String, Vec<&'static str>) {
    let mut label = String::new();
    let mut types = Vec::new();
    let mut chars = code.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' && chars.peek() == Some(&'%') {
            label.push('%');
            chars.next();
        } else if c == '%' && matches!(chars.peek(), Some('s' | 'n' | 'b')) {
            types.push(match chars.next() {
                Some('b') => "bool",
                Some('n') => "number",
                _ => "value",
            });
            label.push(' ');
        } else {
            label.push(c);
        }
    }
    (
        label.split_whitespace().collect::<Vec<_>>().join(" "),
        types,
    )
}

fn mutation_array(mutation: &Value, key: &str, required: bool) -> Result<Vec<Value>, String> {
    match mutation.get(key) {
        Some(Value::Array(values)) => Ok(values.clone()),
        Some(Value::String(encoded)) => {
            serde_json::from_str(encoded).map_err(|_| format!("invalid procedure {key}"))
        }
        None if !required => Ok(Vec::new()),
        _ => Err(format!("missing or invalid procedure {key}")),
    }
}

fn string_array(mutation: &Value, key: &str, required: bool) -> Result<Vec<String>, String> {
    mutation_array(mutation, key, required)?
        .into_iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("invalid procedure {key}"))
        })
        .collect()
}
