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

//! builds the generated Rust module and initializes its Scratch variables.
//!
//! allocates stable Rust identifiers for project variables, writes saved
//! variable values, then delegates each target's scripts to the script module.

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::parser::{InputValue, ScratchProject, ScratchTarget, parse_input};

use super::{
    expressions::constant_numeric_input,
    literals::{line, literal, quote},
    scripts::{generate_green_flag_functions, generate_scripts},
};

#[derive(Clone)]
pub(super) struct VariableInfo {
    pub(super) slot: usize,
    pub(super) name: String,
    pub(super) target: String,
    pub(super) is_stage: bool,
    pub(super) target_index: usize,
    pub(super) numeric: bool,
    pub(super) non_nan: bool,
}

#[derive(Clone)]
pub(super) struct ListInfo {
    pub(super) slot: usize,
    pub(super) name: String,
    pub(super) target: String,
    pub(super) is_stage: bool,
    pub(super) target_index: usize,
}

/// generate the module header, saved variable declarations, and target scripts.
pub fn generate(project: &ScratchProject) -> String {
    let variables = allocate_variables(project);
    let lists = allocate_lists(project);
    let mut output = String::from(
        "#[allow(unused_imports)]\nuse crate::runtime::{\n    scratch_acos_degrees, scratch_add, scratch_and, scratch_asin_degrees, scratch_atan_degrees,\n    scratch_bool, scratch_contains, scratch_cos, scratch_divide, scratch_equals,\n    scratch_greater_than, scratch_join, scratch_length, scratch_less_than, scratch_letter,\n    scratch_mod, scratch_multiply, scratch_not, scratch_or, scratch_random, scratch_round,\n    scratch_sin, scratch_subtract, scratch_tan, scratch_to_number, Clock, ResumableScript,\n    ScratchList, ScratchListMetadata, ScratchState, ScratchValue, ScratchVariableMetadata, ScriptScheduler, ScriptStatus, WarpBudget,\n};\n\n#[allow(unused_macros)]\nmacro_rules! print_value {\n    ($name:expr, $value:expr) => {\n        println!(\"{}: {:?}\", $name, $value);\n    };\n}\n\n",
    );
    output = output.replace(
        "ScratchVariableMetadata, ScriptScheduler, ScriptStatus",
        "ScratchVariableMetadata, SchedulerStatus, ScriptScheduler, ScriptStatus",
    );

    generate_green_flag_functions(project, &variables, &lists, &mut output);
    generate_scripts(project, &mut output);

    output.push_str("#[allow(dead_code)]\npub struct GeneratedProgram { state: ScratchState, scheduler: ScriptScheduler<GeneratedScript> }\n\n#[allow(dead_code)]\nimpl GeneratedProgram {\n    pub fn new() -> Self { Self::build(None) }\n    pub fn new_with_clock(clock: Box<dyn Clock>) -> Self { Self::build(Some(clock)) }\n    fn build(clock: Option<Box<dyn Clock>>) -> Self {\n");

    let mut initial_values = vec![String::new(); variables.len()];
    let mut variable_metadata = vec![String::new(); variables.len()];
    for (id, info) in &variables {
        let variable = project
            .targets
            .iter()
            .find(|target| target.name == info.target && target.is_stage == info.is_stage)
            .and_then(|target| target.variables.get(id))
            .or_else(|| {
                project
                    .targets
                    .iter()
                    .find_map(|target| target.variables.get(id))
            })
            .expect("every allocated variable has an initializer");
        initial_values[info.slot] = format!("({}).into()", literal(&variable.value));
        variable_metadata[info.slot] = format!(
            "ScratchVariableMetadata::for_target({}, {}, {}, {}, {})",
            quote(id),
            quote(&info.name),
            quote(&info.target),
            info.target_index,
            info.is_stage
        );
    }
    let mut initial_lists = vec![String::new(); lists.len()];
    let mut list_metadata = vec![String::new(); lists.len()];
    for (id, info) in &lists {
        let list = project
            .targets
            .iter()
            .find(|target| target.name == info.target && target.is_stage == info.is_stage)
            .and_then(|target| target.lists.get(id))
            .or_else(|| {
                project
                    .targets
                    .iter()
                    .find_map(|target| target.lists.get(id))
            })
            .expect("every allocated list has saved contents");
        initial_lists[info.slot] = format!(
            "ScratchList::new(vec![{}])",
            list.items
                .iter()
                .map(|item| format!("({}).into()", literal(item)))
                .collect::<Vec<_>>()
                .join(", ")
        );
        list_metadata[info.slot] = format!(
            "ScratchListMetadata::for_target({}, {}, {}, {}, {})",
            quote(id),
            quote(&info.name),
            quote(&info.target),
            info.target_index,
            info.is_stage
        );
    }
    line(
        &mut output,
        1,
        &format!(
            "let mut state = ScratchState::with_project_data(vec![{}], vec![{}], vec![{}], vec![{}], vec![false; {}]);",
            initial_values.join(", "),
            variable_metadata.join(", "),
            initial_lists.join(", "),
            list_metadata.join(", "),
            lists.len()
        ),
    );

    for (target_index, target) in project.targets.iter().enumerate() {
        line(
            &mut output,
            1,
            &format!(
                "state.register_target({target_index}, {}, {});",
                quote(&target.name),
                target.is_stage
            ),
        );
    }

    output.push_str("let scripts = new_scripts();\nlet scheduler = match clock { Some(clock) => ScriptScheduler::with_clock(scripts, clock), None => ScriptScheduler::new(scripts) };\n");
    output.push_str("    Self { state, scheduler }\n    }\n\n");
    output.push_str(
        "    pub fn step(&mut self) -> bool { self.advance() == SchedulerStatus::Progressed }\n",
    );
    output.push_str("    pub fn advance(&mut self) -> SchedulerStatus { let status = self.scheduler.advance(&mut self.state); let deleted = self.state.current_instance(); if !self.state.instance_is_alive(deleted) { self.scheduler.remove_target_scripts(deleted); } for instance in self.state.take_pending_clones() { if let Some(target) = self.state.instance_target(instance) { self.scheduler.add_scripts(new_clone_scripts(target, instance)); } } status }\n");
    output.push_str("    pub fn duration_until(&self, deadline: u64) -> std::time::Duration { self.scheduler.duration_until(deadline) }\n");
    output.push_str("    pub fn restart(&mut self) { self.state.remove_clones(); self.scheduler.restart(new_scripts()); }\n");
    output.push_str(
        "    pub fn is_complete(&self) -> bool { self.scheduler.active_scripts() == 0 }\n",
    );
    output.push_str("    pub fn state(&self) -> &ScratchState { &self.state }\n");
    output.push_str("    pub fn run_until_complete(&mut self) { loop { match self.advance() { SchedulerStatus::Progressed => {}, SchedulerStatus::WaitingUntil(deadline) => std::thread::sleep(self.duration_until(deadline)), SchedulerStatus::Complete => break } } }\n}\n\n");
    output.push_str("pub fn run() { let mut program = GeneratedProgram::new(); program.run_until_complete(); }\n");
    output
}

// assign each Scratch variable ID a stable shared-state slot.
fn allocate_variables(project: &ScratchProject) -> HashMap<String, VariableInfo> {
    let numeric_variables = numeric_variables(project);
    let non_nan_variables = non_nan_variables(project);
    let mut entries = BTreeMap::new();
    for target in &project.targets {
        for (id, variable) in &target.variables {
            let info = entries.entry(id.clone()).or_insert_with(|| VariableInfo {
                slot: 0,
                name: variable.name.clone(),
                target: target.name.clone(),
                is_stage: target.is_stage,
                target_index: project
                    .targets
                    .iter()
                    .position(|candidate| {
                        candidate.name == target.name && candidate.is_stage == target.is_stage
                    })
                    .unwrap_or(0),
                numeric: numeric_variables.contains(id),
                non_nan: non_nan_variables.contains(id),
            });
            if target.is_stage {
                info.name = variable.name.clone();
                info.target = target.name.clone();
                info.is_stage = true;
                info.target_index = project
                    .targets
                    .iter()
                    .position(|candidate| candidate.is_stage)
                    .unwrap_or(0);
            }
        }
    }
    let mut output = HashMap::new();
    for (slot, (id, mut info)) in entries.into_iter().enumerate() {
        info.slot = slot;
        output.insert(id, info);
    }
    output
}

// prove numeric variables non-nan when every write preserves that property.
fn non_nan_variables(project: &ScratchProject) -> HashSet<String> {
    let mut non_nan = project
        .targets
        .iter()
        .flat_map(|target| &target.variables)
        .filter(|(_, variable)| variable.value.as_f64().is_some_and(|value| !value.is_nan()))
        .map(|(id, _)| id.clone())
        .collect::<HashSet<_>>();

    loop {
        let previous = non_nan.clone();
        for target in &project.targets {
            for block in target.blocks.values() {
                if !matches!(
                    block.opcode.as_str(),
                    "data_setvariableto" | "data_changevariableby"
                ) {
                    continue;
                }
                let Some(id) = block
                    .fields
                    .get("VARIABLE")
                    .and_then(|field| field.as_array())
                    .and_then(|field| field.get(1))
                    .and_then(|id| id.as_str())
                else {
                    continue;
                };
                let preserves_non_nan = if block.opcode == "data_setvariableto" {
                    input_is_non_nan_number(
                        target,
                        block.inputs.get("VALUE"),
                        &previous,
                        &mut HashSet::new(),
                    )
                } else {
                    constant_numeric_input(target, block.inputs.get("VALUE"), &mut HashSet::new())
                        .is_some_and(f64::is_finite)
                };
                if !preserves_non_nan {
                    non_nan.remove(id);
                }
            }
        }
        if non_nan == previous {
            return non_nan;
        }
    }
}

fn input_is_non_nan_number(
    target: &ScratchTarget,
    input: Option<&serde_json::Value>,
    non_nan: &HashSet<String>,
    active: &mut HashSet<String>,
) -> bool {
    match input.and_then(parse_input) {
        Some(InputValue::Number(value)) => value
            .as_f64()
            .or_else(|| value.as_str().and_then(|text| text.parse().ok()))
            .is_some_and(|value: f64| !value.is_nan()),
        Some(InputValue::VariableReference(reference)) => non_nan.contains(reference.id),
        Some(InputValue::BlockReference(id)) => {
            if !active.insert(id.to_owned()) {
                return false;
            }
            let result = target.blocks.get(id).is_some_and(|block| {
                if block.opcode == "data_variable" {
                    return block
                        .fields
                        .get("VARIABLE")
                        .and_then(|field| field.as_array())
                        .and_then(|field| field.get(1))
                        .and_then(|id| id.as_str())
                        .is_some_and(|id| non_nan.contains(id));
                }
                matches!(
                    block.opcode.as_str(),
                    "math_number"
                        | "math_integer"
                        | "math_whole_number"
                        | "math_positive_number"
                        | "math_angle"
                ) && block
                    .fields
                    .get("NUM")
                    .and_then(|field| field.as_array())
                    .and_then(|field| field.first())
                    .and_then(|value| {
                        value
                            .as_f64()
                            .or_else(|| value.as_str().and_then(|text| text.parse().ok()))
                    })
                    .is_some_and(|value: f64| !value.is_nan())
            });
            active.remove(id);
            result
        }
        _ => false,
    }
}

// prove variables numeric only when their saved value and every assignment are numeric.
fn numeric_variables(project: &ScratchProject) -> HashSet<String> {
    let mut numeric = project
        .targets
        .iter()
        .flat_map(|target| &target.variables)
        .filter(|(_, variable)| variable.value.is_number() && !variable.is_cloud)
        .map(|(id, _)| id.clone())
        .collect::<HashSet<_>>();

    loop {
        let previous = numeric.clone();
        for target in &project.targets {
            for block in target.blocks.values() {
                if block.opcode != "data_setvariableto" {
                    continue;
                }
                let Some(id) = block
                    .fields
                    .get("VARIABLE")
                    .and_then(|field| field.as_array())
                    .and_then(|field| field.get(1))
                    .and_then(|id| id.as_str())
                else {
                    continue;
                };
                if !input_is_numeric(
                    target,
                    block.inputs.get("VALUE"),
                    &previous,
                    &mut HashSet::new(),
                ) {
                    numeric.remove(id);
                }
            }
        }
        if numeric == previous {
            return numeric;
        }
    }
}

fn input_is_numeric(
    target: &ScratchTarget,
    input: Option<&serde_json::Value>,
    numeric: &HashSet<String>,
    active: &mut HashSet<String>,
) -> bool {
    match input.and_then(parse_input) {
        Some(InputValue::Number(_)) => true,
        Some(InputValue::VariableReference(reference)) => numeric.contains(reference.id),
        Some(InputValue::BlockReference(id)) => {
            if !active.insert(id.to_owned()) {
                return false;
            }
            let result = target
                .blocks
                .get(id)
                .is_some_and(|block| match block.opcode.as_str() {
                    "operator_add" | "operator_subtract" | "operator_multiply"
                    | "operator_divide" | "operator_mod" | "operator_round" | "operator_random"
                    | "operator_mathop" | "data_lengthoflist" | "data_itemnumoflist" => true,
                    "data_variable" => block
                        .fields
                        .get("VARIABLE")
                        .and_then(|field| field.as_array())
                        .and_then(|field| field.get(1))
                        .and_then(|id| id.as_str())
                        .is_some_and(|id| numeric.contains(id)),
                    "math_number"
                    | "math_integer"
                    | "math_whole_number"
                    | "math_positive_number"
                    | "math_angle" => true,
                    _ => false,
                });
            active.remove(id);
            result
        }
        _ => false,
    }
}

// assign each scratch list id a stable shared-state slot.
fn allocate_lists(project: &ScratchProject) -> HashMap<String, ListInfo> {
    let mut entries = BTreeMap::new();
    for target in &project.targets {
        for (id, list) in &target.lists {
            let info = entries.entry(id.clone()).or_insert_with(|| ListInfo {
                slot: 0,
                name: list.name.clone(),
                target: target.name.clone(),
                is_stage: target.is_stage,
                target_index: project
                    .targets
                    .iter()
                    .position(|candidate| {
                        candidate.name == target.name && candidate.is_stage == target.is_stage
                    })
                    .unwrap_or(0),
            });
            if target.is_stage {
                info.name = list.name.clone();
                info.target = target.name.clone();
                info.is_stage = true;
                info.target_index = project
                    .targets
                    .iter()
                    .position(|candidate| candidate.is_stage)
                    .unwrap_or(0);
            }
        }
    }
    let mut output = HashMap::new();
    for (slot, (id, mut info)) in entries.into_iter().enumerate() {
        info.slot = slot;
        output.insert(id, info);
    }
    output
}

// convert a Scratch variable name into a valid Rust identifier.
pub(super) fn rust_identifier(name: &str) -> String {
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

// identify Rust keywords that need a prefix before they can be used as variables.
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
