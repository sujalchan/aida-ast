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

//! emits resumable custom block and green flag script frames.

use std::collections::HashMap;

use serde_json::Value;

use crate::parser::{InputValue, ScratchProject, ScratchTarget, parse_input};

use super::{
    literals::line,
    project::{ListInfo, VariableInfo, rust_identifier},
    statements::generate_resumable_script,
};

#[derive(Clone)]
pub(super) struct ProcedureSignature {
    pub(super) frame_id: usize,
    pub(super) definition_id: String,
    pub(super) proccode: String,
    pub(super) argument_ids: Vec<String>,
    pub(super) argument_names: Vec<String>,
    pub(super) warp: bool,
}

// emit a script enum and one step function for every green flag hat.
pub(super) fn generate_green_flag_functions(
    project: &ScratchProject,
    variables: &HashMap<String, VariableInfo>,
    lists: &HashMap<String, ListInfo>,
    output: &mut String,
) {
    let scripts = green_flag_scripts(project);
    if scripts.is_empty() {
        output.push_str("enum GeneratedScript {}\n\nimpl ResumableScript for GeneratedScript {\n    fn step(&mut self, _state: &mut ScratchState, _clock: &dyn Clock) -> ScriptStatus { match *self {} }\n}\n\n");
        return;
    }

    output.push_str("#[allow(dead_code)]\nstruct ProcedureFrame { function: usize, pc: usize, repeat_counts: Vec<usize>, wait_deadlines: Vec<Option<u64>>, arguments: Vec<ScratchValue>, warp_mode: bool }\n");
    output.push_str(
        "struct ScriptState { frames: Vec<ProcedureFrame>, warp_budget: WarpBudget }\n\n",
    );
    output.push_str("impl ScriptState {\n    fn warp_budget_expired(&mut self, clock: &dyn Clock) -> bool {\n        if !self.frames.last().is_some_and(|frame| frame.warp_mode) { self.warp_budget.reset(); return false; }\n        self.warp_budget.expired(clock)\n    }\n    fn yield_at_safe_point(&mut self, clock: &dyn Clock) -> bool {\n        if !self.frames.last().is_some_and(|frame| frame.warp_mode) { self.warp_budget.reset(); return true; }\n        self.warp_budget.expired(clock)\n    }\n}\n\n");
    line(output, 0, "enum GeneratedScript {");
    for script in &scripts {
        line(output, 1, &format!("{}(ScriptState),", script.variant));
    }
    line(output, 0, "}\n");

    output.push_str("impl ResumableScript for GeneratedScript {\n    fn step(&mut self, state: &mut ScratchState, clock: &dyn Clock) -> ScriptStatus {\n        match self {\n");
    for script in &scripts {
        line(
            output,
            3,
            &format!(
                "Self::{}(script) => step_{}(script, state, clock),",
                script.variant, script.function
            ),
        );
    }
    output.push_str("        }\n    }\n}\n\n");

    for script in &scripts {
        let procedures = procedure_signatures(script.target);
        let (start_pc, loop_count, wait_count) = generate_resumable_script(
            script.target,
            script.hat.next.as_deref(),
            variables,
            lists,
            &procedures,
            &script.function,
            output,
        );
        line(
            output,
            0,
            &format!(
                "fn new_{}() -> GeneratedScript {{ GeneratedScript::{}(ScriptState {{ frames: vec![ProcedureFrame {{ function: 0, pc: {}, repeat_counts: vec![0; {loop_count}], wait_deadlines: vec![None; {wait_count}], arguments: vec![], warp_mode: false }}], warp_budget: WarpBudget::new(500) }}) }}\n",
                script.function,
                script.variant,
                if start_pc == usize::MAX {
                    "usize::MAX".to_owned()
                } else {
                    start_pc.to_string()
                }
            ),
        );
    }
}

// collect green flag hats across targets in stable target and block order.
pub(super) fn generate_scripts(project: &ScratchProject, output: &mut String) {
    let scripts = green_flag_scripts(project);
    let values = scripts
        .iter()
        .map(|script| format!("new_{}()", script.function))
        .collect::<Vec<_>>();
    line(
        output,
        0,
        &format!(
            "fn new_scripts() -> Vec<GeneratedScript> {{ vec![{}] }}\n",
            values.join(", ")
        ),
    );
}

struct GreenFlagScript<'a> {
    target: &'a ScratchTarget,
    hat: &'a crate::parser::ScratchBlock,
    variant: String,
    function: String,
}

fn green_flag_scripts(project: &ScratchProject) -> Vec<GreenFlagScript<'_>> {
    let mut scripts = Vec::new();
    for target in &project.targets {
        let target_name = rust_identifier(&target.name).to_ascii_lowercase();
        for (_, hat) in event_hats(target)
            .into_iter()
            .filter(|(_, hat)| hat.opcode == "event_whenflagclicked")
        {
            let function = format!("green_flag_script_{target_name}_{}", scripts.len() + 1);
            scripts.push(GreenFlagScript {
                target,
                hat,
                variant: format!("Script{}", scripts.len() + 1),
                function,
            });
        }
    }
    scripts
}

pub(super) fn procedure_signatures(target: &ScratchTarget) -> Vec<ProcedureSignature> {
    let mut definitions: Vec<_> = target
        .blocks
        .iter()
        .filter(|(_, block)| block.opcode == "procedures_definition")
        .collect();
    definitions.sort_by(|(left_id, _), (right_id, _)| left_id.cmp(right_id));

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

        let argument_names = parse_string_array(mutation.get("argumentnames"));
        let argument_ids = parse_string_array(mutation.get("argumentids"));
        procedures.push(ProcedureSignature {
            frame_id: procedures.len() + 1,
            definition_id: definition_id.to_owned(),
            proccode: proccode.to_owned(),
            argument_ids,
            argument_names,
            warp: mutation
                .get("warp")
                .and_then(|value| {
                    value
                        .as_bool()
                        .or_else(|| value.as_str().and_then(|value| value.parse().ok()))
                })
                .unwrap_or(false),
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

fn event_hats(target: &ScratchTarget) -> Vec<(&str, &crate::parser::ScratchBlock)> {
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
        .map(|(id, block)| (id.as_str(), block))
        .collect();
    hats.sort_by(|(left_id, _), (right_id, _)| left_id.cmp(right_id));
    hats
}
