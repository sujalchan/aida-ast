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
use std::error::Error;

#[derive(Debug, Deserialize)]
pub struct ScratchProject {
    pub targets: Vec<ScratchTarget>,
}

#[derive(Debug, Deserialize)]
pub struct ScratchTarget {
    pub name: String,
    #[serde(rename = "isStage")]
    pub is_stage: bool,

    pub blocks: HashMap<String, ScratchBlock>,

    #[serde(default)]
    pub variables: HashMap<String, ScratchVariable>,
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

#[derive(Debug, Deserialize)]
pub struct ScratchBlock {
    pub opcode: String,

    pub next: Option<String>,
    pub parent: Option<String>,

    pub inputs: HashMap<String, Value>,
    pub fields: HashMap<String, Value>,

    pub shadow: bool,

    #[serde(rename = "topLevel")]
    pub top_level: bool,

    pub x: Option<f64>,
    pub y: Option<f64>,
}

// Print the targets found
pub fn print_targets(json_str: &str) -> Result<(), Box<dyn Error>> {
    let project: ScratchProject = serde_json::from_str(json_str)?;

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

        print_blocks(target);
    }
    Ok(())
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
        let Some(value) = input.get(1) else {
            return;
        };

        // primitive input such as [4, "5"]
        if let Some(primitive) = value.as_array() {
            if let Some(data) = primitive.get(1) {
                println!("    {name}: {data}");
            }

            return;
        }

        // block reference such as "abc123"
        if let Some(block_id) = value.as_str() {
            if let Some(block) = target.blocks.get(block_id) {
                println!("    {name}: {} ({block_id})", block.opcode);
            } else {
                println!("    {name}: unknown block ({block_id})");
            }
        }
    }
}
