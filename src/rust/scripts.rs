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
use std::collections::{BTreeMap, BTreeSet};

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

// emit a script enum and one step function for each supported event hat.
pub(super) fn generate_green_flag_functions(
    project: &ScratchProject,
    variables: &HashMap<String, VariableInfo>,
    lists: &HashMap<String, ListInfo>,
    fast_math: bool,
    output: &mut String,
) {
    let scripts = event_scripts(project);
    if scripts.is_empty() {
        output.push_str("enum GeneratedScript {}\n\nimpl ResumableScript for GeneratedScript {\n    fn step(&mut self, _state: &mut ScratchState, _clock: &dyn Clock) -> ScriptStatus { match *self {} }\n}\n\n");
        output.push_str("fn new_broadcast_scripts(_message: usize, _state: &ScratchState) -> Vec<GeneratedScript> { Vec::new() }\n\n");
        return;
    }
    let message_keys = broadcast_message_keys(&scripts);
    let message_key_arms = message_keys
        .iter()
        .map(|(message, key)| {
            format!(
                "{} => {key}",
                super::literals::quote(&message.to_uppercase())
            )
        })
        .collect::<Vec<_>>();
    let message_key_match = if message_key_arms.is_empty() {
        "_ => 0".to_owned()
    } else {
        format!("{}, _ => 0", message_key_arms.join(", "))
    };
    line(
        output,
        0,
        &format!(
            "#[allow(dead_code)]\nfn broadcast_message_key(message: String) -> usize {{ match message.to_uppercase().as_str() {{ {message_key_match} }} }}\n"
        ),
    );

    output.push_str("#[allow(dead_code)]\nstruct ProcedureFrame { function: usize, pc: usize, repeat_counts: Vec<usize>, arguments: Vec<ScratchValue>, warp_mode: bool }\n");
    output
        .push_str("struct ScriptState { frames: Vec<ProcedureFrame>, __WARP_BUDGET_FIELD__ }\n\n");
    line(output, 0, "enum GeneratedScript {");
    for script in &scripts {
        line(
            output,
            1,
            &format!("{}(usize, ScriptState),", script.variant),
        );
    }
    line(output, 0, "}\n");

    output.push_str("impl ResumableScript for GeneratedScript {\n    fn step(&mut self, state: &mut ScratchState, clock: &dyn Clock) -> ScriptStatus {\n        match self {\n");
    for script in &scripts {
        line(
            output,
            3,
            &format!(
                "Self::{}(_, script) => step_{}(script, state, clock),",
                script.variant, script.function
            ),
        );
    }
    output.push_str("        }\n    }\n    fn target_instance(&self) -> usize { match self {\n");
    for script in &scripts {
        line(
            output,
            3,
            &format!("Self::{}(instance, _) => *instance,", script.variant),
        );
    }
    output.push_str("    } }\n    fn broadcast_receiver(&self) -> Option<(usize, usize, usize)> { match self {\n");
    for (index, script) in scripts.iter().enumerate() {
        if let Some(message) = &script.broadcast_message {
            let key = message_keys[message];
            line(
                output,
                3,
                &format!(
                    "Self::{}(instance, _) => Some(({key}, {index}, *instance)),",
                    script.variant
                ),
            );
        }
    }
    output.push_str(
        "        _ => None,\n    } }\n    fn restart_for_broadcast(&mut self) { match self {\n",
    );
    for script in &scripts {
        if script.broadcast_message.is_some() {
            line(
                output,
                3,
                &format!(
                    "Self::{}(instance, _) => {{ let instance = *instance; *self = new_{}_for(instance); }},",
                    script.variant, script.function
                ),
            );
        }
    }
    output.push_str("        _ => {},\n    } }\n}\n\n");

    for script in &scripts {
        let procedures = procedure_signatures(script.target);
        let (start_pc, loop_count) = generate_resumable_script(
            script.target,
            script.hat.next.as_deref(),
            variables,
            lists,
            &procedures,
            &script.function,
            fast_math,
            output,
        );
        line(
            output,
            0,
            &format!(
                "fn new_{}_for(instance: usize) -> GeneratedScript {{ GeneratedScript::{}(instance, ScriptState {{ frames: vec![ProcedureFrame {{ function: 0, pc: {}, repeat_counts: vec![0; {loop_count}], arguments: vec![], warp_mode: false }}], __WARP_BUDGET_INIT__ }}) }}\n",
                script.function,
                script.variant,
                if start_pc == usize::MAX {
                    "usize::MAX".to_owned()
                } else {
                    start_pc.to_string()
                }
            ),
        );
        if script.hat.opcode == "event_whenflagclicked" {
            line(
                output,
                0,
                &format!(
                    "fn new_{}() -> GeneratedScript {{ new_{}_for({}) }}\n",
                    script.function, script.function, script.target_index
                ),
            );
        } else if script.hat.opcode == "control_start_as_clone" {
            line(
                output,
                0,
                &format!(
                    "fn new_{}(instance: usize) -> GeneratedScript {{ new_{}_for(instance) }}\n",
                    script.function, script.function
                ),
            );
        }
    }
}

// collect green flag hats across targets in stable target and block order.
pub(super) fn generate_scripts(project: &ScratchProject, output: &mut String) {
    let scripts = event_scripts(project);
    let values = scripts
        .iter()
        .filter(|script| script.hat.opcode == "event_whenflagclicked")
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
    let mut receiver_groups = BTreeMap::<String, Vec<&GreenFlagScript<'_>>>::new();
    for script in &scripts {
        let Some(message) = &script.broadcast_message else {
            continue;
        };
        receiver_groups
            .entry(message.clone())
            .or_default()
            .push(script);
    }
    let message_keys = broadcast_message_keys(&scripts);
    let receiver_arms = receiver_groups.into_iter().map(|(message, receivers)| {
        let starters = receivers.into_iter().map(|script| format!(
            "for instance in state.instances_for_target({}) {{ scripts.push(new_{}_for(instance)); }}",
            script.target_index, script.function
        )).collect::<Vec<_>>().join(" ");
        format!("{} => {{ {} }}", message_keys[&message], starters)
    }).collect::<Vec<_>>();
    let receiver_match = if receiver_arms.is_empty() {
        "_ => {}".to_owned()
    } else {
        format!("{}, _ => {{}}", receiver_arms.join(", "))
    };
    line(
        output,
        0,
        &format!(
            "#[allow(dead_code, unused_variables, unused_mut)]\nfn new_broadcast_scripts(message: usize, state: &ScratchState) -> Vec<GeneratedScript> {{ let mut scripts = Vec::new(); match message {{ {receiver_match} }} scripts }}\n",
        ),
    );
    let mut clone_targets = Vec::new();
    for target_index in 0..project.targets.len() {
        let values = scripts
            .iter()
            .filter(|script| {
                script.target_index == target_index && script.hat.opcode == "control_start_as_clone"
            })
            .map(|script| format!("new_{}(_instance)", script.function))
            .collect::<Vec<_>>();
        if !values.is_empty() {
            clone_targets.push(format!("{target_index} => vec![{}]", values.join(", ")));
        }
    }
    let clone_arms = if clone_targets.is_empty() {
        "_ => Vec::new()".to_owned()
    } else {
        format!("{}, _ => Vec::new()", clone_targets.join(", "))
    };
    line(
        output,
        0,
        &format!(
            "fn new_clone_scripts(target: usize, _instance: usize) -> Vec<GeneratedScript> {{ match target {{ {clone_arms} }} }}\n"
        ),
    );

    let uses_warp_budget_expired = output.contains("script.warp_budget_expired(");
    let uses_yield_at_safe_point = output.contains("script.yield_at_safe_point(");
    let uses_warp_budget = output.contains("script.warp_budget")
        || uses_warp_budget_expired
        || uses_yield_at_safe_point;
    let budget_field = if uses_warp_budget {
        "warp_budget: WarpBudget,"
    } else {
        ""
    };
    let budget_init = if uses_warp_budget {
        "warp_budget: WarpBudget::new(500),"
    } else {
        ""
    };
    *output = output
        .replace("__WARP_BUDGET_FIELD__", budget_field)
        .replace("__WARP_BUDGET_INIT__", budget_init);

    let mut helpers = Vec::new();
    if uses_warp_budget_expired {
        helpers.push("    fn warp_budget_expired(&mut self, clock: &dyn Clock) -> bool {\n        if !self.frames.last().is_some_and(|frame| frame.warp_mode) { self.warp_budget.reset(); return false; }\n        self.warp_budget.expired(clock)\n    }");
    }
    if uses_yield_at_safe_point {
        helpers.push("    fn yield_at_safe_point(&mut self, clock: &dyn Clock) -> bool {\n        if !self.frames.last().is_some_and(|frame| frame.warp_mode) { self.warp_budget.reset(); return true; }\n        self.warp_budget.expired(clock)\n    }");
    }
    if !helpers.is_empty() {
        output.push_str("impl ScriptState {\n");
        output.push_str(&helpers.join("\n"));
        output.push_str("\n}\n\n");
    }
}

struct GreenFlagScript<'a> {
    target: &'a ScratchTarget,
    hat: &'a crate::parser::ScratchBlock,
    variant: String,
    function: String,
    target_index: usize,
    broadcast_message: Option<String>,
}

fn event_scripts(project: &ScratchProject) -> Vec<GreenFlagScript<'_>> {
    let mut scripts = Vec::new();
    for (target_index, target) in project.targets.iter().enumerate() {
        let target_name = rust_identifier(&target.name).to_ascii_lowercase();
        for (_, hat) in event_hats(target).into_iter().filter(|(_, hat)| {
            matches!(
                hat.opcode.as_str(),
                "event_whenflagclicked" | "control_start_as_clone" | "event_whenbroadcastreceived"
            )
        }) {
            let function_type = match hat.opcode.as_str() {
                "event_whenflagclicked" => "green_flag_script",
                "control_start_as_clone" => "clone_start_script",
                _ => "broadcast_receiver_script",
            };
            let function = format!("{function_type}_{target_name}_{}", scripts.len() + 1);
            scripts.push(GreenFlagScript {
                target,
                hat,
                variant: format!("Script{}", scripts.len() + 1),
                function,
                target_index,
                broadcast_message: if hat.opcode == "event_whenbroadcastreceived" {
                    broadcast_message(project, hat)
                } else {
                    None
                },
            });
        }
    }
    scripts
}

fn broadcast_message_keys(scripts: &[GreenFlagScript<'_>]) -> BTreeMap<String, usize> {
    scripts
        .iter()
        .filter_map(|script| script.broadcast_message.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .enumerate()
        .map(|(index, message)| (message, index + 1))
        .collect()
}

fn broadcast_message(
    project: &ScratchProject,
    hat: &crate::parser::ScratchBlock,
) -> Option<String> {
    let field = hat.fields.get("BROADCAST_OPTION")?.as_array()?;
    let name = field.first()?.as_str()?;
    let id = field.get(1).and_then(serde_json::Value::as_str);
    let stage = project.targets.iter().find(|target| target.is_stage)?;
    id.and_then(|id| stage.broadcasts.get(id))
        .cloned()
        .or_else(|| Some(name.to_owned()))
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
                        | "control_start_as_clone"
                )
        })
        .map(|(id, block)| (id.as_str(), block))
        .collect();
    hats.sort_by(|(left_id, _), (right_id, _)| left_id.cmp(right_id));
    hats
}
