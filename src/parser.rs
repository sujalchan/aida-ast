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

use crate::value::{InferredValue, infer_value};

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

impl ScratchTarget {
    fn variable<'a>(
        &'a self,
        id: &str,
        stage_variables: Option<&'a HashMap<String, ScratchVariable>>,
    ) -> Option<&'a ScratchVariable> {
        self.variables
            .get(id)
            .or_else(|| stage_variables.and_then(|variables| variables.get(id)))
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

impl ScratchVariable {
    pub fn inferred_value(&self) -> Option<InferredValue> {
        infer_value(&self.value)
    }
}

#[derive(Debug, Deserialize)]
pub struct ScratchBlock {
    pub opcode: String,

    pub next: Option<String>,
    pub parent: Option<String>,

    pub inputs: HashMap<String, ScratchInput>,
    pub fields: HashMap<String, Value>,

    pub shadow: bool,

    #[serde(rename = "topLevel")]
    pub top_level: bool,

    pub x: Option<f64>,
    pub y: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(transparent)]
pub struct ScratchInput(Vec<Value>);

impl ScratchInput {
    fn primitive(&self) -> Option<&[Value]> {
        let value = self.0.get(1)?.as_array()?;
        value.first()?.as_u64()?;
        Some(value)
    }

    fn literal_value(&self) -> Option<&Value> {
        let primitive = self.primitive()?;
        match primitive.first()?.as_u64()? {
            4..=10 => primitive.get(1),
            _ => None,
        }
    }

    fn variable_id(&self) -> Option<&str> {
        let primitive = self.primitive()?;
        if primitive.first()?.as_u64()? != 12 {
            return None;
        }
        primitive.get(2)?.as_str()
    }
}

// Print the targets found
pub fn print_targets(json_str: &str) -> Result<(), Box<dyn Error>> {
    let project: ScratchProject = serde_json::from_str(json_str)?;

    let total_targets = project.targets.len();

    // Get sprite count by filtering out the Stage
    let sprite_count = project.targets.iter().filter(|t| !t.is_stage).count();
    let stage_count = total_targets - sprite_count;
    let stage_variables = project
        .targets
        .iter()
        .find(|target| target.is_stage)
        .map(|stage| &stage.variables);

    println!("Total targets: {total_targets} ({stage_count} Stage, {sprite_count} Sprites)");

    for target in &project.targets {
        let block_count = target.blocks.len();

        if target.is_stage {
            println!(" - Stage: {}, {block_count} blocks", target.name);
        } else {
            println!(" - Sprite: {}, {block_count} blocks", target.name);
        }

        for (id, variable) in &target.variables {
            let label = if variable.is_cloud {
                "Cloud variable"
            } else {
                "Variable"
            };
            println!("   {label}: {} ({id})", variable.name);
            if let Some(value) = variable.inferred_value() {
                println!("      Type: {}", value.type_name());
                println!("      Value: {}", variable.value);
            }
        }

        print_blocks(target, stage_variables);
    }
    Ok(())
}

// Print block data
pub fn print_blocks(
    target: &ScratchTarget,
    stage_variables: Option<&HashMap<String, ScratchVariable>>,
) {
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
            print_input(name, input, target, stage_variables);
        }

        println!("  Fields:");
        for (name, value) in &block.fields {
            println!("    {name}: {value}");
        }
    }

    fn print_input(
        name: &str,
        input: &ScratchInput,
        target: &ScratchTarget,
        stage_variables: Option<&HashMap<String, ScratchVariable>>,
    ) {
        println!("    {name}:");

        let variable = input
            .variable_id()
            .and_then(|id| target.variable(id, stage_variables));
        if let Some(variable) = variable {
            if let Some(value) = variable.inferred_value() {
                println!("      Type: {}", value.type_name());
                println!("      Value: {}", variable.value);
            }
        } else if let Some(data) = input.literal_value() {
            if let Some(value) = infer_value(data) {
                println!("      Type: {}", value.type_name());
                println!("      Value: {data}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ScratchInput, ScratchTarget};
    use crate::value::{InferredValue, infer_value};
    use serde_json::json;

    #[test]
    fn infers_stored_variables_and_block_literals() {
        let target: ScratchTarget = serde_json::from_value(json!({
            "name": "Stage",
            "isStage": true,
            "blocks": {},
            "variables": {
                "score-id": ["score", "42"],
                "enabled-id": ["enabled", true, true]
            }
        }))
        .unwrap();

        let score = &target.variables["score-id"];
        assert_eq!(score.inferred_value(), Some(InferredValue::I64(42)));
        assert!(!score.is_cloud);
        assert_eq!(
            target.variables["enabled-id"].inferred_value(),
            Some(InferredValue::Bool(true))
        );
        assert!(target.variables["enabled-id"].is_cloud);

        let literal: ScratchInput = serde_json::from_value(json!([1, [10, "3.5"]])).unwrap();
        assert_eq!(
            literal.literal_value().and_then(infer_value),
            Some(InferredValue::F64(3.5))
        );

        let reference: ScratchInput =
            serde_json::from_value(json!([1, [12, "score", "score-id"]])).unwrap();
        assert_eq!(reference.variable_id(), Some("score-id"));
        assert!(reference.literal_value().is_none());

        let sprite: ScratchTarget = serde_json::from_value(json!({
            "name": "Sprite1",
            "isStage": false,
            "blocks": {}
        }))
        .unwrap();
        let resolved = sprite
            .variable("score-id", Some(&target.variables))
            .unwrap();
        assert_eq!(resolved.inferred_value(), Some(InferredValue::I64(42)));
    }
}
