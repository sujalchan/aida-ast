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

use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct ScratchProject {
    pub targets: Vec<ScratchTarget>,
}

#[derive(Debug, Deserialize)]
pub struct ScratchTarget {
    pub name: String,
    #[serde(rename = "isStage")]
    pub is_stage: bool,

    #[serde(default, deserialize_with = "deserialize_blocks")]
    pub blocks: HashMap<String, ScratchBlock>,

    #[serde(default)]
    pub variables: HashMap<String, ScratchVariable>,

    #[serde(default)]
    pub lists: HashMap<String, ScratchList>,
}

#[derive(Debug, Deserialize)]
#[serde(from = "(String, Vec<Value>)")]
pub struct ScratchList {
    pub name: String,
    pub items: Vec<Value>,
}

impl From<(String, Vec<Value>)> for ScratchList {
    fn from((name, items): (String, Vec<Value>)) -> Self {
        Self { name, items }
    }
}

#[derive(Debug, Deserialize)]
#[serde(from = "ScratchVariableData")]
pub struct ScratchVariable {
    pub name: String,
    pub value: Value,
    pub is_cloud: bool,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ScratchVariableData {
    Local((String, Value)),
    Cloud((String, Value, bool)),
}

impl From<ScratchVariableData> for ScratchVariable {
    fn from(data: ScratchVariableData) -> Self {
        match data {
            ScratchVariableData::Local((name, value)) => Self {
                name,
                value,
                is_cloud: false,
            },
            ScratchVariableData::Cloud((name, value, is_cloud)) => Self {
                name,
                value,
                is_cloud,
            },
        }
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct ScratchBlock {
    pub opcode: String,

    pub next: Option<String>,
    pub parent: Option<String>,

    #[serde(default)]
    pub inputs: HashMap<String, Value>,
    #[serde(default)]
    pub fields: HashMap<String, Value>,

    #[serde(default)]
    pub shadow: bool,

    #[serde(default, rename = "topLevel")]
    pub top_level: bool,

    pub x: Option<f64>,
    pub y: Option<f64>,

    #[serde(default)]
    pub mutation: Value,
}

// SB3 stores loose variable/list reporters as compact arrays, not block objects.
// Normalize these to ordinary blocks while retaining both block and variable IDs.
fn deserialize_blocks<'de, D>(deserializer: D) -> Result<HashMap<String, ScratchBlock>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let entries = HashMap::<String, Value>::deserialize(deserializer)?;
    entries
        .into_iter()
        .map(|(id, value)| {
            let block = if let Some(primitive) = value.as_array() {
                let (opcode, field) = match primitive.first().and_then(Value::as_u64) {
                    Some(12) => ("data_variable", "VARIABLE"),
                    Some(13) => ("data_listcontents", "LIST"),
                    _ => {
                        return Err(serde::de::Error::custom(format!(
                            "invalid compact block: {id}"
                        )));
                    }
                };
                let name = primitive.get(1).and_then(Value::as_str);
                let reference = primitive.get(2).and_then(Value::as_str);
                let (Some(name), Some(reference)) = (name, reference) else {
                    return Err(serde::de::Error::custom(format!(
                        "invalid reference in block: {id}"
                    )));
                };
                ScratchBlock {
                    opcode: opcode.to_owned(),
                    fields: HashMap::from([(
                        field.to_owned(),
                        serde_json::json!([name, reference]),
                    )]),
                    top_level: true,
                    x: primitive.get(3).and_then(Value::as_f64),
                    y: primitive.get(4).and_then(Value::as_f64),
                    ..Default::default()
                }
            } else {
                serde_json::from_value(value).map_err(serde::de::Error::custom)?
            };
            Ok((id, block))
        })
        .collect()
}

#[derive(Debug)]
pub struct ScratchReference<'a> {
    pub name: &'a str,
    pub id: &'a str,
}

#[derive(Debug)]
pub enum InputValue<'a> {
    Empty,
    Number(&'a Value),
    String(&'a Value),
    BlockReference(&'a str),
    VariableReference(ScratchReference<'a>),
    ListReference(ScratchReference<'a>),
}

/// Decode the active input. An obscured shadow at index 2 is not its value.
pub fn parse_input(input: &Value) -> Option<InputValue<'_>> {
    let input = input.as_array()?;
    if !matches!(input.first()?.as_u64()?, 1..=3) {
        return None;
    }
    let value = input.get(1)?;
    if value.is_null() {
        return Some(InputValue::Empty);
    }
    if let Some(id) = value.as_str() {
        return Some(InputValue::BlockReference(id));
    }

    let primitive = value.as_array()?;
    let code = primitive.first()?.as_u64()?;
    let value = primitive.get(1)?;
    match code {
        4..=8 => Some(InputValue::Number(value)),
        9..=11 => Some(InputValue::String(value)),
        12 | 13 => {
            let reference = ScratchReference {
                name: value.as_str()?,
                id: primitive.get(2)?.as_str()?,
            };
            if code == 12 {
                Some(InputValue::VariableReference(reference))
            } else {
                Some(InputValue::ListReference(reference))
            }
        }
        _ => None,
    }
}

pub fn parse_project(json_str: &str) -> Result<ScratchProject, serde_json::Error> {
    serde_json::from_str(json_str)
}

// Print the targets found
pub fn print_project(project: &ScratchProject) {
    let total_targets = project.targets.len();

    // Get sprite count by filtering out the Stage
    let sprite_count = project.targets.iter().filter(|t| !t.is_stage).count();
    let stage_count = total_targets - sprite_count;

    println!("Total targets: {total_targets} ({stage_count} Stage, {sprite_count} Sprites)");

    for target in &project.targets {
        let block_count = target.blocks.len();

        if target.is_stage {
            println!(" - Stage: {}, {block_count} blocks", target.name);
        } else {
            println!(" - Sprite: {}, {block_count} blocks", target.name);
        }

        for (id, variable) in &target.variables {
            let kind = if variable.is_cloud {
                "Cloud variable"
            } else {
                "Variable"
            };
            println!("   {kind}: {} ({id}) = {}", variable.name, variable.value);
        }

        for (id, list) in &target.lists {
            println!("   List: {} ({id}) = {:?}", list.name, list.items);
        }

        print_blocks(target);
    }
}

// Print block data
pub fn print_blocks(target: &ScratchTarget) {
    for (block_id, block) in &target.blocks {
        println!("\nBlock ID: {block_id}");
        println!("  Opcode: {}", block.opcode);
        println!("  Parent: {:?}", block.parent);
        println!("  Next: {:?}", block.next);
        println!(". Shadow: {}", block.shadow);
        println!("  Position: {:?}, {:?}", block.x, block.y);
        println!("  Top Level: {}", block.top_level);
        println!("  Inputs:");
        for (name, input) in &block.inputs {
            print_input(name, input, target);
        }

        println!("  Fields:");
        for (name, value) in &block.fields {
            println!("    {name}: {value}");
        }
    }

    fn print_input(name: &str, input: &Value, target: &ScratchTarget) {
        match parse_input(input) {
            Some(InputValue::Empty) => println!("    {name}: (empty)"),
            Some(InputValue::Number(data) | InputValue::String(data)) => {
                if let Some(text) = data.as_str() {
                    println!("    {name}: {text}");
                } else {
                    println!("    {name}: {data}");
                }
            }
            Some(InputValue::BlockReference(block_id)) => {
                if let Some(block) = target.blocks.get(block_id) {
                    println!("    {name}: {} ({block_id})", block.opcode);
                } else {
                    println!("    {name}: unknown block ({block_id})");
                }
            }
            Some(
                InputValue::VariableReference(reference) | InputValue::ListReference(reference),
            ) => {
                println!("    {name}: {} ({})", reference.name, reference.id);
            }
            None => println!("    {name}: unknown input {input}"),
        }
    }
}
