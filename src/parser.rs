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
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct ScratchProject {
    pub targets: Vec<ScratchTarget>,
    #[serde(default)]
    pub monitors: Vec<ScratchMonitor>,
}

#[derive(Debug, Deserialize)]
pub struct ScratchMonitor {
    pub id: String,
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub opcode: String,
    #[serde(default)]
    pub params: HashMap<String, Value>,
    #[serde(default, rename = "spriteName")]
    pub sprite_name: Option<String>,
    #[serde(default)]
    pub value: Value,
    #[serde(default)]
    pub x: f64,
    #[serde(default)]
    pub y: f64,
    #[serde(default)]
    pub width: f64,
    #[serde(default)]
    pub height: f64,
    #[serde(default)]
    pub visible: bool,
    #[serde(default, rename = "sliderMin")]
    pub slider_min: f64,
    #[serde(default, rename = "sliderMax")]
    pub slider_max: f64,
}

#[derive(Debug)]
pub struct ScratchTarget {
    pub name: String,
    pub is_stage: bool,
    pub target_index: usize,
    pub layer_order: u32,
    pub x: f64,
    pub y: f64,
    pub direction: f64,
    pub size: f64,
    pub visible: bool,
    // preserve each target's startup rotation style for renderer initialization.
    pub rotation_style: String,
    pub volume: f64,
    // retain target drag metadata for Scratch-compatible pointer event timing.
    pub draggable: bool,
    pub blocks: HashMap<String, ScratchBlock>,
    pub variables: HashMap<String, ScratchVariable>,
    /// Variable IDs in their saved project order, used when a Scratch reporter
    /// resolves a variable by name rather than by ID.
    pub variable_order: Vec<String>,
    pub lists: HashMap<String, ScratchList>,
    pub costumes: Vec<ScratchAsset>,
    pub sounds: Vec<ScratchAsset>,
    pub current_costume: usize,
    pub broadcasts: HashMap<String, String>,

    // Shared compile-time project identities let reporters resolve static target
    // menus without adding target-name searches to their runtime hot path.
    pub target_identities: Arc<Vec<(String, usize, bool)>>,
}

#[derive(Debug, Deserialize)]
struct ScratchTargetData {
    name: String,
    #[serde(rename = "isStage")]
    is_stage: bool,
    #[serde(default, rename = "layerOrder")]
    layer_order: u32,
    #[serde(default)]
    x: f64,
    #[serde(default)]
    y: f64,
    #[serde(default = "default_direction")]
    direction: f64,
    #[serde(default = "default_size")]
    size: f64,
    #[serde(default = "default_visible")]
    visible: bool,
    #[serde(default = "default_rotation_style", rename = "rotationStyle")]
    rotation_style: String,
    #[serde(default = "default_volume")]
    volume: f64,
    #[serde(default)]
    draggable: bool,
    #[serde(default, deserialize_with = "deserialize_blocks")]
    blocks: HashMap<String, ScratchBlock>,
    #[serde(default)]
    variables: OrderedVariables,
    #[serde(default)]
    lists: HashMap<String, ScratchList>,
    #[serde(default)]
    costumes: Vec<ScratchAsset>,
    #[serde(default)]
    sounds: Vec<ScratchAsset>,
    #[serde(default, rename = "currentCostume")]
    current_costume: usize,
    #[serde(default)]
    broadcasts: HashMap<String, String>,
}

#[derive(Debug, Default)]
struct OrderedVariables {
    values: HashMap<String, ScratchVariable>,
    order: Vec<String>,
}

impl<'de> Deserialize<'de> for OrderedVariables {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct OrderedVariablesVisitor;

        impl<'de> serde::de::Visitor<'de> for OrderedVariablesVisitor {
            type Value = OrderedVariables;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a map of Scratch variable IDs to definitions")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut variables = OrderedVariables::default();
                while let Some((id, variable)) = map.next_entry::<String, ScratchVariable>()? {
                    if !variables.values.contains_key(&id) {
                        variables.order.push(id.clone());
                    }
                    variables.values.insert(id, variable);
                }
                Ok(variables)
            }
        }

        deserializer.deserialize_map(OrderedVariablesVisitor)
    }
}

impl<'de> Deserialize<'de> for ScratchTarget {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let target = ScratchTargetData::deserialize(deserializer)?;
        Ok(Self {
            name: target.name,
            is_stage: target.is_stage,
            target_index: 0,
            layer_order: target.layer_order,
            x: target.x,
            y: target.y,
            direction: target.direction,
            size: target.size,
            visible: target.visible,
            rotation_style: target.rotation_style,
            volume: target.volume,
            draggable: target.draggable,
            blocks: target.blocks,
            variables: target.variables.values,
            variable_order: target.variables.order,
            lists: target.lists,
            costumes: target.costumes,
            sounds: target.sounds,
            current_costume: target.current_costume,
            broadcasts: target.broadcasts,
            target_identities: Arc::new(Vec::new()),
        })
    }
}

fn default_direction() -> f64 {
    90.0
}
fn default_size() -> f64 {
    100.0
}
fn default_visible() -> bool {
    true
}
fn default_rotation_style() -> String {
    "all around".to_owned()
}
fn default_volume() -> f64 {
    100.0
}

#[derive(Debug, Deserialize)]
pub struct ScratchAsset {
    pub name: String,
    #[serde(default, rename = "dataFormat")]
    pub data_format: String,
    #[serde(default, rename = "assetId")]
    pub asset_id: String,
    #[serde(default)]
    pub md5ext: String,
    #[serde(default, rename = "bitmapResolution")]
    pub bitmap_resolution: Option<f64>,
    // Resolved from the asset bytes when loading an .sb3. These are Scratch
    // skin dimensions, rather than the asset's painted bounds.
    #[serde(default, rename = "aidaLogicalSize")]
    pub logical_size: Option<[f64; 2]>,
    #[serde(default, rename = "rotationCenterX")]
    pub rotation_center_x: f64,
    #[serde(default, rename = "rotationCenterY")]
    pub rotation_center_y: f64,
}

impl ScratchAsset {
    pub fn archive_path(&self) -> Option<String> {
        if !self.md5ext.is_empty() {
            Some(self.md5ext.clone())
        } else if !self.asset_id.is_empty() && !self.data_format.is_empty() {
            Some(format!("{}.{}", self.asset_id, self.data_format))
        } else {
            None
        }
    }

    pub fn identity(&self) -> &str {
        if self.asset_id.is_empty() {
            &self.md5ext
        } else {
            &self.asset_id
        }
    }
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

    #[serde(skip)]
    pub target_index: usize,

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
    let mut project: ScratchProject = serde_json::from_str(json_str)?;
    let target_identities = Arc::new(
        project
            .targets
            .iter()
            .enumerate()
            .map(|(index, target)| (target.name.clone(), index, target.is_stage))
            .collect(),
    );
    let broadcasts = project
        .targets
        .iter()
        .find(|target| target.is_stage)
        .map(|stage| stage.broadcasts.clone())
        .unwrap_or_default();
    for (target_index, target) in project.targets.iter_mut().enumerate() {
        target.target_index = target_index;
        target.target_identities = Arc::clone(&target_identities);
        for block in target.blocks.values_mut() {
            block.target_index = target_index;
        }
        for (id, name) in &broadcasts {
            target
                .broadcasts
                .entry(id.clone())
                .or_insert_with(|| name.clone());
        }
    }
    Ok(project)
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

#[cfg(test)]
mod graphics_metadata_tests {
    use super::parse_project;

    #[test]
    fn parses_sprite_assets_layer_order_and_variable_monitors() {
        let project = parse_project(
            r#"{
                "targets": [{
                    "name": "Stage", "isStage": true, "variables": {}, "lists": {},
                    "blocks": {}, "costumes": [], "sounds": []
                }, {
                    "name": "Sprite", "isStage": false, "layerOrder": 4,
                    "x": 12, "y": -8, "direction": 45, "size": 80,
                    "visible": true, "variables": {"v-id": ["score", 2]}, "lists": {},
                    "blocks": {}, "currentCostume": 0,
                    "costumes": [{"name": "costume", "md5ext": "asset.png",
                        "bitmapResolution": 2, "rotationCenterX": 10, "rotationCenterY": 11}],
                    "sounds": []
                }],
                "monitors": [{"id": "v-id", "mode": "large", "opcode": "data_variable",
                    "params": {"VARIABLE": "score"}, "spriteName": "Sprite", "value": 2,
                    "x": 10, "y": 20, "visible": true}]
            }"#,
        )
        .unwrap();

        let sprite = &project.targets[1];
        assert_eq!(sprite.layer_order, 4);
        assert_eq!(sprite.direction, 45.0);
        assert_eq!(sprite.costumes[0].md5ext, "asset.png");
        assert_eq!(project.monitors[0].id, "v-id");
        assert_eq!(project.monitors[0].sprite_name.as_deref(), Some("Sprite"));
    }
}
