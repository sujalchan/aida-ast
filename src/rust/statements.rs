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

//! traverses Scratch statement chains and emits their Rust equivalents.
//!
//! handles variable assignments, increments, and say output while marking
//! cycles, missing blocks, and unsupported statements in generated code.

use std::collections::{BTreeSet, HashMap, HashSet};

use serde_json::Value;

use crate::parser::{InputValue, ScratchBlock, ScratchTarget, parse_input};

use super::{
    expressions::{
        LocalValueKind, boolean_input, collect_numeric_local_slots, constant_numeric_input,
        constant_random_bounds, expression, expression_argument, local_expression, local_to_bool,
        local_to_number,
    },
    literals::{field_value, line, quote},
    project::{ListInfo, VariableInfo},
    scripts::ProcedureSignature,
};

struct ScriptGraph {
    nodes: Vec<ScriptNode>,
    block_pcs: HashMap<String, usize>,
    boundary_pcs: HashMap<String, usize>,
    fallthrough_pcs: HashMap<String, usize>,
    repeat_slots: HashMap<String, usize>,
}

struct ProcedureEntry {
    function_id: usize,
    start_pc: usize,
    repeat_count: usize,
    direct_name: Option<String>,
    numeric_direct: bool,
}

enum ScriptNode {
    Block(String),
    LoopBoundary(String),
}

impl ScriptGraph {
    fn new() -> Self {
        Self {
            nodes: Vec::new(),
            block_pcs: HashMap::new(),
            boundary_pcs: HashMap::new(),
            fallthrough_pcs: HashMap::new(),
            repeat_slots: HashMap::new(),
        }
    }

    fn add_chain(&mut self, target: &ScratchTarget, first: Option<&str>, continuation: usize) {
        let mut current = first;
        let mut chain_seen = HashSet::new();
        while let Some(id) = current {
            if !chain_seen.insert(id.to_owned()) || self.block_pcs.contains_key(id) {
                break;
            }
            let Some(block) = target.blocks.get(id) else {
                break;
            };
            let pc = self.nodes.len();
            self.block_pcs.insert(id.to_owned(), pc);
            self.nodes.push(ScriptNode::Block(id.to_owned()));
            let next = block.next.as_deref();
            self.fallthrough_pcs.insert(id.to_owned(), continuation);

            if matches!(
                block.opcode.as_str(),
                "control_repeat" | "control_repeat_until" | "control_while" | "control_forever"
            ) {
                let boundary_pc = self.nodes.len();
                self.boundary_pcs.insert(id.to_owned(), boundary_pc);
                self.nodes.push(ScriptNode::LoopBoundary(id.to_owned()));
                if block.opcode == "control_repeat" {
                    let slot = self.repeat_slots.len();
                    self.repeat_slots.insert(id.to_owned(), slot);
                }
                let branch =
                    block.inputs.get("SUBSTACK").and_then(parse_input).and_then(
                        |value| match value {
                            InputValue::BlockReference(id) => Some(id),
                            _ => None,
                        },
                    );
                self.add_chain(target, branch, boundary_pc);
            }

            if matches!(block.opcode.as_str(), "control_if" | "control_if_else") {
                let then_branch = block.inputs.get("SUBSTACK").and_then(parse_input).and_then(
                    |value| match value {
                        InputValue::BlockReference(id) => Some(id),
                        _ => None,
                    },
                );
                let else_branch = block
                    .inputs
                    .get("SUBSTACK2")
                    .and_then(parse_input)
                    .and_then(|value| match value {
                        InputValue::BlockReference(id) => Some(id),
                        _ => None,
                    });

                self.add_chain(target, next, continuation);
                let branch_continuation = next
                    .and_then(|next_id| self.block_pcs.get(next_id).copied())
                    .unwrap_or(continuation);
                self.fallthrough_pcs
                    .insert(id.to_owned(), branch_continuation);
                self.add_chain(target, then_branch, branch_continuation);
                if block.opcode == "control_if_else" {
                    self.add_chain(target, else_branch, branch_continuation);
                }
                return;
            }

            current = next;
        }
    }
}

// The whitelist admits only bounded leaf bodies. Loops, calls and every
// potentially suspending command remain on the resumable path.
fn direct_eligible(target: &ScratchTarget, procedure: &ProcedureSignature) -> bool {
    if procedure.argument_names.len() != procedure.argument_ids.len()
        || procedure.argument_names.iter().collect::<HashSet<_>>().len() != procedure.argument_names.len() { return false; }
    let Some(definition) = target.blocks.get(&procedure.definition_id) else { return false };
    let mut pending: Vec<&str> = definition.next.as_deref().into_iter().collect();
    let mut seen = HashSet::new();
    while let Some(id) = pending.pop() {
        if !seen.insert(id) || seen.len() > 80 { return false; }
        let Some(block) = target.blocks.get(id) else { return false };
        match block.opcode.as_str() {
            "control_if" | "control_if_else" => {
                for input in ["SUBSTACK", "SUBSTACK2"] {
                    if let Some(InputValue::BlockReference(branch)) = block.inputs.get(input).and_then(parse_input) { pending.push(branch); }
                }
            }
            "control_stop" if field_value(block, "STOP_OPTION").and_then(Value::as_str) == Some("this script") => {},
            "data_setvariableto" | "data_changevariableby" | "data_replaceitemoflist"
            | "motion_gotoxy" | "pen_penDown" | "pen_penUp" => {},
            _ => return false,
        }
        if let Some(next) = block.next.as_deref() { pending.push(next); }
    }
    true
}

fn direct_name(target_index: usize, frame_id: usize) -> String {
    format!("direct_procedure_{target_index}_{frame_id}")
}

pub(super) fn generate_direct_procedures(
    target: &ScratchTarget,
    target_index: usize,
    variables: &HashMap<String, VariableInfo>,
    lists: &HashMap<String, ListInfo>,
    procedures: &[ProcedureSignature],
    output: &mut String,
) {
    for procedure in procedures.iter().filter(|p| direct_eligible(target, p)) {
        let arguments = procedure.argument_names.iter().enumerate().map(|(i, name)| (name.clone(), format!("argument_{i}"))).collect::<HashMap<_, _>>();
        let params = (0..procedure.argument_ids.len()).map(|i| format!(", argument_{i}: ScratchValue")).collect::<String>();
        let name = direct_name(target_index, procedure.frame_id);
        let numeric = super::direct_numeric::generate_numeric_direct(target, procedure, variables, lists, &name, false);
        let body_name = if numeric.is_some() { format!("{name}_dynamic") } else { name };
        line(output, 0, "#[allow(dead_code, unused_variables, unreachable_code)]");
        line(output, 0, &format!("fn {body_name}(state: &mut ScratchState, clock: &dyn Clock{params}) {{"));
        let definition = &target.blocks[&procedure.definition_id];
        generate_direct_chain(target, definition.next.as_deref(), variables, lists, &arguments, output, 1);
        line(output, 0, "}\n");
        if let Some(numeric) = numeric {
            output.push_str(&numeric);
            output.push_str(&super::direct_numeric::generate_numeric_direct(target, procedure, variables, lists, &direct_name(target_index, procedure.frame_id), true).unwrap());
        }
    }
}

fn generate_direct_chain<'a>(
    target: &'a ScratchTarget, mut id: Option<&'a str>, variables: &HashMap<String, VariableInfo>,
    lists: &HashMap<String, ListInfo>, arguments: &HashMap<String, String>, output: &mut String, indent: usize,
) {
    while let Some(current) = id {
        let block = &target.blocks[current];
        match block.opcode.as_str() {
            "control_if" | "control_if_else" => {
                let condition = boolean_input(target, block, "CONDITION", variables, lists, arguments, &mut HashSet::new());
                line(output, indent, &format!("if {condition} {{"));
                let branch = |name| block.inputs.get(name).and_then(parse_input).and_then(|value| match value { InputValue::BlockReference(id) => Some(id), _ => None });
                generate_direct_chain(target, branch("SUBSTACK"), variables, lists, arguments, output, indent + 1);
                if block.opcode == "control_if_else" {
                    line(output, indent, "} else {");
                    generate_direct_chain(target, branch("SUBSTACK2"), variables, lists, arguments, output, indent + 1);
                }
                line(output, indent, "}");
            }
            "control_stop" => line(output, indent, "return;"),
            "data_setvariableto" if variable_field(block, "VARIABLE", variables).is_some_and(|variable| !variable.is_stage && !variable.numeric) => {
                let variable = variable_field(block, "VARIABLE", variables).unwrap();
                let input = block.inputs.get("VALUE");
                let value = if let Some(source) = variable_input(target, input, variables, lists, arguments) {
                    format!("({source}).clone()")
                } else {
                    format!("({}).into()", expression(target, input, variables, lists, arguments, &mut HashSet::new()))
                };
                line(output, indent, &format!("let scratch_new_value: ScratchValue = {value};"));
                line(output, indent, &format!("state.set_local_value({}, scratch_new_value);", variable.slot));
            }
            _ => generate_resumable_statement(target, block, usize::MAX, variables, lists, arguments, output, indent),
        }
        id = block.next.as_deref();
    }
}

// emit a resumable step function for one green flag script.
pub(super) fn generate_resumable_script(
    target: &ScratchTarget,
    first: Option<&str>,
    variables: &HashMap<String, VariableInfo>,
    lists: &HashMap<String, ListInfo>,
    procedures: &[ProcedureSignature],
    function_name: &str,
    target_index: usize,
    fast_math: bool,
    output: &mut String,
) -> (usize, usize) {
    let mut graph = ScriptGraph::new();
    graph.add_chain(target, first, usize::MAX);
    let start_pc = first
        .and_then(|id| graph.block_pcs.get(id).copied())
        .unwrap_or(usize::MAX);
    // Compile only bodies transitively reachable from this event script. Keep
    // the original frame IDs so calls and recursive frames retain their identity.
    let calls_in = |current: &ScriptGraph| {
        current
            .nodes
            .iter()
            .filter_map(|node| match node {
                ScriptNode::Block(id) => target.blocks.get(id),
                ScriptNode::LoopBoundary(_) => None,
            })
            .filter(|block| block.opcode == "procedures_call")
            .filter_map(|block| procedure_for_call(block, procedures))
            .map(|procedure| procedure.definition_id.clone())
            .collect::<Vec<_>>()
    };
    let mut reachable = HashSet::new();
    let mut pending = calls_in(&graph);
    while let Some(id) = pending.pop() {
        if !reachable.insert(id.clone()) {
            continue;
        }
        if let Some(definition) = target.blocks.get(&id) {
            let mut called_graph = ScriptGraph::new();
            called_graph.add_chain(target, definition.next.as_deref(), usize::MAX);
            pending.extend(calls_in(&called_graph));
        }
    }
    let mut procedure_graphs = Vec::new();
    let mut procedure_entries = HashMap::new();
    for procedure in procedures.iter().filter(|procedure| reachable.contains(&procedure.definition_id)) {
        let Some(definition) = target.blocks.get(&procedure.definition_id) else {
            continue;
        };
        let mut procedure_graph = ScriptGraph::new();
        procedure_graph.add_chain(target, definition.next.as_deref(), usize::MAX);
        let procedure_start = definition
            .next
            .as_deref()
            .and_then(|id| procedure_graph.block_pcs.get(id).copied())
            .unwrap_or(usize::MAX);
        procedure_entries.insert(
            procedure.definition_id.clone(),
            ProcedureEntry {
                function_id: procedure.frame_id,
                start_pc: procedure_start,
                repeat_count: procedure_graph.repeat_slots.len(),
                direct_name: direct_eligible(target, procedure).then(|| direct_name(target_index, procedure.frame_id)),
                numeric_direct: direct_eligible(target, procedure) && super::direct_numeric::generate_numeric_direct(target, procedure, variables, lists, &direct_name(target_index, procedure.frame_id), true).is_some(),
            },
        );
        procedure_graphs.push((procedure, procedure_graph));
    }

    line(
        output,
        0,
        "#[allow(dead_code, unused_variables, unused_mut, unused_assignments, unreachable_code)]",
    );
    line(
        output,
        0,
        &format!(
            "fn step_{function_name}(script: &mut ScriptState, state: &mut ScratchState, clock: &dyn Clock) -> ScriptStatus {{"
        ),
    );
    line(output, 1, "loop {");
    line(
        output,
        2,
        "let (function, pc) = match script.frames.last() {",
    );
    line(output, 3, "Some(frame) => (frame.function, frame.pc),");
    line(output, 3, "None => return ScriptStatus::Complete,");
    line(output, 2, "};");
    line(output, 2, "match function {");
    line(output, 3, "0 => match pc {");
    let mut emitted_blocks = HashSet::new();
    for node in &graph.nodes {
        match node {
            ScriptNode::Block(id) => generate_resumable_block_chain(
                target,
                id,
                &graph,
                variables,
                lists,
                &HashMap::new(),
                procedures,
                &procedure_entries,
                &mut emitted_blocks,
                fast_math,
                output,
            ),
            ScriptNode::LoopBoundary(id) => {
                if target
                    .blocks
                    .get(id)
                    .and_then(|block| numeric_loop_plan(target, block, variables))
                    .is_none()
                {
                    generate_loop_boundary(target, id, &graph, output);
                }
            }
        }
    }
    line(output, 4, "_ => return ScriptStatus::Complete,");
    line(output, 3, "},");

    for (procedure, procedure_graph) in &procedure_graphs {
        let argument_accesses = procedure
            .argument_names
            .iter()
            .enumerate()
            .map(|(index, name)| {
                (
                    name.clone(),
                    format!("script.frames.last().unwrap().arguments[{index}]"),
                )
            })
            .collect::<HashMap<_, _>>();
        line(output, 3, &format!("{} => match pc {{", procedure.frame_id));
        let mut emitted_blocks = HashSet::new();
        for node in &procedure_graph.nodes {
            match node {
                ScriptNode::Block(id) => generate_resumable_block_chain(
                    target,
                    id,
                    procedure_graph,
                    variables,
                    lists,
                    &argument_accesses,
                    procedures,
                    &procedure_entries,
                    &mut emitted_blocks,
                    fast_math,
                    output,
                ),
                ScriptNode::LoopBoundary(id) => {
                    if target
                        .blocks
                        .get(id)
                        .and_then(|block| numeric_loop_plan(target, block, variables))
                        .is_none()
                    {
                        generate_loop_boundary(target, id, procedure_graph, output)
                    }
                }
            }
        }
        line(output, 4, "_ => { script.frames.pop(); continue; },");
        line(output, 3, "},");
    }
    line(output, 3, "_ => return ScriptStatus::Complete,");
    line(output, 2, "}");
    line(output, 1, "}");
    line(output, 0, "}\n");
    (start_pc, graph.repeat_slots.len())
}

fn generate_resumable_block(
    target: &ScratchTarget,
    id: &str,
    graph: &ScriptGraph,
    variables: &HashMap<String, VariableInfo>,
    lists: &HashMap<String, ListInfo>,
    arguments: &HashMap<String, String>,
    procedures: &[ProcedureSignature],
    procedure_entries: &HashMap<String, ProcedureEntry>,
    output: &mut String,
) {
    let pc = graph.block_pcs[id];
    let indent = if arguments.is_empty() { 4 } else { 4 };
    line(output, indent, &format!("{pc} => {{"));
    let Some(block) = target.blocks.get(id) else {
        line(output, indent + 1, "script.frames.pop(); continue;");
        line(output, indent, "},");
        return;
    };
    let next_pc = block
        .next
        .as_ref()
        .and_then(|next| graph.block_pcs.get(next).copied())
        .unwrap_or(graph.fallthrough_pcs[id]);
    match block.opcode.as_str() {
        "control_if" | "control_if_else" => {
            let condition = boolean_input(
                target,
                block,
                "CONDITION",
                variables,
                lists,
                arguments,
                &mut HashSet::new(),
            );
            let branch_pc = |input_name: &str| {
                block
                    .inputs
                    .get(input_name)
                    .and_then(parse_input)
                    .and_then(|input| match input {
                        InputValue::BlockReference(id) => graph.block_pcs.get(id).copied(),
                        _ => None,
                    })
                    .unwrap_or(next_pc)
            };
            line(output, indent + 1, &format!("if {condition} {{"));
            line(
                output,
                indent + 2,
                &format!(
                    "script.frames.last_mut().unwrap().pc = {};",
                    branch_pc("SUBSTACK")
                ),
            );
            line(output, indent + 1, "} else {");
            line(
                output,
                indent + 2,
                &format!(
                    "script.frames.last_mut().unwrap().pc = {};",
                    if block.opcode == "control_if_else" {
                        branch_pc("SUBSTACK2")
                    } else {
                        next_pc
                    }
                ),
            );
            line(output, indent + 1, "}");
            line(output, indent + 1, "continue;");
        }
        "control_repeat" => {
            let times = expression(
                target,
                block.inputs.get("TIMES"),
                variables,
                lists,
                arguments,
                &mut HashSet::new(),
            );
            let slot = graph.repeat_slots[id];
            let boundary = graph.boundary_pcs[id];
            let branch_pc = block
                .inputs
                .get("SUBSTACK")
                .and_then(parse_input)
                .and_then(|input| match input {
                    InputValue::BlockReference(id) => graph.block_pcs.get(id).copied(),
                    _ => None,
                });
            let fused = generate_warp_repeat_list_append(
                target, block, graph, variables, lists, arguments, &times, slot, next_pc, output,
                indent,
            );
            if !fused {
                line(
                    output,
                    4,
                    &format!("let repeat_count = scratch_round({times}).max(0.0) as usize;"),
                );
                line(
                    output,
                    4,
                    &format!(
                        "script.frames.last_mut().unwrap().repeat_counts[{slot}] = repeat_count;"
                    ),
                );
                line(output, indent + 1, "if repeat_count == 0 {");
                line(
                    output,
                    indent + 2,
                    &format!(
                        "script.frames.last_mut().unwrap().pc = {};",
                        pc_literal(next_pc)
                    ),
                );
                line(output, indent + 1, "} else {");
                line(
                    output,
                    indent + 2,
                    &format!(
                        "script.frames.last_mut().unwrap().pc = {};",
                        branch_pc.unwrap_or(boundary)
                    ),
                );
                if branch_pc.is_none() {
                    line(output, indent + 2, "continue;");
                }
                line(output, indent + 1, "}");
                line(output, indent + 1, "continue;");
            }
        }
        "control_repeat_until" | "control_while" => {
            let condition = boolean_input(
                target,
                block,
                "CONDITION",
                variables,
                lists,
                arguments,
                &mut HashSet::new(),
            );
            let continue_when = if block.opcode == "control_repeat_until" {
                format!("!({condition})")
            } else {
                condition
            };
            let branch_pc = block
                .inputs
                .get("SUBSTACK")
                .and_then(parse_input)
                .and_then(|input| match input {
                    InputValue::BlockReference(id) => graph.block_pcs.get(id).copied(),
                    _ => None,
                });
            line(output, indent + 1, &format!("if {continue_when} {{"));
            if let Some(branch_pc) = branch_pc {
                line(
                    output,
                    indent + 2,
                    &format!("script.frames.last_mut().unwrap().pc = {branch_pc};"),
                );
            } else {
                line(
                    output,
                    indent + 2,
                    &format!(
                        "script.frames.last_mut().unwrap().pc = {};",
                        graph.boundary_pcs[id]
                    ),
                );
            }
            line(output, indent + 2, "continue;");
            line(output, indent + 1, "}");
            line(
                output,
                indent + 2,
                &format!(
                    "script.frames.last_mut().unwrap().pc = {};",
                    pc_literal(next_pc)
                ),
            );
            line(output, indent + 1, "continue;");
        }
        "control_forever" => {
            let branch_pc = block
                .inputs
                .get("SUBSTACK")
                .and_then(parse_input)
                .and_then(|input| match input {
                    InputValue::BlockReference(id) => graph.block_pcs.get(id).copied(),
                    _ => None,
                })
                .unwrap_or(graph.boundary_pcs[id]);
            line(
                output,
                indent + 1,
                &format!("script.frames.last_mut().unwrap().pc = {branch_pc};"),
            );
            line(output, indent + 1, "continue;");
        }
        "control_wait_until" => {
            let condition = boolean_input(
                target,
                block,
                "CONDITION",
                variables,
                lists,
                arguments,
                &mut HashSet::new(),
            );
            line(output, indent + 1, &format!("if !({condition}) {{"));
            line(
                output,
                indent + 2,
                &format!("script.frames.last_mut().unwrap().pc = {pc};"),
            );
            line(
                output,
                indent + 2,
                "if script.yield_at_safe_point(clock) { return ScriptStatus::Yield; }",
            );
            line(output, indent + 2, "continue;");
            line(output, indent + 1, "}");
            line(
                output,
                indent + 1,
                &format!(
                    "script.frames.last_mut().unwrap().pc = {};",
                    pc_literal(next_pc)
                ),
            );
            line(output, indent + 1, "continue;");
        }
        "control_wait" => {
            let duration = expression(
                target,
                block.inputs.get("DURATION"),
                variables,
                lists,
                arguments,
                &mut HashSet::new(),
            );
            line(
                output,
                indent + 1,
                &format!(
                    "let duration_ms = (scratch_to_number({duration}).max(0.0) * 1000.0).ceil() as u64; let wait_deadline = clock.now_millis().saturating_add(duration_ms);"
                ),
            );
            line(
                output,
                indent + 1,
                &format!(
                    "script.frames.last_mut().unwrap().pc = {};",
                    pc_literal(next_pc)
                ),
            );
            line(
                output,
                indent + 1,
                "return ScriptStatus::WaitUntil(wait_deadline);",
            );
        }
        "sound_playuntildone" => {
            let selector = owned_input(
                target,
                block.inputs.get("SOUND_MENU"),
                variables,
                lists,
                arguments,
            );
            line(
                output,
                indent + 1,
                &format!("let scratch_sound_selector: ScratchValue = {selector};"),
            );
            line(
                output,
                indent + 1,
                "let sound_wait = state.start_sound(scratch_sound_selector, clock.now_millis());",
            );
            line(
                output,
                indent + 1,
                &format!(
                    "script.frames.last_mut().unwrap().pc = {};",
                    pc_literal(next_pc)
                ),
            );
            line(
                output,
                indent + 1,
                "if let Some(sound_wait) = sound_wait { return ScriptStatus::WaitForSignal(sound_wait); }",
            );
            line(output, indent + 1, "continue;");
        }
        "sound_seteffectto" | "sound_changeeffectby" => {
            let effect = field_value(block, "EFFECT")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_ascii_lowercase();
            let effect = match effect.as_str() {
                "pitch" => "Some(ScratchSoundEffect::Pitch)",
                "pan" => "Some(ScratchSoundEffect::Pan)",
                _ => "None",
            };
            let value = expression(
                target,
                block.inputs.get("VALUE"),
                variables,
                lists,
                arguments,
                &mut HashSet::new(),
            );
            line(
                output,
                indent + 1,
                &format!("if let Some(scratch_sound_effect) = {effect} {{"),
            );
            line(
                output,
                indent + 2,
                &format!(
                    "state.set_sound_effect(scratch_sound_effect, scratch_to_number({value}), {}, clock.now_millis());",
                    block.opcode == "sound_changeeffectby"
                ),
            );
            line(
                output,
                indent + 2,
                &format!(
                    "script.frames.last_mut().unwrap().pc = {};",
                    pc_literal(next_pc)
                ),
            );
            line(output, indent + 2, "return ScriptStatus::Yield;");
            line(output, indent + 1, "} else {");
            line(
                output,
                indent + 2,
                &format!(
                    "script.frames.last_mut().unwrap().pc = {};",
                    pc_literal(next_pc)
                ),
            );
            line(output, indent + 2, "continue;");
            line(output, indent + 1, "}");
        }
        "sound_setvolumeto" | "sound_changevolumeby" => {
            let value = expression(
                target,
                block.inputs.get("VOLUME"),
                variables,
                lists,
                arguments,
                &mut HashSet::new(),
            );
            line(
                output,
                indent + 1,
                &format!(
                    "state.set_sound_volume(scratch_to_number({value}), {}, clock.now_millis());",
                    block.opcode == "sound_changevolumeby"
                ),
            );
            line(
                output,
                indent + 1,
                "if REFRESH_SOUND_VOLUME_MONITOR { state.request_presentation(); }",
            );
            line(
                output,
                indent + 1,
                &format!(
                    "script.frames.last_mut().unwrap().pc = {};",
                    pc_literal(next_pc)
                ),
            );
            line(output, indent + 1, "return ScriptStatus::Yield;");
        }
        "looks_switchbackdropto" | "looks_nextbackdrop" => {
            generate_resumable_statement(
                target,
                block,
                next_pc,
                variables,
                lists,
                arguments,
                output,
                indent + 1,
            );
            // redraw ends the scheduler round after runnable scripts finish their turn.
            line(
                output,
                indent + 1,
                &format!(
                    "script.frames.last_mut().unwrap().pc = {};",
                    pc_literal(next_pc)
                ),
            );
            line(output, indent + 1, "continue;");
        }
        "motion_glideto" | "motion_glidesecstoxy" => generate_resumable_statement(
            target,
            block,
            next_pc,
            variables,
            lists,
            arguments,
            output,
            indent + 1,
        ),
        "procedures_call" => generate_resumable_procedure_call(
            target,
            block,
            next_pc,
            variables,
            lists,
            arguments,
            procedures,
            procedure_entries,
            output,
            indent + 1,
            true,
            None,
            None,
        ),
        _ => {
            generate_resumable_statement(
                target,
                block,
                next_pc,
                variables,
                lists,
                arguments,
                output,
                indent + 1,
            );
            line(
                output,
                indent + 1,
                &format!(
                    "script.frames.last_mut().unwrap().pc = {};",
                    pc_literal(next_pc)
                ),
            );
            line(output, indent + 1, "continue;");
        }
    }
    line(output, indent, "},");
}

// fuse a warp repeat whose only body action appends to one list, checking the watchdog by chunk.
#[allow(clippy::too_many_arguments)]
fn generate_warp_repeat_list_append(
    target: &ScratchTarget,
    repeat: &ScratchBlock,
    graph: &ScriptGraph,
    variables: &HashMap<String, VariableInfo>,
    lists: &HashMap<String, ListInfo>,
    arguments: &HashMap<String, String>,
    times: &str,
    repeat_slot: usize,
    next_pc: usize,
    output: &mut String,
    indent: usize,
) -> bool {
    let Some(InputValue::BlockReference(body_id)) =
        repeat.inputs.get("SUBSTACK").and_then(parse_input)
    else {
        return false;
    };
    let Some(body) = target.blocks.get(body_id) else {
        return false;
    };
    if body.opcode != "data_addtolist" || body.next.is_some() {
        return false;
    }
    let Some(list) = list_field(body, "LIST", lists) else {
        return false;
    };
    let Some(body_pc) = graph.block_pcs.get(body_id).copied() else {
        return false;
    };
    let item = owned_input(target, body.inputs.get("ITEM"), variables, lists, arguments);
    let item_reads_state = item.contains("state.");
    let numeric_random =
        body.inputs
            .get("ITEM")
            .and_then(parse_input)
            .and_then(|input| match input {
                InputValue::BlockReference(id) => target
                    .blocks
                    .get(id)
                    .and_then(|block| constant_random_bounds(target, block)),
                _ => None,
            });

    line(
        output,
        indent + 1,
        "if script.frames.last().unwrap().warp_mode {",
    );
    line(
        output,
        indent + 2,
        &format!("let repeat_count = scratch_round({times}).max(0.0) as usize;"),
    );
    line(
        output,
        indent + 2,
        &format!("script.frames.last_mut().unwrap().repeat_counts[{repeat_slot}] = repeat_count;"),
    );
    line(
        output,
        indent + 2,
        "let mut repeat_remaining = repeat_count;",
    );
    line(output, indent + 2, "while repeat_remaining > 0 {");
    line(
        output,
        indent + 3,
        "let repeat_chunk = repeat_remaining.min(1024);",
    );
    line(output, indent + 3, "let mut scratch_list_changed = false;");
    if item_reads_state {
        line(output, indent + 3, "for _ in 0..repeat_chunk {");
        line(
            output,
            indent + 4,
            &format!("let scratch_list_item: ScratchValue = {item};"),
        );
        line(
            output,
            indent + 4,
            &format!(
                "scratch_list_changed |= state.list_mut({}).add(scratch_list_item);",
                list.slot
            ),
        );
        line(output, indent + 3, "}");
    } else if let Some((from, to, integer_bounds)) = numeric_random {
        line(
            output,
            indent + 3,
            &format!(
                "scratch_list_changed = with_scratch_random(|scratch_random| state.list_mut({}).add_generated_numbers(repeat_chunk, || scratch_random.between_with_integer_bounds({from:?}_f64, {to:?}_f64, {integer_bounds})));",
                list.slot
            ),
        );
    } else {
        line(
            output,
            indent + 3,
            &format!(
                "scratch_list_changed = state.list_mut({}).add_generated(repeat_chunk, || {{ let scratch_list_item: ScratchValue = {item}; scratch_list_item }});",
                list.slot
            ),
        );
    }
    line(output, indent + 3, "repeat_remaining -= repeat_chunk;");
    line(
        output,
        indent + 3,
        &format!(
            "script.frames.last_mut().unwrap().repeat_counts[{repeat_slot}] = repeat_remaining;"
        ),
    );
    line(
        output,
        indent + 3,
        &format!("state.list_changed_result({}, scratch_list_changed);", list.slot),
    );
    line(
        output,
        indent + 3,
        "if script.warp_budget.expired_now(clock) {",
    );
    line(
        output,
        indent + 4,
        &format!(
            "script.frames.last_mut().unwrap().pc = if repeat_remaining > 0 {{ {body_pc} }} else {{ {} }};",
            pc_literal(next_pc)
        ),
    );
    line(output, indent + 4, "return ScriptStatus::Yield;");
    line(output, indent + 3, "}");
    line(output, indent + 2, "}");
    line(
        output,
        indent + 2,
        &format!(
            "script.frames.last_mut().unwrap().pc = {};",
            pc_literal(next_pc)
        ),
    );
    line(output, indent + 2, "continue;");
    line(output, indent + 1, "} else {");
    line(
        output,
        indent + 2,
        &format!("let repeat_count = scratch_round({times}).max(0.0) as usize;"),
    );
    line(
        output,
        indent + 2,
        &format!("script.frames.last_mut().unwrap().repeat_counts[{repeat_slot}] = repeat_count;"),
    );
    line(output, indent + 2, "if repeat_count == 0 {");
    line(
        output,
        indent + 3,
        &format!(
            "script.frames.last_mut().unwrap().pc = {};",
            pc_literal(next_pc)
        ),
    );
    line(output, indent + 2, "} else {");
    line(
        output,
        indent + 3,
        &format!("script.frames.last_mut().unwrap().pc = {body_pc};"),
    );
    line(output, indent + 2, "}");
    line(output, indent + 2, "continue;");
    line(output, indent + 1, "}");
    true
}

// A loop body containing only synchronous statements and direct leaf calls
// can execute as structured Rust between the existing loop safe points. Each
// call still records its own continuation before a possible budget yield.
fn structured_loop_body(
    target: &ScratchTarget,
    first: &str,
    procedures: &[ProcedureSignature],
    entries: &HashMap<String, ProcedureEntry>,
) -> bool {
    let mut pending = vec![first];
    let mut seen = HashSet::new();
    let mut has_direct_call = false;
    while let Some(id) = pending.pop() {
        if !seen.insert(id) || seen.len() > 80 { return false; }
        let Some(block) = target.blocks.get(id) else { return false };
        match block.opcode.as_str() {
            "control_if" | "control_if_else" => {
                for name in ["SUBSTACK", "SUBSTACK2"] {
                    if let Some(InputValue::BlockReference(branch)) = block.inputs.get(name).and_then(parse_input) { pending.push(branch); }
                }
            }
            "procedures_call" => {
                let Some(procedure) = procedure_for_call(block, procedures) else { return false };
                if !entries.get(&procedure.definition_id).is_some_and(|entry| entry.direct_name.is_some()) { return false; }
                has_direct_call = true;
            }
            "data_setvariableto" | "data_changevariableby" | "data_replaceitemoflist"
            | "motion_gotoxy" | "pen_penDown" | "pen_penUp" => {},
            _ => return false,
        }
        if let Some(next) = block.next.as_deref() { pending.push(next); }
    }
    has_direct_call
}

#[allow(clippy::too_many_arguments)]
#[derive(Clone)]
struct CachedNumericItem {
    name: String,
    list_slot: usize,
    index_slots: HashSet<usize>,
}

fn emit_structured_loop_chain<'a>(
    target: &'a ScratchTarget, mut current: Option<&'a str>, graph: &ScriptGraph,
    variables: &HashMap<String, VariableInfo>, lists: &HashMap<String, ListInfo>,
    arguments: &HashMap<String, String>, procedures: &[ProcedureSignature],
    entries: &HashMap<String, ProcedureEntry>, output: &mut String, indent: usize,
    cached_indexes: &mut HashSet<usize>,
    cached_items: &mut HashMap<String, CachedNumericItem>,
) {
    while let Some(id) = current {
        let block = &target.blocks[id];
        let next_pc = block.next.as_ref().and_then(|next| graph.block_pcs.get(next).copied()).unwrap_or(graph.fallthrough_pcs[id]);
        match block.opcode.as_str() {
            "control_if" | "control_if_else" => {
                let condition = boolean_input(target, block, "CONDITION", variables, lists, arguments, &mut HashSet::new());
                line(output, indent, &format!("if {condition} {{"));
                let branch = |name| block.inputs.get(name).and_then(parse_input).and_then(|value| match value { InputValue::BlockReference(id) => Some(id), _ => None });
                let mut yes = cached_indexes.clone();
                let mut yes_items = cached_items.clone();
                emit_structured_loop_chain(target, branch("SUBSTACK"), graph, variables, lists, arguments, procedures, entries, output, indent + 1, &mut yes, &mut yes_items);
                let mut no = cached_indexes.clone();
                let mut no_items = cached_items.clone();
                if block.opcode == "control_if_else" {
                    line(output, indent, "} else {");
                    emit_structured_loop_chain(target, branch("SUBSTACK2"), graph, variables, lists, arguments, procedures, entries, output, indent + 1, &mut no, &mut no_items);
                }
                cached_indexes.retain(|slot| yes.contains(slot) && no.contains(slot));
                cached_items.retain(|key, _| yes_items.contains_key(key) && no_items.contains_key(key));
                line(output, indent, "}");
            }
            "procedures_call" => {
                let procedure = procedure_for_call(block, procedures).unwrap();
                let entry = &entries[&procedure.definition_id];
                let (_, slots) = cached_procedure_indexes(target, block, procedure, entry, variables);
                generate_resumable_procedure_call(target, block, next_pc, variables, lists, arguments, procedures, entries, output, indent, false, Some(cached_indexes), Some(cached_items));
                cached_indexes.extend(slots);
                let definition = &target.blocks[&procedure.definition_id];
                let mut body = ScriptGraph::new();
                body.add_chain(target, definition.next.as_deref(), usize::MAX);
                for node in body.nodes {
                    if let ScriptNode::Block(body_id) = node {
                        let statement = &target.blocks[&body_id];
                        if matches!(statement.opcode.as_str(), "data_setvariableto" | "data_changevariableby") {
                            if let Some(variable) = variable_field(statement, "VARIABLE", variables) { cached_indexes.remove(&variable.slot); cached_items.retain(|_, item| !item.index_slots.contains(&variable.slot)); }
                        }
                        if matches!(statement.opcode.as_str(), "data_addtolist" | "data_deleteoflist" | "data_deletealloflist" | "data_insertatlist" | "data_replaceitemoflist") {
                            if let Some(list) = list_field(statement, "LIST", lists) { cached_items.retain(|_, item| item.list_slot != list.slot); } else { cached_items.clear(); }
                        }
                    }
                }
            }
            "data_setvariableto" => {
                if let Some(slot) = emit_shared_list_copy(target, block, variables, lists, arguments, output, indent) {
                    cached_items.retain(|_, item| !item.index_slots.contains(&slot));
                    cached_indexes.insert(slot);
                } else {
                    generate_resumable_statement(target, block, next_pc, variables, lists, arguments, output, indent);
                    if let Some(variable) = variable_field(block, "VARIABLE", variables) { cached_indexes.remove(&variable.slot); cached_items.retain(|_, item| !item.index_slots.contains(&variable.slot)); }
                }
            }
            _ => {
                generate_resumable_statement(target, block, next_pc, variables, lists, arguments, output, indent);
                if block.opcode == "data_changevariableby" {
                    if let Some(variable) = variable_field(block, "VARIABLE", variables) { cached_indexes.remove(&variable.slot); cached_items.retain(|_, item| !item.index_slots.contains(&variable.slot)); }
                }
                if block.opcode == "data_replaceitemoflist" {
                    if let Some(list) = list_field(block, "LIST", lists) { cached_items.retain(|_, item| item.list_slot != list.slot); } else { cached_items.clear(); }
                }
            },
        }
        current = block.next.as_deref();
    }
}

fn emit_shared_list_copy(
    target: &ScratchTarget, block: &ScratchBlock, variables: &HashMap<String, VariableInfo>,
    lists: &HashMap<String, ListInfo>, arguments: &HashMap<String, String>, output: &mut String, indent: usize,
) -> Option<usize> {
    let Some(variable) = variable_field(block, "VARIABLE", variables) else { return None };
    if variable.is_stage || variable.target != target.name || variable.slot < lists.values().map(|list| list.slot + 1).max().unwrap_or(0) { return None; }
    let Some(InputValue::BlockReference(id)) = block.inputs.get("VALUE").and_then(parse_input) else { return None };
    let Some(reporter) = target.blocks.get(id) else { return None };
    if reporter.opcode != "data_itemoflist" { return None; }
    let Some(list) = list_field(reporter, "LIST", lists) else { return None };
    if !list.is_stage { return None; }
    let index = expression(target, reporter.inputs.get("INDEX"), variables, lists, arguments, &mut HashSet::new());
    line(output, indent, &format!("let scratch_list_index: ScratchValue = ({index}).to_owned().into();"));
    line(output, indent, &format!("let cached_index_{} = state.set_local_from_shared_list::<{}>({}, {}, scratch_list_index);", variable.slot, list.cache_numeric_reads, variable.slot, list.slot));
    Some(variable.slot)
}

fn generate_resumable_block_chain(
    target: &ScratchTarget,
    id: &str,
    graph: &ScriptGraph,
    variables: &HashMap<String, VariableInfo>,
    lists: &HashMap<String, ListInfo>,
    arguments: &HashMap<String, String>,
    procedures: &[ProcedureSignature],
    procedure_entries: &HashMap<String, ProcedureEntry>,
    emitted_blocks: &mut HashSet<String>,
    fast_math: bool,
    output: &mut String,
) {
    if emitted_blocks.contains(id) {
        return;
    }
    let Some(block) = target.blocks.get(id) else {
        generate_resumable_block(
            target,
            id,
            graph,
            variables,
            lists,
            arguments,
            procedures,
            procedure_entries,
            output,
        );
        return;
    };
    let enclosing_loop = graph.nodes.iter().find_map(|node| match node {
        ScriptNode::Block(loop_id) => target.blocks.get(loop_id).and_then(|candidate| {
            (matches!(candidate.opcode.as_str(), "control_repeat" | "control_repeat_until" | "control_while" | "control_forever")
                && matches!(candidate.inputs.get("SUBSTACK").and_then(parse_input), Some(InputValue::BlockReference(body)) if body == id))
                .then_some((loop_id.as_str(), candidate))
        }),
        ScriptNode::LoopBoundary(_) => None,
    });
    if let Some((loop_id, enclosing)) = enclosing_loop.filter(|_| structured_loop_body(target, id, procedures, procedure_entries)) {
        emitted_blocks.insert(id.to_owned());
        line(output, 4, &format!("{} => {{", graph.block_pcs[id]));
        let repeat = enclosing.opcode == "control_repeat";
        if repeat { line(output, 5, "loop {"); }
        let body_indent = if repeat { 6 } else { 5 };
        emit_structured_loop_chain(target, Some(id), graph, variables, lists, arguments, procedures, procedure_entries, output, body_indent, &mut HashSet::new(), &mut HashMap::new());
        let mut last = id;
        while let Some(next) = target.blocks[last].next.as_deref() { last = next; }
        if repeat {
            let repeat_slot = graph.repeat_slots[loop_id];
            let next_pc = enclosing.next.as_ref().and_then(|next| graph.block_pcs.get(next).copied()).unwrap_or(graph.fallthrough_pcs[loop_id]);
            line(output, 6, &format!("let repeat_remaining = script.frames.last().unwrap().repeat_counts[{repeat_slot}].saturating_sub(1);"));
            line(output, 6, &format!("script.frames.last_mut().unwrap().repeat_counts[{repeat_slot}] = repeat_remaining;"));
            line(output, 6, &format!("script.frames.last_mut().unwrap().pc = if repeat_remaining > 0 {{ {} }} else {{ {} }};", graph.block_pcs[id], pc_literal(next_pc)));
            line(output, 6, "if script.yield_at_safe_point(clock) { return ScriptStatus::Yield; }");
            line(output, 6, "if repeat_remaining == 0 { break; }");
            line(output, 5, "}");
        } else {
            line(output, 5, &format!("script.frames.last_mut().unwrap().pc = {};", pc_literal(graph.fallthrough_pcs[last])));
        }
        line(output, 5, "continue;");
        line(output, 4, "},");
        return;
    }
    if let Some(plan) = fused_repeat_plan(target, block, variables) {
        emitted_blocks.insert(id.to_owned());
        emitted_blocks.extend(plan.blocks.iter().cloned());
        generate_fused_repeat_loop(
            target,
            id,
            block,
            graph,
            variables,
            lists,
            arguments,
            &plan,
            output,
        );
        return;
    }
    if let Some(plan) = numeric_loop_plan(target, block, variables) {
        emitted_blocks.insert(id.to_owned());
        emitted_blocks.extend(plan.body.iter().cloned());
        generate_native_numeric_loop(
            target, id, block, graph, variables, &plan, fast_math, output,
        );
        return;
    }
    if !is_straight_line_block(block) {
        emitted_blocks.insert(id.to_owned());
        generate_resumable_block(
            target,
            id,
            graph,
            variables,
            lists,
            arguments,
            procedures,
            procedure_entries,
            output,
        );
        return;
    }

    let start_pc = graph.block_pcs[id];
    line(output, 4, &format!("{start_pc} => {{"));
    let mut current = Some(id);
    let mut last = id;
    while let Some(current_id) = current {
        if !emitted_blocks.insert(current_id.to_owned()) {
            break;
        }
        let Some(current_block) = target.blocks.get(current_id) else {
            break;
        };
        generate_resumable_statement(
            target,
            current_block,
            current_block
                .next
                .as_ref()
                .and_then(|next| graph.block_pcs.get(next).copied())
                .unwrap_or(graph.fallthrough_pcs[current_id]),
            variables,
            lists,
            arguments,
            output,
            5,
        );
        last = current_id;
        current = current_block.next.as_deref().filter(|next_id| {
            graph.block_pcs.contains_key(*next_id)
                && target
                    .blocks
                    .get(*next_id)
                    .is_some_and(is_straight_line_block)
        });
    }
    let last_block = &target.blocks[last];
    let next_pc = last_block
        .next
        .as_ref()
        .and_then(|next| graph.block_pcs.get(next).copied())
        .unwrap_or(graph.fallthrough_pcs[last]);
    line(
        output,
        5,
        &format!(
            "script.frames.last_mut().unwrap().pc = {};",
            pc_literal(next_pc)
        ),
    );
    line(output, 5, "continue;");
    line(output, 4, "},");
}

struct FusedRepeatPlan {
    body: Vec<String>,
    blocks: Vec<String>,
    local_random: bool,
}

// fuse synchronous repeat bodies to avoid one program-counter dispatch per block.
fn fused_repeat_plan(
    target: &ScratchTarget,
    block: &ScratchBlock,
    variables: &HashMap<String, VariableInfo>,
) -> Option<FusedRepeatPlan> {
    if block.opcode != "control_repeat" {
        return None;
    }
    let mut body = Vec::new();
    let mut blocks = Vec::new();
    let mut seen = HashSet::new();
    let mut current = block
        .inputs
        .get("SUBSTACK")
        .and_then(parse_input)
        .and_then(|input| match input {
            InputValue::BlockReference(id) => Some(id.to_owned()),
            _ => None,
        });
    while let Some(id) = current {
        if !seen.insert(id.clone()) {
            return None;
        }
        let statement = target.blocks.get(&id)?;
        match statement.opcode.as_str() {
            "data_setvariableto" | "data_changevariableby" | "data_replaceitemoflist" => {}
            "control_if" => {
                if !fused_repeat_branch(target, statement)? {
                    return None;
                }
                let mut branch = statement
                    .inputs
                    .get("SUBSTACK")
                    .and_then(parse_input)
                    .and_then(|input| match input {
                        InputValue::BlockReference(id) => Some(id.to_owned()),
                        _ => None,
                    });
                while let Some(branch_id) = branch {
                    if !seen.insert(branch_id.clone()) {
                        return None;
                    }
                    let branch_block = target.blocks.get(&branch_id)?;
                    blocks.push(branch_id);
                    branch = branch_block.next.clone();
                }
            }
            _ => return None,
        }
        blocks.push(id.clone());
        body.push(id.clone());
        current = statement.next.clone();
    }
    if body.is_empty() {
        return None;
    }
    let local_random = blocks.iter().any(|id| {
        target.blocks.get(id).is_some_and(|statement| {
            statement.inputs.values().any(|input| {
                reporter_contains_opcode(target, input, "operator_random", &mut HashSet::new())
            })
        })
    });
    if local_random
        && blocks.iter().any(|id| {
            target.blocks.get(id).is_some_and(|statement| {
                !fused_random_inputs_safe(target, statement, variables)
            })
        })
    {
        return None;
    }
    if local_random && !fused_random_assignment_order_safe(target, &body, variables) {
        return None;
    }
    Some(FusedRepeatPlan {
        body,
        blocks,
        local_random,
    })
}

// ensure random-borrowed list operations see values written numerically earlier in the loop.
fn fused_random_assignment_order_safe(
    target: &ScratchTarget,
    body: &[String],
    variables: &HashMap<String, VariableInfo>,
) -> bool {
    let mut numeric_slots = HashSet::new();
    for id in body {
        let Some(statement) = target.blocks.get(id) else {
            return false;
        };
        match statement.opcode.as_str() {
            "data_setvariableto" => {
                let Some(variable) = variable_field(statement, "VARIABLE", variables) else {
                    return false;
                };
                let Some(value) = statement.inputs.get("VALUE") else {
                    return false;
                };
                if variable.numeric
                    && reporter_contains_opcode(
                        target,
                        value,
                        "operator_random",
                        &mut HashSet::new(),
                    )
                {
                    numeric_slots.insert(variable.slot);
                }
            }
            "data_changevariableby" => {
                let Some(variable) = variable_field(statement, "VARIABLE", variables) else {
                    return false;
                };
                numeric_slots.insert(variable.slot);
            }
            "data_replaceitemoflist" => {
                for input in ["INDEX", "ITEM"] {
                    let Some(variable) =
                        variable_input_info(target, statement.inputs.get(input), variables)
                    else {
                        return false;
                    };
                    if !variable.numeric || !numeric_slots.contains(&variable.slot) {
                        return false;
                    }
                }
            }
            "control_if" => {}
            _ => return false,
        }
    }
    true
}

// locate reporter opcodes through connected inputs without changing their evaluation order.
fn reporter_contains_opcode(
    target: &ScratchTarget,
    input: &Value,
    opcode: &str,
    active: &mut HashSet<String>,
) -> bool {
    let Some(InputValue::BlockReference(id)) = parse_input(input) else {
        return false;
    };
    if !active.insert(id.to_owned()) {
        return false;
    }
    let found = target.blocks.get(id).is_some_and(|block| {
        block.opcode == opcode
            || block
                .inputs
                .values()
                .any(|nested| reporter_contains_opcode(target, nested, opcode, active))
    });
    active.remove(id);
    found
}

// keep local random borrowing away from list selectors that can draw randomness internally.
fn fused_random_inputs_safe(
    target: &ScratchTarget,
    statement: &ScratchBlock,
    variables: &HashMap<String, VariableInfo>,
) -> bool {
    if statement.opcode == "data_replaceitemoflist"
        && !variable_input_info(target, statement.inputs.get("INDEX"), variables)
            .zip(variable_input_info(target, statement.inputs.get("ITEM"), variables))
            .is_some_and(|(index, item)| index.numeric && item.numeric)
    {
        return false;
    }
    for input in statement.inputs.values() {
        if !fused_random_input_safe(target, input, variables, &mut HashSet::new()) {
            return false;
        }
    }
    true
}

fn fused_random_input_safe(
    target: &ScratchTarget,
    input: &Value,
    variables: &HashMap<String, VariableInfo>,
    active: &mut HashSet<String>,
) -> bool {
    let Some(InputValue::BlockReference(id)) = parse_input(input) else {
        return true;
    };
    if !active.insert(id.to_owned()) {
        return false;
    }
    let safe = target.blocks.get(id).is_some_and(|block| {
        let list_index_is_dynamic = block.opcode == "data_itemoflist"
            && !variable_input_info(target, block.inputs.get("INDEX"), variables)
                .is_some_and(|index| index.numeric);
        !list_index_is_dynamic
            && block
                .inputs
                .values()
                .all(|nested| fused_random_input_safe(target, nested, variables, active))
    });
    active.remove(id);
    safe
}

// permit only non-suspending display and stop blocks in an inline conditional branch.
fn fused_repeat_branch(target: &ScratchTarget, block: &ScratchBlock) -> Option<bool> {
    if block.inputs.get("CONDITION").is_none() {
        return Some(false);
    }
    let mut current = block
        .inputs
        .get("SUBSTACK")
        .and_then(parse_input)
        .and_then(|input| match input {
            InputValue::BlockReference(id) => Some(id),
            _ => None,
        });
    while let Some(id) = current {
        let nested = target.blocks.get(id)?;
        match nested.opcode.as_str() {
            "looks_say" | "looks_think" => {}
            "control_stop"
                if field_value(nested, "STOP_OPTION").and_then(Value::as_str) == Some("all") => {}
            _ => return Some(false),
        }
        current = nested.next.as_deref();
    }
    Some(true)
}

struct FusedReplacementCompare {
    expected: VariableInfo,
    negated: bool,
}

// fuse an immediate list read/equality with its preceding replacement without caching list state.
fn fused_replacement_compare(
    target: &ScratchTarget,
    replacement: &ScratchBlock,
    condition: &ScratchBlock,
    variables: &HashMap<String, VariableInfo>,
    lists: &HashMap<String, ListInfo>,
) -> Option<FusedReplacementCompare> {
    let condition = condition
        .inputs
        .get("CONDITION")
        .and_then(parse_input)
        .and_then(|input| match input {
            InputValue::BlockReference(id) => target.blocks.get(id),
            _ => None,
        })?;
    let (comparison, negated) = if condition.opcode == "operator_not" {
        let comparison = condition
            .inputs
            .get("OPERAND")
            .and_then(parse_input)
            .and_then(|input| match input {
                InputValue::BlockReference(id) => target.blocks.get(id),
                _ => None,
            })?;
        (comparison, true)
    } else {
        (condition, false)
    };
    if comparison.opcode != "operator_equals" {
        return None;
    }
    let replaced_list = list_field(replacement, "LIST", lists)?;
    let replaced_index = variable_input_info(target, replacement.inputs.get("INDEX"), variables)?;
    for (item_input, expected_input) in [
        ("OPERAND1", "OPERAND2"),
        ("OPERAND2", "OPERAND1"),
    ] {
        let InputValue::BlockReference(id) = parse_input(comparison.inputs.get(item_input)?)? else {
            continue;
        };
        let item_reporter = target.blocks.get(id)?;
        if item_reporter.opcode != "data_itemoflist"
            || list_field(item_reporter, "LIST", lists)?.slot != replaced_list.slot
        {
            continue;
        }
        let item_index = variable_input_info(target, item_reporter.inputs.get("INDEX"), variables)?;
        let expected = variable_input_info(target, comparison.inputs.get(expected_input), variables)?;
        if replaced_index.slot == item_index.slot
            && replaced_index.numeric
            && item_index.numeric
            && expected.numeric
        {
            return Some(FusedReplacementCompare { expected, negated });
        }
    }
    None
}

// Only batch a closed synchronous body: two constant-bound random writes,
// an immediate replacement/read comparison, a literal-only branch, and a
// constant variable increment. No operation in this body can inspect the
// temporarily borrowed list through ScratchState or suspend the script.
struct NumericReplacementBatch {
    list_slot: usize,
    index_slot: usize,
    item_slot: usize,
    expected_slot: usize,
}

fn numeric_replacement_batch(
    target: &ScratchTarget,
    plan: &FusedRepeatPlan,
    variables: &HashMap<String, VariableInfo>,
    lists: &HashMap<String, ListInfo>,
) -> Option<NumericReplacementBatch> {
    if !plan.local_random || plan.body.len() != 5 {
        return None;
    }
    let blocks: Vec<_> = plan.body.iter().map(|id| &target.blocks[id]).collect();
    if blocks[0].opcode != "data_setvariableto"
        || blocks[1].opcode != "data_setvariableto"
        || blocks[2].opcode != "data_replaceitemoflist"
        || blocks[3].opcode != "control_if"
        || blocks[4].opcode != "data_changevariableby"
    {
        return None;
    }
    let mut assigned = HashSet::new();
    for block in &blocks[..2] {
        let variable = variable_field(block, "VARIABLE", variables)?;
        let InputValue::BlockReference(id) = parse_input(block.inputs.get("VALUE")?)? else {
            return None;
        };
        if !variable.numeric
            || !assigned.insert(variable.slot)
            || constant_random_bounds(target, target.blocks.get(id)?).is_none()
        {
            return None;
        }
    }
    constant_numeric_input(target, blocks[4].inputs.get("VALUE"), &mut HashSet::new())?;
    variable_field(blocks[4], "VARIABLE", variables)?;
    let list = list_field(blocks[2], "LIST", lists)?;
    let index = variable_input_info(target, blocks[2].inputs.get("INDEX"), variables)?;
    let item = variable_input_info(target, blocks[2].inputs.get("ITEM"), variables)?;
    let compare = fused_replacement_compare(target, blocks[2], blocks[3], variables, lists)?;
    if !list.is_stage
        || !assigned.contains(&index.slot)
        || !assigned.contains(&item.slot)
        || !assigned.contains(&compare.expected.slot)
    {
        return None;
    }
    let mut branch = blocks[3]
        .inputs
        .get("SUBSTACK")
        .and_then(parse_input)
        .and_then(|input| match input {
            InputValue::BlockReference(id) => Some(id),
            _ => None,
        });
    while let Some(id) = branch {
        let nested = target.blocks.get(id)?;
        if matches!(nested.opcode.as_str(), "looks_say" | "looks_think")
            && !matches!(
                nested.inputs.get("MESSAGE").and_then(parse_input),
                Some(InputValue::String(_) | InputValue::Number(_))
            )
        {
            return None;
        }
        branch = nested.next.as_deref();
    }
    Some(NumericReplacementBatch {
        list_slot: list.slot,
        index_slot: index.slot,
        item_slot: item.slot,
        expected_slot: compare.expected.slot,
    })
}

#[allow(clippy::too_many_arguments)]
fn generate_fused_repeat_loop(
    target: &ScratchTarget,
    id: &str,
    block: &ScratchBlock,
    graph: &ScriptGraph,
    variables: &HashMap<String, VariableInfo>,
    lists: &HashMap<String, ListInfo>,
    arguments: &HashMap<String, String>,
    plan: &FusedRepeatPlan,
    output: &mut String,
) {
    let pc = graph.block_pcs[id];
    let next_pc = block
        .next
        .as_ref()
        .and_then(|next| graph.block_pcs.get(next).copied())
        .unwrap_or(graph.fallthrough_pcs[id]);
    let repeat_slot = graph.repeat_slots[id];
    let times = expression(
        target,
        block.inputs.get("TIMES"),
        variables,
        lists,
        arguments,
        &mut HashSet::new(),
    );

    line(output, 4, &format!("{pc} => {{"));
    line(
        output,
        5,
        "let scratch_warp_mode = script.frames.last().is_some_and(|frame| frame.warp_mode);",
    );
    line(
        output,
        5,
        "if !scratch_warp_mode { script.warp_budget.reset(); }",
    );
    line(
        output,
        5,
        &format!(
            "let mut repeat_remaining = script.frames.last().unwrap().repeat_counts[{repeat_slot}];"
        ),
    );
    line(output, 5, "if repeat_remaining == 0 {");
    line(
        output,
        6,
        &format!("repeat_remaining = scratch_round({times}).max(0.0) as usize;"),
    );
    line(
        output,
        6,
        &format!(
            "script.frames.last_mut().unwrap().repeat_counts[{repeat_slot}] = repeat_remaining;"
        ),
    );
    line(output, 5, "}");
    line(output, 5, "if repeat_remaining == 0 {");
    line(
        output,
        6,
        &format!(
            "script.frames.last_mut().unwrap().pc = {};",
            pc_literal(next_pc)
        ),
    );
    line(output, 6, "continue;");
    line(output, 5, "}");
    let batch = numeric_replacement_batch(target, plan, variables, lists);
    if plan.local_random {
        if let Some(batch) = &batch {
            line(
                output,
                5,
                &format!(
                    "let scratch_fused_status = state.with_stage_numeric_list_batch({}, |state, scratch_batch| {{ with_scratch_random(|loop_random| {{",
                    batch.list_slot
                ),
            );
        } else {
            line(
                output,
                5,
                "let scratch_fused_status = with_scratch_random(|loop_random| {",
            );
        }
        line(output, 6, "loop {");
    } else {
        line(output, 5, "loop {");
    }
    let loop_indent = if plan.local_random { 6 } else { 5 };
    let body_indent = loop_indent + 1;
    for (step_index, statement_id) in plan.body.iter().enumerate() {
        let statement = &target.blocks[statement_id];
        let following_compare = statement
            .next
            .as_ref()
            .and_then(|next| target.blocks.get(next))
            .filter(|next| next.opcode == "control_if")
            .and_then(|next| fused_replacement_compare(target, statement, next, variables, lists));
        let previous_compare = step_index
            .checked_sub(1)
            .and_then(|previous| target.blocks.get(&plan.body[previous]))
            .filter(|previous| previous.opcode == "data_replaceitemoflist")
            .and_then(|previous| {
                fused_replacement_compare(target, previous, statement, variables, lists)
            });
        if let Some(batch) = &batch {
            if step_index < 2 {
                let variable = variable_field(statement, "VARIABLE", variables).unwrap();
                let value = fused_random_expression(
                    expression(
                        target,
                        statement.inputs.get("VALUE"),
                        variables,
                        lists,
                        arguments,
                        &mut HashSet::new(),
                    ),
                    true,
                );
                let setter = if variable.is_stage {
                    "set_shared_number"
                } else {
                    "set_existing_local_number"
                };
                line(
                    output,
                    body_indent,
                    &format!(
                        "let scratch_batch_value_{} = {value}; state.{setter}({}, scratch_batch_value_{});",
                        variable.slot, variable.slot, variable.slot
                    ),
                );
                continue;
            }
            if step_index == 2 {
                line(
                    output,
                    body_indent,
                    &format!(
                        "let (scratch_list_changed, scratch_list_replace_matches) = scratch_batch.replace_number_and_equals(scratch_batch_value_{}, scratch_batch_value_{}, scratch_batch_value_{}); state.list_changed_result({}, scratch_list_changed);",
                        batch.index_slot, batch.item_slot, batch.expected_slot, batch.list_slot
                    ),
                );
                continue;
            }
        }
        match statement.opcode.as_str() {
            "data_setvariableto" => generate_fused_set_variable(
                target,
                statement,
                variables,
                lists,
                arguments,
                plan.local_random,
                output,
                body_indent,
            ),
            "data_changevariableby" => generate_fused_change_variable(
                target,
                statement,
                variables,
                lists,
                arguments,
                plan.local_random,
                output,
                body_indent,
            ),
            "data_replaceitemoflist" => generate_fused_replace_item(
                target,
                statement,
                variables,
                lists,
                arguments,
                plan.local_random,
                following_compare.as_ref(),
                output,
                body_indent,
            ),
            "control_if" => generate_fused_if(
                target,
                statement,
                variables,
                lists,
                arguments,
                plan.local_random,
                previous_compare.as_ref(),
                output,
                body_indent,
            ),
            _ => unreachable!("fused repeat plan only contains validated statements"),
        }
    }
    line(output, body_indent, "repeat_remaining -= 1;");
    line(output, body_indent, "if repeat_remaining == 0 {");
    line(
        output,
        body_indent + 1,
        &format!("script.frames.last_mut().unwrap().repeat_counts[{repeat_slot}] = 0;"),
    );
    line(
        output,
        body_indent + 1,
        &format!(
            "script.frames.last_mut().unwrap().pc = {};",
            pc_literal(next_pc)
        ),
    );
    line(
        output,
        body_indent + 1,
        if plan.local_random {
            "if script.yield_at_safe_point(clock) { return Some(ScriptStatus::Yield); }"
        } else {
            "if script.yield_at_safe_point(clock) { return ScriptStatus::Yield; }"
        },
    );
    line(output, body_indent + 1, "break;");
    line(output, body_indent, "}");
    line(
        output,
        body_indent,
        "if script.yield_at_safe_point(clock) {",
    );
    line(
        output,
        body_indent + 1,
        &format!(
            "script.frames.last_mut().unwrap().repeat_counts[{repeat_slot}] = repeat_remaining;"
        ),
    );
    line(
        output,
        body_indent + 1,
        &format!("script.frames.last_mut().unwrap().pc = {pc};"),
    );
    line(
        output,
        body_indent + 1,
        if plan.local_random {
            "return Some(ScriptStatus::Yield);"
        } else {
            "return ScriptStatus::Yield;"
        },
    );
    line(output, body_indent, "}");
    line(output, loop_indent, "}");
    if plan.local_random {
        line(output, 6, "None");
        line(output, 5, if batch.is_some() { "}) });" } else { "});" });
        line(
            output,
            5,
            "if let Some(scratch_fused_status) = scratch_fused_status { return scratch_fused_status; }",
        );
    }
    line(output, 5, "continue;");
    line(output, 4, "},");
}

#[allow(clippy::too_many_arguments)]
fn generate_fused_set_variable(
    target: &ScratchTarget,
    block: &ScratchBlock,
    variables: &HashMap<String, VariableInfo>,
    lists: &HashMap<String, ListInfo>,
    arguments: &HashMap<String, String>,
    local_random: bool,
    output: &mut String,
    indent: usize,
) {
    if let Some(variable) = variable_field(block, "VARIABLE", variables) {
        let input = block.inputs.get("VALUE");
        let value = fused_random_expression(
            expression(target, input, variables, lists, arguments, &mut HashSet::new()),
            local_random,
        );
        if variable.numeric {
            let setter = if variable.is_stage {
                "set_shared_number"
            } else {
                "set_local_number"
            };
            if let Some(source) = variable_input_info(target, input, variables)
                .filter(|source| source.numeric)
            {
                let source = source.read_expression();
                line(
                    output,
                    indent,
                    &format!(
                        "if let ScratchValue::Number(scratch_numeric_value) = {source} {{ state.{setter}({}, *scratch_numeric_value); }} else {{ let scratch_new_value: ScratchValue = ({source}).clone(); {} }}",
                        variable.slot,
                        variable.write_statement("scratch_new_value")
                    ),
                );
            } else {
                line(
                    output,
                    indent,
                    &format!("state.{setter}({}, {value});", variable.slot),
                );
            }
        } else if let Some(source) = variable_input(target, input, variables, lists, arguments) {
            line(
                output,
                indent,
                &format!("let scratch_new_value: ScratchValue = ({source}).clone();"),
            );
            line(output, indent, &variable.write_statement("scratch_new_value"));
        } else {
            line(
                output,
                indent,
                &format!("let scratch_new_value: ScratchValue = ({value}).into();"),
            );
            line(output, indent, &variable.write_statement("scratch_new_value"));
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn generate_fused_change_variable(
    target: &ScratchTarget,
    block: &ScratchBlock,
    variables: &HashMap<String, VariableInfo>,
    lists: &HashMap<String, ListInfo>,
    arguments: &HashMap<String, String>,
    local_random: bool,
    output: &mut String,
    indent: usize,
) {
    if let Some(variable) = variable_field(block, "VARIABLE", variables) {
        let value = fused_random_expression(
            expression(
                target,
                block.inputs.get("VALUE"),
                variables,
                lists,
                arguments,
                &mut HashSet::new(),
            ),
            local_random,
        );
        let method = if variable.is_stage {
            "change_shared_number"
        } else {
            "change_local_number"
        };
        line(
            output,
            indent,
            &format!("state.{method}({}, scratch_to_number({value}));", variable.slot),
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn generate_fused_replace_item(
    target: &ScratchTarget,
    block: &ScratchBlock,
    variables: &HashMap<String, VariableInfo>,
    lists: &HashMap<String, ListInfo>,
    arguments: &HashMap<String, String>,
    local_random: bool,
    compare: Option<&FusedReplacementCompare>,
    output: &mut String,
    indent: usize,
) {
    if let Some(list) = list_field(block, "LIST", lists) {
        let index_input = block.inputs.get("INDEX");
        let item_input = block.inputs.get("ITEM");
        if let Some(compare) = compare {
            let index = variable_input_info(target, index_input, variables)
                .expect("fused replacement index was validated");
            let item = variable_input_info(target, item_input, variables)
                .expect("fused replacement value was validated");
            line(
                output,
                indent,
                &format!(
                    "let scratch_list_replace_inputs = match (state.get({}), state.get({}), state.get({})) {{ (ScratchValue::Number(scratch_index), ScratchValue::Number(scratch_item), ScratchValue::Number(scratch_expected)) => Some((*scratch_index, *scratch_item, *scratch_expected)), _ => None }}; let scratch_list_replace_result = scratch_list_replace_inputs.map(|(scratch_index, scratch_item, scratch_expected)| state.list_mut({}).replace_number_and_equals(scratch_index, scratch_item, scratch_expected));",
                    index.slot,
                    item.slot,
                    compare.expected.slot,
                    list.slot
                ),
            );
            line(
                output,
                indent,
                &format!(
                    "let (scratch_list_changed, scratch_list_replace_matches) = scratch_list_replace_result.unwrap_or_else(|| {{ let scratch_list_index: ScratchValue = ({}).clone(); let scratch_list_item: ScratchValue = ({}).clone(); let scratch_expected: ScratchValue = ({}).clone(); let scratch_list_changed = state.list_mut({}).replace(scratch_list_index.clone(), scratch_list_item); let scratch_list_replace_matches = scratch_equals(state.list({}).get(&scratch_list_index), &scratch_expected); (scratch_list_changed, scratch_list_replace_matches) }});",
                    index.read_expression(),
                    item.read_expression(),
                    compare.expected.read_expression(),
                    list.slot,
                    list.slot
                ),
            );
            line(
                output,
                indent,
                &format!(
                    "state.list_changed_result({}, scratch_list_changed);",
                    list.slot
                ),
            );
            return;
        }
        if let Some((index, item)) = variable_input_info(target, index_input, variables)
            .zip(variable_input_info(target, item_input, variables))
            .filter(|(index, item)| index.numeric && item.numeric)
        {
            line(
                output,
                indent,
                &format!(
                    "let scratch_list_numeric_pair = match ({}, {}) {{ (ScratchValue::Number(scratch_list_index), ScratchValue::Number(scratch_list_item)) => Some((*scratch_list_index, *scratch_list_item)), _ => None }};",
                    index.read_expression(),
                    item.read_expression()
                ),
            );
            line(
                output,
                indent,
                &format!(
                    "let scratch_list_changed = if let Some((scratch_list_index, scratch_list_item)) = scratch_list_numeric_pair {{ state.list_mut({}).replace_number(scratch_list_index, scratch_list_item) }} else {{ let scratch_list_index: ScratchValue = ({}).clone(); let scratch_list_item: ScratchValue = ({}).clone(); state.list_mut({}).replace(scratch_list_index, scratch_list_item) }};",
                    list.slot,
                    index.read_expression(),
                    item.read_expression(),
                    list.slot
                ),
            );
        } else {
            let item = fused_random_expression(
                owned_input(target, item_input, variables, lists, arguments),
                local_random,
            );
            let index = fused_random_expression(
                owned_input(target, index_input, variables, lists, arguments),
                local_random,
            );
            line(
                output,
                indent,
                &format!("let scratch_list_index: ScratchValue = {index};"),
            );
            line(
                output,
                indent,
                &format!("let scratch_list_item: ScratchValue = {item};"),
            );
            line(
                output,
                indent,
                &format!(
                    "let scratch_list_changed = state.list_mut({}).replace(scratch_list_index, scratch_list_item);",
                    list.slot
                ),
            );
        }
        line(
            output,
            indent,
            &format!("state.list_changed_result({}, scratch_list_changed);", list.slot),
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn generate_fused_if(
    target: &ScratchTarget,
    block: &ScratchBlock,
    variables: &HashMap<String, VariableInfo>,
    lists: &HashMap<String, ListInfo>,
    arguments: &HashMap<String, String>,
    local_random: bool,
    compare: Option<&FusedReplacementCompare>,
    output: &mut String,
    indent: usize,
) {
    let condition = compare.map_or_else(
        || {
            fused_random_expression(
                expression(
                    target,
                    block.inputs.get("CONDITION"),
                    variables,
                    lists,
                    arguments,
                    &mut HashSet::new(),
                ),
                local_random,
            )
        },
        |compare| {
            if compare.negated {
                "!scratch_list_replace_matches".to_owned()
            } else {
                "scratch_list_replace_matches".to_owned()
            }
        },
    );
    line(output, indent, &format!("if scratch_bool({condition}) {{"));
    let mut current = block
        .inputs
        .get("SUBSTACK")
        .and_then(parse_input)
        .and_then(|input| match input {
            InputValue::BlockReference(id) => Some(id),
            _ => None,
        });
    while let Some(id) = current {
        let nested = &target.blocks[id];
        match nested.opcode.as_str() {
            "looks_say" | "looks_think" => {
                let value = fused_random_expression(
                    expression(
                        target,
                        nested.inputs.get("MESSAGE"),
                        variables,
                        lists,
                        arguments,
                        &mut HashSet::new(),
                    ),
                    local_random,
                );
                line(
                    output,
                    indent + 1,
                    &format!("let scratch_bubble_text = scratch_string({value});"),
                );
                if nested.opcode == "looks_say" {
                    let name = say_variable_name(target, nested.inputs.get("MESSAGE"))
                        .unwrap_or_else(|| "say".to_owned());
                    line(
                        output,
                        indent + 1,
                        &format!(
                            "print_value!(state, {}, scratch_bubble_text.clone());",
                            quote(&name)
                        ),
                    );
                }
                line(
                    output,
                    indent + 1,
                    &format!(
                        "state.set_speech_bubble(BubbleKind::{}, scratch_bubble_text, None);",
                        if nested.opcode == "looks_say" {
                            "Say"
                        } else {
                            "Think"
                        }
                    ),
                );
            }
            "control_stop" => match field_value(nested, "STOP_OPTION").and_then(Value::as_str) {
                Some("all") => line(
                    output,
                    indent + 1,
                    if local_random {
                        "return Some(ScriptStatus::StopAll);"
                    } else {
                        "return ScriptStatus::StopAll;"
                    },
                ),
                _ => unreachable!("fused branch validated the stop option"),
            },
            _ => unreachable!("fused branch validated the statement opcode"),
        }
        current = nested.next.as_deref();
    }
    line(output, indent, "}");
}

// bind Scratch random reporters to the loop's existing PRNG borrow when safe.
fn fused_random_expression(expression: String, local_random: bool) -> String {
    if local_random {
        expression.replace(
            "scratch_random_constant(",
            "loop_random.between_with_integer_bounds(",
        )
    } else {
        expression
    }
}

struct NumericLoopPlan {
    body: Vec<String>,
    local_slots: Vec<usize>,
    written_slots: Vec<usize>,
}

fn numeric_loop_plan(
    target: &ScratchTarget,
    block: &ScratchBlock,
    variables: &HashMap<String, VariableInfo>,
) -> Option<NumericLoopPlan> {
    if !matches!(
        block.opcode.as_str(),
        "control_repeat" | "control_repeat_until" | "control_while" | "control_forever"
    ) {
        return None;
    }
    let mut body = Vec::new();
    let mut current = block
        .inputs
        .get("SUBSTACK")
        .and_then(parse_input)
        .and_then(|input| match input {
            InputValue::BlockReference(id) => Some(id),
            _ => None,
        });
    let mut seen = HashSet::new();
    let mut local_slots = HashSet::new();
    let mut written_slots = BTreeSet::new();
    while let Some(id) = current {
        if !seen.insert(id.to_owned()) {
            return None;
        }
        let statement = target.blocks.get(id)?;
        if !matches!(
            statement.opcode.as_str(),
            "data_setvariableto" | "data_changevariableby"
        ) {
            return None;
        }
        let variable = variable_field(statement, "VARIABLE", variables)?;
        if !variable.numeric {
            return None;
        }
        written_slots.insert(variable.slot);
        local_slots.insert(variable.slot);
        if !collect_numeric_local_slots(
            target,
            statement.inputs.get("VALUE"),
            variables,
            &mut local_slots,
            &mut HashSet::new(),
        ) {
            return None;
        }
        body.push(id.to_owned());
        current = statement.next.as_deref();
    }
    if body.is_empty() {
        return None;
    }

    let loop_input = match block.opcode.as_str() {
        "control_repeat" => block.inputs.get("TIMES"),
        "control_repeat_until" | "control_while" => block.inputs.get("CONDITION"),
        "control_forever" => None,
        _ => unreachable!(),
    };
    if loop_input.is_some()
        && !collect_numeric_local_slots(
            target,
            loop_input,
            variables,
            &mut local_slots,
            &mut HashSet::new(),
        )
    {
        return None;
    }

    let mut local_slots = local_slots.into_iter().collect::<Vec<_>>();
    local_slots.sort_unstable();
    Some(NumericLoopPlan {
        body,
        local_slots,
        written_slots: written_slots.into_iter().collect(),
    })
}

fn generate_native_numeric_loop(
    target: &ScratchTarget,
    id: &str,
    block: &ScratchBlock,
    graph: &ScriptGraph,
    variables: &HashMap<String, VariableInfo>,
    plan: &NumericLoopPlan,
    fast_math: bool,
    output: &mut String,
) {
    let pc = graph.block_pcs[id];
    let next_pc = block
        .next
        .as_ref()
        .and_then(|next| graph.block_pcs.get(next).copied())
        .unwrap_or(graph.fallthrough_pcs[id]);
    let locals = plan
        .local_slots
        .iter()
        .map(|slot| (*slot, format!("scratch_local_{slot}")))
        .collect::<HashMap<_, _>>();

    line(output, 4, &format!("{pc} => {{"));
    for slot in &plan.local_slots {
        line(
            output,
            5,
            &format!(
                "let mut scratch_local_{slot} = scratch_to_number({});",
                variable_read_for_slot(variables, *slot)
            ),
        );
    }
    line(
        output,
        5,
        "let scratch_warp_mode = script.frames.last().is_some_and(|frame| frame.warp_mode);",
    );
    line(
        output,
        5,
        "if !scratch_warp_mode { script.warp_budget.reset(); }",
    );

    if block.opcode == "control_repeat" {
        generate_native_repeat_loop(
            target, id, block, graph, variables, plan, &locals, next_pc, output, fast_math,
        );
    } else {
        line(output, 5, "loop {");
        if block.opcode != "control_forever" {
            let condition = local_expression(
                target,
                block.inputs.get("CONDITION"),
                variables,
                &locals,
                fast_math,
                &mut HashSet::new(),
            )
            .expect("numeric loop condition was validated");
            let condition = local_to_bool(condition);
            let exit_condition = if block.opcode == "control_repeat_until" {
                condition
            } else {
                format!("!({condition})")
            };
            line(output, 6, &format!("if {exit_condition} {{ break; }}"));
        }
        generate_native_numeric_body(target, variables, plan, &locals, fast_math, output, 6);
        line(
            output,
            6,
            "if !scratch_warp_mode || script.warp_budget.expired(clock) {",
        );
        line(
            output,
            7,
            &format!("script.frames.last_mut().unwrap().pc = {pc};"),
        );
        generate_local_commits(plan, variables, output, 7);
        line(output, 7, "return ScriptStatus::Yield;");
        line(output, 6, "}");
        line(output, 5, "}");
        generate_local_commits(plan, variables, output, 5);
        line(
            output,
            5,
            &format!(
                "script.frames.last_mut().unwrap().pc = {};",
                pc_literal(next_pc)
            ),
        );
        line(output, 5, "continue;");
    }
    line(output, 4, "},");
}

#[allow(clippy::too_many_arguments)]
fn generate_native_repeat_loop(
    target: &ScratchTarget,
    id: &str,
    block: &ScratchBlock,
    graph: &ScriptGraph,
    variables: &HashMap<String, VariableInfo>,
    plan: &NumericLoopPlan,
    locals: &HashMap<usize, String>,
    next_pc: usize,
    output: &mut String,
    fast_math: bool,
) {
    let pc = graph.block_pcs[id];
    let slot = graph.repeat_slots[id];
    let times = local_expression(
        target,
        block.inputs.get("TIMES"),
        variables,
        locals,
        fast_math,
        &mut HashSet::new(),
    )
    .expect("numeric repeat count was validated");
    let times = local_to_number(times);
    line(
        output,
        5,
        &format!("let mut repeat_remaining = script.frames.last().unwrap().repeat_counts[{slot}];"),
    );
    line(output, 5, "if repeat_remaining == 0 {");
    line(
        output,
        6,
        &format!("repeat_remaining = scratch_round({times}).max(0.0) as usize;"),
    );
    line(
        output,
        6,
        &format!("script.frames.last_mut().unwrap().repeat_counts[{slot}] = repeat_remaining;"),
    );
    line(output, 5, "}");
    line(output, 5, "if repeat_remaining == 0 {");
    line(
        output,
        6,
        &format!(
            "script.frames.last_mut().unwrap().pc = {};",
            pc_literal(next_pc)
        ),
    );
    line(output, 6, "continue;");
    line(output, 5, "}");
    line(output, 5, "loop {");
    generate_native_numeric_body(target, variables, plan, locals, fast_math, output, 6);
    line(output, 6, "repeat_remaining -= 1;");
    line(
        output,
        6,
        &format!("script.frames.last_mut().unwrap().repeat_counts[{slot}] = repeat_remaining;"),
    );
    line(
        output,
        6,
        "if !scratch_warp_mode || script.warp_budget.expired(clock) {",
    );
    line(
        output,
        7,
        &format!(
            "script.frames.last_mut().unwrap().pc = if repeat_remaining > 0 {{ {pc} }} else {{ {} }};",
            pc_literal(next_pc)
        ),
    );
    generate_local_commits(plan, variables, output, 7);
    line(output, 7, "return ScriptStatus::Yield;");
    line(output, 6, "}");
    line(output, 6, "if repeat_remaining == 0 { break; }");
    line(output, 5, "}");
    generate_local_commits(plan, variables, output, 5);
    line(
        output,
        5,
        &format!(
            "script.frames.last_mut().unwrap().pc = {};",
            pc_literal(next_pc)
        ),
    );
    line(output, 5, "continue;");
}

fn generate_native_numeric_body(
    target: &ScratchTarget,
    variables: &HashMap<String, VariableInfo>,
    plan: &NumericLoopPlan,
    locals: &HashMap<usize, String>,
    fast_math: bool,
    output: &mut String,
    indent: usize,
) {
    for id in &plan.body {
        let block = &target.blocks[id];
        let variable = variable_field(block, "VARIABLE", variables)
            .expect("numeric loop target was validated");
        let value = local_expression(
            target,
            block.inputs.get("VALUE"),
            variables,
            locals,
            fast_math,
            &mut HashSet::new(),
        )
        .expect("numeric loop expression was validated");
        let local = &locals[&variable.slot];
        if block.opcode == "data_setvariableto" {
            assert_eq!(value.kind, LocalValueKind::Number);
            line(
                output,
                indent,
                &format!("{local} = {};", expression_argument(&value.code)),
            );
        } else if let Some(constant) = variable
            .non_nan
            .then(|| {
                constant_numeric_input(target, block.inputs.get("VALUE"), &mut HashSet::new())
                    .filter(|value| value.is_finite())
            })
            .flatten()
        {
            line(output, indent, &format!("{local} += {constant:?}_f64;"));
        } else if fast_math && value.kind == LocalValueKind::Number {
            line(
                output,
                indent,
                &format!("{local} += {};", expression_argument(&value.code)),
            );
        } else {
            line(
                output,
                indent,
                &format!(
                    "{local} = scratch_add({local}, {});",
                    expression_argument(&value.code)
                ),
            );
        }
    }
}

fn generate_local_commits(
    plan: &NumericLoopPlan,
    variables: &HashMap<String, VariableInfo>,
    output: &mut String,
    indent: usize,
) {
    for slot in &plan.written_slots {
        line(
            output,
            indent,
            &variable_write_for_slot(variables, *slot, &format!("scratch_local_{slot}.into()")),
        );
    }
}

fn variable_read_for_slot(variables: &HashMap<String, VariableInfo>, slot: usize) -> String {
    variables
        .values()
        .find(|variable| variable.slot == slot)
        .map(VariableInfo::read_expression)
        .unwrap_or_else(|| format!("state.get({slot})"))
}

fn variable_write_for_slot(
    variables: &HashMap<String, VariableInfo>,
    slot: usize,
    value: &str,
) -> String {
    variables
        .values()
        .find(|variable| variable.slot == slot)
        .map(|variable| variable.write_statement(value))
        .unwrap_or_else(|| format!("state.set({slot}, {value});"))
}

fn is_straight_line_block(block: &ScratchBlock) -> bool {
    !block.opcode.starts_with("control_")
        && !matches!(
            block.opcode.as_str(),
            "motion_glideto"
                | "motion_glidesecstoxy"
                | "looks_sayforsecs"
                | "looks_thinkforsecs"
                | "looks_switchbackdropto"
                | "looks_nextbackdrop"
                | "sound_playuntildone"
                | "sound_seteffectto"
                | "sound_changeeffectby"
                | "sound_setvolumeto"
                | "sound_changevolumeby"
        )
        && block.opcode != "procedures_call"
        && block.opcode != "event_broadcast"
        && block.opcode != "event_broadcastandwait"
}

fn generate_loop_boundary(
    target: &ScratchTarget,
    id: &str,
    graph: &ScriptGraph,
    output: &mut String,
) {
    let pc = graph.boundary_pcs[id];
    line(output, 4, &format!("{pc} => {{"));
    let block = &target.blocks[id];
    if block.opcode == "control_repeat" {
        let slot = graph.repeat_slots[id];
        let next_pc = block
            .next
            .as_ref()
            .and_then(|next| graph.block_pcs.get(next).copied())
            .unwrap_or(graph.fallthrough_pcs[id]);
        let branch_pc = block
            .inputs
            .get("SUBSTACK")
            .and_then(parse_input)
            .and_then(|input| match input {
                InputValue::BlockReference(id) => graph.block_pcs.get(id).copied(),
                _ => None,
            })
            .unwrap_or(pc);
        line(
            output,
            5,
            &format!(
                "let repeat_remaining = script.frames.last().unwrap().repeat_counts[{slot}].saturating_sub(1);"
            ),
        );
        line(
            output,
            5,
            &format!("script.frames.last_mut().unwrap().repeat_counts[{slot}] = repeat_remaining;"),
        );
        line(
            output,
            5,
            &format!(
                "let next_pc = if repeat_remaining > 0 {{ {branch_pc} }} else {{ {} }};",
                pc_literal(next_pc)
            ),
        );
        line(output, 5, "script.frames.last_mut().unwrap().pc = next_pc;");
    } else {
        let loop_pc = graph.block_pcs[id];
        line(
            output,
            5,
            &format!("script.frames.last_mut().unwrap().pc = {loop_pc};"),
        );
    }
    line(
        output,
        5,
        "if script.yield_at_safe_point(clock) { return ScriptStatus::Yield; }",
    );
    line(output, 5, "continue;");
    line(output, 4, "},");
}

// Only cache indexes whose expression is composed of pure arithmetic,
// literals and variable reads. Preserve dynamic selector values in the cache.
fn collect_list_index_variables(
    target: &ScratchTarget, input: Option<&Value>, variables: &HashMap<String, VariableInfo>,
    active: &mut HashSet<String>, found: &mut HashSet<usize>,
) -> bool {
    match input.and_then(parse_input) {
        Some(InputValue::Number(_) | InputValue::String(_)) => true,
        Some(InputValue::VariableReference(reference)) => variables.get(reference.id).is_some_and(|variable| { found.insert(variable.slot); true }),
        Some(InputValue::BlockReference(id)) => {
            if !active.insert(id.to_owned()) { return false; }
            let result = target.blocks.get(id).is_some_and(|block| match block.opcode.as_str() {
                "data_variable" => variable_field(block, "VARIABLE", variables).is_some_and(|variable| { found.insert(variable.slot); true }),
                "operator_add" | "operator_subtract" | "operator_multiply" | "operator_divide" => {
                    collect_list_index_variables(target, block.inputs.get("NUM1"), variables, active, found)
                        && collect_list_index_variables(target, block.inputs.get("NUM2"), variables, active, found)
                }
                "math_number" | "math_integer" | "math_whole_number" | "math_positive_number" | "math_angle" | "text" => true,
                _ => false,
            });
            active.remove(id);
            result
        }
        _ => false,
    }
}

fn cacheable_list_argument(target: &ScratchTarget, input: Option<&Value>, variables: &HashMap<String, VariableInfo>) -> Option<HashSet<usize>> {
    let InputValue::BlockReference(id) = input.and_then(parse_input)? else { return None };
    let block = target.blocks.get(id)?;
    if block.opcode != "data_itemoflist" { return None; }
    let mut found = HashSet::new();
    collect_list_index_variables(target, block.inputs.get("INDEX"), variables, &mut HashSet::new(), &mut found).then_some(found)
}

// The speculative numeric list read may be retried only when the index
// expression cannot itself invoke Scratch's random list selector.
fn index_has_random_selector(target: &ScratchTarget, input: Option<&Value>, active: &mut HashSet<String>) -> bool {
    match input.and_then(parse_input) {
        Some(InputValue::String(value)) => value.as_str().is_some_and(|text| matches!(text, "random" | "any")),
        Some(InputValue::BlockReference(id)) => {
            if !active.insert(id.to_owned()) { return true; }
            let found = target.blocks.get(id).is_none_or(|block| {
                (block.opcode == "text" && field_value(block, "TEXT").and_then(Value::as_str).is_some_and(|text| matches!(text, "random" | "any")))
                    || block.inputs.values().any(|nested| index_has_random_selector(target, Some(nested), active))
            });
            active.remove(id);
            found
        }
        _ => false,
    }
}

fn numeric_list_call_candidate(target: &ScratchTarget, block: &ScratchBlock, procedure: &ProcedureSignature, entry: &ProcedureEntry, caches: &[Option<HashSet<usize>>]) -> bool {
    entry.numeric_direct && !procedure.argument_ids.is_empty() && caches.iter().all(Option::is_some)
        && procedure.argument_ids.iter().all(|id| {
            let Some(InputValue::BlockReference(reporter)) = block.inputs.get(id).and_then(parse_input) else { return false };
            target.blocks.get(reporter).is_some_and(|reporter| !index_has_random_selector(target, reporter.inputs.get("INDEX"), &mut HashSet::new()))
        })
}

fn cached_procedure_indexes(
    target: &ScratchTarget, block: &ScratchBlock, procedure: &ProcedureSignature,
    entry: &ProcedureEntry, variables: &HashMap<String, VariableInfo>,
) -> (Vec<Option<HashSet<usize>>>, Vec<usize>) {
    let caches = procedure.argument_ids.iter().map(|id| {
        if entry.direct_name.is_some() { cacheable_list_argument(target, block.inputs.get(id), variables) } else { None }
    }).collect::<Vec<_>>();
    let mut counts = HashMap::<usize, usize>::new();
    for slots in caches.iter().flatten() { for slot in slots { *counts.entry(*slot).or_default() += 1; } }
    let numeric_candidate = numeric_list_call_candidate(target, block, procedure, entry, &caches);
    let mut result = counts.into_iter().filter_map(|(slot, count)| (count >= 2 || numeric_candidate).then_some(slot)).collect::<Vec<_>>();
    result.sort_unstable();
    (caches, result)
}

fn generate_resumable_procedure_call(
    target: &ScratchTarget,
    block: &ScratchBlock,
    return_pc: usize,
    variables: &HashMap<String, VariableInfo>,
    lists: &HashMap<String, ListInfo>,
    arguments: &HashMap<String, String>,
    procedures: &[ProcedureSignature],
    entries: &HashMap<String, ProcedureEntry>,
    output: &mut String,
    indent: usize,
    emit_continue: bool,
    reusable_indexes: Option<&HashSet<usize>>,
    mut reusable_items: Option<&mut HashMap<String, CachedNumericItem>>,
) {
    let Some(procedure) = procedure_for_call(block, procedures) else {
        line(
            output,
            indent,
            "todo!(\"unsupported or missing custom procedure\");",
        );
        return;
    };
    let Some(entry) = entries.get(&procedure.definition_id) else {
        line(output, indent, "todo!(\"missing custom procedure body\");");
        return;
    };

    let (argument_caches, cached_slots) = cached_procedure_indexes(target, block, procedure, entry, variables);
    let raw_values = procedure.argument_ids.iter().enumerate().map(|(index, argument_id)| {
        let input = block.inputs.get(argument_id);
        if input.is_none() { return format!("todo!({})", quote(&format!("missing procedure argument {index}"))); }
        let mut cached_variables;
        let expression_variables = if let Some(slots) = &argument_caches[index] {
            cached_variables = variables.clone();
            for variable in cached_variables.values_mut() {
                if slots.contains(&variable.slot) && cached_slots.contains(&variable.slot) {
                    variable.index_read_override = Some(format!("cached_index_{}", variable.slot));
                }
            }
            &cached_variables
        } else { variables };
        let value = expression(target, input, expression_variables, lists, arguments, &mut HashSet::new());
        value
    }).collect::<Vec<_>>();
    let values = raw_values.iter().map(|value| format!("({value}).to_owned().into()")).collect::<Vec<_>>();
    if let Some(direct_name) = &entry.direct_name {
        line(output, indent, "let inherited_warp = script.frames.last().is_some_and(|frame| frame.warp_mode);");
        line(output, indent, &format!("script.frames.last_mut().unwrap().pc = {};", pc_literal(return_pc)));
        for slot in &cached_slots {
            if reusable_indexes.is_some_and(|cached| cached.contains(slot)) { continue; }
            if let Some(variable) = variables.values().find(|variable| variable.slot == *slot) {
                line(output, indent, &format!("let cached_index_{slot} = CachedListIndex::new({});", variable.read_expression()));
            }
        }
        let numeric_candidate = numeric_list_call_candidate(target, block, procedure, entry, &argument_caches);
        if numeric_candidate {
            let guards = cached_slots.iter().map(|slot| format!("cached_index_{slot}.is_numeric()")).collect::<Vec<_>>();
            let guard = if guards.is_empty() { "true".to_owned() } else { guards.join(" && ") };
            let mut item_names = Vec::new();
            for (index, value) in raw_values.iter().enumerate() {
                if let Some(items) = reusable_items.as_deref_mut() {
                    if !items.contains_key(value) {
                        let InputValue::BlockReference(id) = block.inputs.get(&procedure.argument_ids[index]).and_then(parse_input).unwrap() else { unreachable!() };
                        let reporter = &target.blocks[id];
                        let list = list_field(reporter, "LIST", lists).unwrap();
                        let item = CachedNumericItem { name: format!("cached_numeric_item_{}", output.len()), list_slot: list.slot, index_slots: argument_caches[index].clone().unwrap_or_default() };
                        line(output, indent, &format!("let mut {} = f64::NAN;", item.name));
                        items.insert(value.clone(), item);
                    }
                    item_names.push(Some(items[value].name.clone()));
                } else { item_names.push(None); }
            }
            line(output, indent, "let mut numeric_call_completed = false;");
            line(output, indent, &format!("if {guard} {{"));
            for (index, value) in raw_values.iter().enumerate() {
                let expression = match &item_names[index] {
                    Some(name) => format!("{{ if {name}.is_nan() {{ {name} = ({value}).number_or_nan(); }} {name} }}"),
                    None => format!("({value}).number_or_nan()"),
                };
                line(output, indent + 1, &format!("let numeric_value_{index} = {expression};"));
            }
            let numeric_vars = (0..values.len()).map(|index| format!("numeric_value_{index}")).collect::<Vec<_>>();
            let valid = numeric_vars.iter().map(|name| format!("!{name}.is_nan()")).collect::<Vec<_>>().join(" && ");
            line(output, indent + 1, &format!("if {valid} {{"));
            line(output, indent + 2, &format!("if (inherited_warp || {}) && script.warp_budget.expired(clock) {{", procedure.warp));
            let frame_values = numeric_vars.iter().map(|name| format!("ScratchValue::Number({name})")).collect::<Vec<_>>().join(", ");
            line(output, indent + 3, &format!("script.frames.push(ProcedureFrame::new({}, {}, vec![0; {}], vec![{frame_values}], inherited_warp || {}));", entry.function_id, pc_literal(entry.start_pc), entry.repeat_count, procedure.warp));
            line(output, indent + 3, "return ScriptStatus::Yield;");
            line(output, indent + 2, "}");
            line(output, indent + 2, &format!("{direct_name}_numbers(state, clock, {});", numeric_vars.join(", ")));
            line(output, indent + 2, "numeric_call_completed = true;");
            line(output, indent + 1, "}");
            line(output, indent, "}");
            line(output, indent, "if !numeric_call_completed {");
        }
        let call_indent = indent + usize::from(numeric_candidate);
        for (index, value) in values.iter().enumerate() {
            line(output, call_indent, &format!("let direct_arg_{index}: ScratchValue = {value};"));
        }
        let args = (0..values.len()).map(|index| format!("direct_arg_{index}")).collect::<Vec<_>>();
        line(output, call_indent, &format!("if (inherited_warp || {}) && script.warp_budget.expired(clock) {{", procedure.warp));
        line(output, call_indent + 1, &format!("script.frames.push(ProcedureFrame::new({}, {}, vec![0; {}], vec![{}], inherited_warp || {}));", entry.function_id, pc_literal(entry.start_pc), entry.repeat_count, args.join(", "), procedure.warp));
        line(output, call_indent + 1, "return ScriptStatus::Yield;");
        line(output, call_indent, "}");
        let arguments = if args.is_empty() { String::new() } else { format!(", {}", args.join(", ")) };
        line(output, call_indent, &format!("{direct_name}(state, clock{arguments});"));
        if numeric_candidate { line(output, indent, "}"); }
        if emit_continue { line(output, indent, "continue;"); }
        return;
    }
    let inherited_warp = procedure.warp;
    line(
        output,
        indent,
        "let inherited_warp = script.frames.last().is_some_and(|frame| frame.warp_mode);",
    );
    line(
        output,
        indent,
        &format!(
            "script.frames.last_mut().unwrap().pc = {};",
            pc_literal(return_pc)
        ),
    );
    line(
        output,
        indent,
        &format!(
            "script.frames.push(ProcedureFrame::new({}, {}, vec![0; {}], vec![{}], inherited_warp || {}));",
            entry.function_id,
            pc_literal(entry.start_pc),
            entry.repeat_count,
            values.join(", "),
            inherited_warp
        ),
    );
    line(
        output,
        indent,
        "let recursive_call = script.frames.iter().filter(|frame| frame.function == script.frames.last().unwrap().function).count() > 1;",
    );
    line(
        output,
        indent,
        "if recursive_call && !script.frames.last().unwrap().warp_mode { return ScriptStatus::Yield; }",
    );
    line(
        output,
        indent,
        "if script.frames.last().unwrap().warp_mode && script.warp_budget_expired(clock) { return ScriptStatus::Yield; }",
    );
    line(output, indent, "continue;");
}

fn pc_literal(pc: usize) -> String {
    if pc == usize::MAX {
        "usize::MAX".to_owned()
    } else {
        pc.to_string()
    }
}

fn generate_resumable_statement(
    target: &ScratchTarget,
    block: &ScratchBlock,
    next_pc: usize,
    variables: &HashMap<String, VariableInfo>,
    lists: &HashMap<String, ListInfo>,
    arguments: &HashMap<String, String>,
    output: &mut String,
    indent: usize,
) {
    match block.opcode.as_str() {
        // scratch vm keeps these imported legacy commands as no-ops.
        "looks_hideallsprites"
        | "looks_changestretchby"
        | "looks_setstretchto"
        | "motion_scroll_right"
        | "motion_scroll_up"
        | "motion_align_scene" => line(output, indent, "let _ = ();"),
        "looks_show" => line(output, indent, "state.set_visible(true);"),
        "looks_hide" => line(output, indent, "state.set_visible(false);"),
        "looks_switchcostumeto" => {
            let costume = owned_input(
                target,
                block.inputs.get("COSTUME"),
                variables,
                lists,
                arguments,
            );
            line(
                output,
                indent,
                &format!("state.switch_costume_to({costume});"),
            );
        }
        "pen_clear" => line(output, indent, "state.clear_pen();"),
        "sound_play" => {
            let selector = owned_input(
                target,
                block.inputs.get("SOUND_MENU"),
                variables,
                lists,
                arguments,
            );
            line(
                output,
                indent,
                &format!(
                    "let scratch_sound_selector: ScratchValue = {selector}; state.start_sound(scratch_sound_selector, clock.now_millis());"
                ),
            );
        }
        "sound_stopallsounds" => line(output, indent, "state.stop_all_sounds(clock.now_millis());"),
        "sound_cleareffects" => line(
            output,
            indent,
            "state.clear_sound_effects(clock.now_millis());",
        ),
        "pen_penDown" => line(output, indent, "state.pen_down();"),
        "pen_penUp" => line(output, indent, "state.pen_up();"),
        "pen_stamp" => line(output, indent, "state.stamp();"),
        "pen_setPenColorToColor" => {
            let color = owned_input(
                target,
                block.inputs.get("COLOR"),
                variables,
                lists,
                arguments,
            );
            line(
                output,
                indent,
                &format!("let scratch_pen_color: ScratchValue = {color};"),
            );
            line(
                output,
                indent,
                if target_has_pen_hsv_controls(target) {
                    "state.set_pen_rgb_color(scratch_pen_color);"
                } else {
                    "state.set_pen_rgb_color_without_hsv(scratch_pen_color);"
                },
            );
        }
        "pen_changePenColorParamBy" | "pen_setPenColorParamTo" => {
            let parameter = pen_color_parameter_input(
                target,
                block.inputs.get("COLOR_PARAM"),
                variables,
                lists,
                arguments,
            );
            let value = expression(
                target,
                block.inputs.get("VALUE"),
                variables,
                lists,
                arguments,
                &mut HashSet::new(),
            );
            line(
                output,
                indent,
                &format!(
                    "state.set_pen_color_parameter({parameter}, scratch_to_number({value}), {});",
                    block.opcode == "pen_changePenColorParamBy"
                ),
            );
        }
        "pen_setPenHueToNumber" | "pen_changePenHueBy" => {
            let value = expression(
                target,
                block.inputs.get("HUE"),
                variables,
                lists,
                arguments,
                &mut HashSet::new(),
            );
            let method = if block.opcode == "pen_setPenHueToNumber" {
                "set_legacy_pen_hue"
            } else {
                "change_legacy_pen_hue"
            };
            line(
                output,
                indent,
                &format!("state.{method}(scratch_to_number({value}));"),
            );
        }
        "pen_setPenShadeToNumber" | "pen_changePenShadeBy" => {
            let value = expression(
                target,
                block.inputs.get("SHADE"),
                variables,
                lists,
                arguments,
                &mut HashSet::new(),
            );
            let method = if block.opcode == "pen_setPenShadeToNumber" {
                "set_legacy_pen_shade"
            } else {
                "change_legacy_pen_shade"
            };
            line(
                output,
                indent,
                &format!("state.{method}(scratch_to_number({value}));"),
            );
        }
        "pen_setPenSizeTo" | "pen_changePenSizeBy" => {
            let value = expression(
                target,
                block.inputs.get("SIZE"),
                variables,
                lists,
                arguments,
                &mut HashSet::new(),
            );
            let method = if block.opcode == "pen_setPenSizeTo" {
                "set_pen_size"
            } else {
                "change_pen_size"
            };
            line(
                output,
                indent,
                &format!("state.{method}(scratch_to_number({value}));"),
            );
        }
        "sensing_resettimer" => line(
            output,
            indent,
            "state.reset_project_timer(clock.now_millis());",
        ),
        "event_broadcast" | "event_broadcastandwait" => {
            let message = broadcast_message_expression(
                target,
                block.inputs.get("BROADCAST_INPUT"),
                variables,
                lists,
                arguments,
            );
            line(
                output,
                indent,
                &format!(
                    "script.frames.last_mut().unwrap().pc = {};",
                    pc_literal(next_pc)
                ),
            );
            line(
                output,
                indent,
                &format!(
                    "return ScriptStatus::Broadcast {{ message: broadcast_message_key(scratch_string({message})), wait: {} }};",
                    block.opcode == "event_broadcastandwait"
                ),
            );
        }
        "control_stop" => match field_value(block, "STOP_OPTION").and_then(Value::as_str) {
            Some("all") => line(output, indent, "return ScriptStatus::StopAll;"),
            Some("this script") => {
                line(
                    output,
                    indent,
                    "if script.frames.len() > 1 { script.frames.pop(); continue; } return ScriptStatus::Complete;",
                );
            }
            Some("other scripts in sprite") | Some("other scripts in stage") => {
                line(
                    output,
                    indent,
                    &format!(
                        "script.frames.last_mut().unwrap().pc = {};",
                        pc_literal(next_pc)
                    ),
                );
                line(output, indent, "return ScriptStatus::StopOtherScripts;");
            }
            _ => {}
        },
        // use mutation results instead of retaining an arc snapshot across each generated write.
        "data_addtolist" => {
            if let Some(list) = list_field(block, "LIST", lists) {
                let input = block.inputs.get("ITEM");
                let item = owned_input(target, input, variables, lists, arguments);
                line(
                    output,
                    indent,
                    &format!("let scratch_list_item: ScratchValue = {item};"),
                );
                line(
                    output,
                    indent,
                    &format!(
                        "let scratch_list_changed = state.list_mut({}).add(scratch_list_item);",
                        list.slot
                    ),
                );
                line(
                    output,
                    indent,
                    &format!("state.list_changed_result({}, scratch_list_changed);", list.slot),
                );
            } else {
                line(output, indent, "todo!(\"missing list reference\");");
            }
        }
        "data_deleteoflist" => {
            if let Some(list) = list_field(block, "LIST", lists) {
                let index = owned_input(
                    target,
                    block.inputs.get("INDEX"),
                    variables,
                    lists,
                    arguments,
                );
                line(
                    output,
                    indent,
                    &format!("let scratch_list_index: ScratchValue = {index};"),
                );
                line(
                    output,
                    indent,
                    &format!(
                        "let scratch_list_changed = state.list_mut({}).delete(scratch_list_index);",
                        list.slot
                    ),
                );
                line(
                    output,
                    indent,
                    &format!("state.list_changed_result({}, scratch_list_changed);", list.slot),
                );
            } else {
                line(output, indent, "todo!(\"missing list reference\");");
            }
        }
        "data_deletealloflist" => {
            if let Some(list) = list_field(block, "LIST", lists) {
                line(
                    output,
                    indent,
                    &format!(
                        "let scratch_list_changed = state.list_mut({}).delete_all();",
                        list.slot
                    ),
                );
                line(
                    output,
                    indent,
                    &format!("state.list_changed_result({}, scratch_list_changed);", list.slot),
                );
            } else {
                line(output, indent, "todo!(\"missing list reference\");");
            }
        }
        "data_insertatlist" | "data_replaceitemoflist" => {
            if let Some(list) = list_field(block, "LIST", lists) {
                let input = block.inputs.get("ITEM");
                let index_input = block.inputs.get("INDEX");
                let numeric_variables = (|| {
                    let index = variable_input_info(target, index_input, variables)?;
                    let item = variable_input_info(target, input, variables)?;
                    (index.numeric && item.numeric).then_some((index, item))
                })();
                if block.opcode == "data_replaceitemoflist"
                    && let Some((index, item)) = numeric_variables
                {
                    line(
                        output,
                        indent,
                        &format!(
                            "let scratch_list_numeric_pair = match ({}, {}) {{ (ScratchValue::Number(scratch_list_index), ScratchValue::Number(scratch_list_item)) => Some((*scratch_list_index, *scratch_list_item)), _ => None }};",
                            index.read_expression(),
                            item.read_expression()
                        ),
                    );
                    line(
                        output,
                        indent,
                        &format!(
                            "let scratch_list_changed = if let Some((scratch_list_index, scratch_list_item)) = scratch_list_numeric_pair {{ state.list_mut({}).replace_number(scratch_list_index, scratch_list_item) }} else {{ let scratch_list_index: ScratchValue = ({}).clone(); let scratch_list_item: ScratchValue = ({}).clone(); state.list_mut({}).replace(scratch_list_index, scratch_list_item) }};",
                            list.slot
                            , index.read_expression(),
                            item.read_expression(),
                            list.slot
                        ),
                    );
                } else {
                    let item = owned_input(target, input, variables, lists, arguments);
                    let index = owned_input(target, index_input, variables, lists, arguments);
                    line(
                        output,
                        indent,
                        &format!("let scratch_list_item: ScratchValue = {item};"),
                    );
                    line(
                        output,
                        indent,
                        &format!("let scratch_list_index: ScratchValue = {index};"),
                    );
                    if block.opcode == "data_insertatlist" {
                        line(
                            output,
                            indent,
                            &format!(
                                "let scratch_list_changed = state.list_mut({}).insert(scratch_list_item, scratch_list_index);",
                                list.slot
                            ),
                        );
                    } else {
                        line(
                            output,
                            indent,
                            &format!(
                                "let scratch_list_changed = state.list_mut({}).replace(scratch_list_index, scratch_list_item);",
                                list.slot
                            ),
                        );
                    }
                }
                line(
                    output,
                    indent,
                    &format!("state.list_changed_result({}, scratch_list_changed);", list.slot),
                );
            } else {
                line(output, indent, "todo!(\"missing list reference\");");
            }
        }
        "data_showvariable" | "data_hidevariable" => {
            if let Some(variable) = variable_field(block, "VARIABLE", variables) {
                line(
                    output,
                    indent,
                    &format!(
                        "state.set_variable_visible({}, {});",
                        variable.slot,
                        block.opcode == "data_showvariable"
                    ),
                );
            } else {
                line(output, indent, "todo!(\"missing variable reference\");");
            }
        }
        "data_showlist" | "data_hidelist" => {
            if let Some(list) = list_field(block, "LIST", lists) {
                line(
                    output,
                    indent,
                    &format!(
                        "state.set_list_visible({}, {});",
                        list.slot,
                        block.opcode == "data_showlist"
                    ),
                );
            } else {
                line(output, indent, "todo!(\"missing list reference\");");
            }
        }
        "data_setvariableto" => {
            if let Some(variable) = variable_field(block, "VARIABLE", variables) {
                let input = block.inputs.get("VALUE");
                let value = expression(
                    target,
                    input,
                    variables,
                    lists,
                    arguments,
                    &mut HashSet::new(),
                );
                if variable.numeric {
                    let setter = if variable.is_stage {
                        "set_shared_number"
                    } else {
                        "set_local_number"
                    };
                    if let Some(source_variable) =
                        variable_input_info(target, input, variables).filter(|source| source.numeric)
                    {
                        let source = source_variable.read_expression();
                        line(
                            output,
                            indent,
                            &format!(
                                "if let ScratchValue::Number(scratch_numeric_value) = {source} {{ state.{setter}({}, *scratch_numeric_value); }} else {{ let scratch_new_value: ScratchValue = ({source}).clone(); {} }}",
                                variable.slot,
                                variable.write_statement("scratch_new_value")
                            ),
                        );
                    } else {
                        line(
                            output,
                            indent,
                            &format!("state.{setter}({}, {value});", variable.slot),
                        );
                    }
                } else if let Some(source) = variable_input(target, input, variables, lists, arguments) {
                    line(
                        output,
                        indent,
                        &format!("let scratch_new_value: ScratchValue = ({source}).clone();"),
                    );
                    line(
                        output,
                        indent,
                        &variable.write_statement("scratch_new_value"),
                    );
                } else {
                    line(
                        output,
                        indent,
                        &format!("let scratch_new_value: ScratchValue = ({value}).into();"),
                    );
                    line(
                        output,
                        indent,
                        &variable.write_statement("scratch_new_value"),
                    );
                }
            } else {
                line(output, indent, "todo!(\"missing variable reference\");");
            }
        }
        "data_changevariableby" => {
            if let Some(variable) = variable_field(block, "VARIABLE", variables) {
                let value = expression(
                    target,
                    block.inputs.get("VALUE"),
                    variables,
                    lists,
                    arguments,
                    &mut HashSet::new(),
                );
                line(
                    output,
                    indent,
                    &format!("let scratch_change = scratch_to_number({value});"),
                );
                // the runtime fast path falls back for dynamic values while keeping numeric monitor writes allocation-free.
                let method = if variable.is_stage {
                    "change_shared_number"
                } else {
                    "change_local_number"
                };
                line(
                    output,
                    indent,
                    &format!("state.{method}({}, scratch_change);", variable.slot),
                );
            } else {
                line(output, indent, "todo!(\"missing variable reference\");");
            }
        }
        "looks_switchbackdropto" => {
            let value = expression(
                target,
                block.inputs.get("BACKDROP"),
                variables,
                lists,
                arguments,
                &mut HashSet::new(),
            );
            line(output, indent, &format!("state.switch_backdrop({value});"));
        }
        "looks_nextbackdrop" => line(output, indent, "state.next_backdrop();"),
        "looks_say" | "looks_think" => {
            let value = expression(
                target,
                block.inputs.get("MESSAGE"),
                variables,
                lists,
                arguments,
                &mut HashSet::new(),
            );
            line(
                output,
                indent,
                &format!("let scratch_bubble_text = scratch_string({value});"),
            );
            if block.opcode == "looks_say" {
                let name = say_variable_name(target, block.inputs.get("MESSAGE"))
                    .unwrap_or_else(|| "say".to_owned());
                line(
                    output,
                    indent,
                    &format!(
                        "print_value!(state, {}, scratch_bubble_text.clone());",
                        quote(&name)
                    ),
                );
            }
            line(
                output,
                indent,
                &format!(
                    "state.set_speech_bubble(BubbleKind::{}, scratch_bubble_text, None);",
                    if block.opcode == "looks_say" {
                        "Say"
                    } else {
                        "Think"
                    }
                ),
            );
        }
        "looks_sayforsecs" | "looks_thinkforsecs" => {
            let value = expression(
                target,
                block.inputs.get("MESSAGE"),
                variables,
                lists,
                arguments,
                &mut HashSet::new(),
            );
            let duration = expression(
                target,
                block.inputs.get("SECS"),
                variables,
                lists,
                arguments,
                &mut HashSet::new(),
            );
            line(
                output,
                indent,
                &format!("let scratch_bubble_text = scratch_string({value});"),
            );
            if block.opcode == "looks_sayforsecs" {
                let name = say_variable_name(target, block.inputs.get("MESSAGE"))
                    .unwrap_or_else(|| "say".to_owned());
                line(
                    output,
                    indent,
                    &format!(
                        "print_value!(state, {}, scratch_bubble_text.clone());",
                        quote(&name)
                    ),
                );
            }
            line(
                output,
                indent,
                &format!(
                    "let duration_ms = (scratch_to_number({duration}).max(0.0) * 1000.0).ceil() as u64; let wait_deadline = clock.now_millis().saturating_add(duration_ms);"
                ),
            );
            line(
                output,
                indent,
                &format!(
                    "state.set_speech_bubble(BubbleKind::{}, scratch_bubble_text, Some(wait_deadline));",
                    if block.opcode == "looks_sayforsecs" {
                        "Say"
                    } else {
                        "Think"
                    }
                ),
            );
            line(
                output,
                indent,
                &format!(
                    "script.frames.last_mut().unwrap().pc = {};",
                    pc_literal(next_pc)
                ),
            );
            line(
                output,
                indent,
                "return ScriptStatus::WaitUntil(wait_deadline);",
            );
        }
        // lower motion through instance-owned state so fencing and rendering stay centralized.
        "motion_movesteps" => {
            let steps = expression(
                target,
                block.inputs.get("STEPS"),
                variables,
                lists,
                arguments,
                &mut HashSet::new(),
            );
            line(
                output,
                indent,
                &format!("state.move_steps(scratch_to_number({steps}));"),
            );
        }
        "motion_setrotationstyle" => {
            let style = field_value(block, "STYLE")
                .and_then(Value::as_str)
                .unwrap_or("all around");
            line(
                output,
                indent,
                &format!("state.set_rotation_style({});", quote(style)),
            );
        }
        "motion_ifonedgebounce" => {
            line(output, indent, "state.bounce_on_edge();");
        }
        "motion_gotoxy" => {
            let x = expression(
                target,
                block.inputs.get("X"),
                variables,
                lists,
                arguments,
                &mut HashSet::new(),
            );
            let y = expression(
                target,
                block.inputs.get("Y"),
                variables,
                lists,
                arguments,
                &mut HashSet::new(),
            );
            line(
                output,
                indent,
                &format!("state.set_xy(scratch_to_number({x}), scratch_to_number({y}));"),
            );
        }
        "motion_setx" => {
            let x = expression(
                target,
                block.inputs.get("X"),
                variables,
                lists,
                arguments,
                &mut HashSet::new(),
            );
            line(
                output,
                indent,
                &format!(
                    "let (motion_x, motion_y) = state.position(state.current_instance()).unwrap_or((0.0, 0.0)); state.set_xy(scratch_to_number({x}), motion_y);"
                ),
            );
        }
        "motion_sety" => {
            let y = expression(
                target,
                block.inputs.get("Y"),
                variables,
                lists,
                arguments,
                &mut HashSet::new(),
            );
            line(
                output,
                indent,
                &format!(
                    "let (motion_x, motion_y) = state.position(state.current_instance()).unwrap_or((0.0, 0.0)); state.set_xy(motion_x, scratch_to_number({y}));"
                ),
            );
        }
        "motion_pointindirection" => {
            let direction = expression(
                target,
                block.inputs.get("DIRECTION"),
                variables,
                lists,
                arguments,
                &mut HashSet::new(),
            );
            line(
                output,
                indent,
                &format!("state.set_direction(scratch_to_number({direction}));"),
            );
        }
        // apply turns to the scheduled instance and let runtime direction handling normalize the angle.
        "motion_turnright" | "motion_turnleft" => {
            let degrees = expression(
                target,
                block.inputs.get("DEGREES"),
                variables,
                lists,
                arguments,
                &mut HashSet::new(),
            );
            let operator = if block.opcode == "motion_turnright" {
                "+"
            } else {
                "-"
            };
            line(
                output,
                indent,
                &format!(
                    "state.set_direction(state.direction() {operator} scratch_to_number({degrees}));"
                ),
            );
        }
        "motion_pointtowards" => {
            let random_target =
                block
                    .inputs
                    .get("TOWARDS")
                    .and_then(parse_input)
                    .and_then(|input| match input {
                        InputValue::BlockReference(id) => target
                            .blocks
                            .get(id)
                            .and_then(|menu| field_value(menu, "TOWARDS"))
                            .and_then(Value::as_str),
                        InputValue::String(value) | InputValue::Number(value) => value.as_str(),
                        _ => None,
                    })
                    == Some("_random_");
            if random_target {
                line(
                    output,
                    indent,
                    "state.set_direction(scratch_round(scratch_random(-180.0_f64, 180.0_f64)));",
                );
            } else if let Some(position) =
                motion_target_position_expression(target, block, "TOWARDS")
            {
                line(
                    output,
                    indent,
                    &format!(
                        "if let Some((target_x, target_y)) = {position} {{ if let Some((motion_x, motion_y)) = state.position(state.current_instance()) {{ state.set_direction(90.0 - (target_y - motion_y).atan2(target_x - motion_x).to_degrees()); }} }}"
                    ),
                );
            }
        }
        "motion_goto" => {
            if let Some(position) = motion_target_position_expression(target, block, "TO") {
                line(
                    output,
                    indent,
                    &format!("if let Some((motion_x, motion_y)) = {position} {{"),
                );
                line(output, indent + 1, "state.set_xy(motion_x, motion_y);");
                line(output, indent, "}");
            } else {
                line(output, indent, "todo!(\"missing motion target\");");
            }
        }
        "motion_changexby" => {
            let dx = expression(
                target,
                block.inputs.get("DX"),
                variables,
                lists,
                arguments,
                &mut HashSet::new(),
            );
            line(
                output,
                indent,
                &format!("state.change_x_by(scratch_to_number({dx}));"),
            );
        }
        "motion_changeyby" => {
            let dy = expression(
                target,
                block.inputs.get("DY"),
                variables,
                lists,
                arguments,
                &mut HashSet::new(),
            );
            line(
                output,
                indent,
                &format!("state.change_y_by(scratch_to_number({dy}));"),
            );
        }
        "motion_glideto" | "motion_glidesecstoxy" => generate_glide_statement(
            target, block, next_pc, variables, lists, arguments, output, indent,
        ),
        "control_create_clone_of" => {
            let option = block
                .inputs
                .get("CLONE_OPTION")
                .and_then(parse_input)
                .and_then(|input| match input {
                    InputValue::BlockReference(id) => target
                        .blocks
                        .get(id)
                        .and_then(|menu| menu.fields.get("CLONE_OPTION"))
                        .and_then(Value::as_array)
                        .and_then(|field| field.first())
                        .and_then(Value::as_str),
                    InputValue::String(value) | InputValue::Number(value) => value.as_str(),
                    _ => None,
                })
                .unwrap_or("");
            if option == "_myself_" {
                line(
                    output,
                    indent,
                    "let clone_target = Some(state.current_target());",
                );
                line(
                    output,
                    indent,
                    "let clone_source = Some(state.current_instance());",
                );
            } else {
                line(
                    output,
                    indent,
                    &format!(
                        "let clone_target = state.clone_target_for_name({});",
                        quote(option)
                    ),
                );
                line(
                    output,
                    indent,
                    "let clone_source = clone_target.and_then(|target| state.original_instance(target));",
                );
            }
            line(
                output,
                indent,
                "if let (Some(target), Some(source)) = (clone_target, clone_source) { state.create_clone(source, target, 300); }",
            );
        }
        "control_delete_this_clone" => {
            line(
                output,
                indent,
                "if state.delete_current_clone() { return ScriptStatus::Complete; }",
            );
        }
        "procedures_call" => line(
            output,
            indent,
            "todo!(\"procedure calls are lowered as resumable frames\");",
        ),
        opcode => line(
            output,
            indent,
            &format!(
                "todo!({});",
                quote(&format!("unsupported statement: {opcode}"))
            ),
        ),
    }
}

fn broadcast_message_expression(
    target: &ScratchTarget,
    input: Option<&Value>,
    variables: &HashMap<String, VariableInfo>,
    lists: &HashMap<String, ListInfo>,
    arguments: &HashMap<String, String>,
) -> String {
    if let Some(id) = input
        .and_then(Value::as_array)
        .and_then(|input| input.get(1))
        .and_then(Value::as_array)
        .and_then(|primitive| primitive.get(2))
        .and_then(Value::as_str)
        && let Some(message) = target.broadcasts.get(id)
    {
        return quote(message);
    }
    expression(
        target,
        input,
        variables,
        lists,
        arguments,
        &mut HashSet::new(),
    )
}

fn motion_target_position_expression(
    target: &ScratchTarget,
    block: &ScratchBlock,
    input_name: &str,
) -> Option<String> {
    let option = block
        .inputs
        .get(input_name)
        .and_then(parse_input)
        .and_then(|input| match input {
            InputValue::BlockReference(id) => target
                .blocks
                .get(id)
                .and_then(|menu| {
                    field_value(
                        menu,
                        if input_name == "TOWARDS" {
                            "TOWARDS"
                        } else {
                            "TO"
                        },
                    )
                })
                .and_then(Value::as_str),
            InputValue::String(value) | InputValue::Number(value) => value.as_str(),
            _ => None,
        })?;

    Some(match option {
        "_random_" => "Some((scratch_round(scratch_random(-240.5_f64, 240.5_f64)), scratch_round(scratch_random(-179.5_f64, 180.5_f64))))".to_owned(),
        "_mouse_" => "Some((state.mouse_x(), state.mouse_y()))".to_owned(),
        name => format!(
            "state.clone_target_for_name({}).and_then(|target| state.original_instance(target)).and_then(|instance| state.position(instance))",
            quote(name)
        ),
    })
}

#[allow(clippy::too_many_arguments)]
fn generate_glide_statement(
    target: &ScratchTarget,
    block: &ScratchBlock,
    next_pc: usize,
    variables: &HashMap<String, VariableInfo>,
    lists: &HashMap<String, ListInfo>,
    arguments: &HashMap<String, String>,
    output: &mut String,
    indent: usize,
) {
    let seconds = expression(
        target,
        block.inputs.get("SECS"),
        variables,
        lists,
        arguments,
        &mut HashSet::new(),
    );
    line(
        output,
        indent,
        "if script.frames.last().unwrap().glide.is_none() {",
    );

    if block.opcode == "motion_glideto" {
        if let Some(position) = motion_target_position_expression(target, block, "TO") {
            line(
                output,
                indent + 1,
                &format!("if let Some((motion_end_x, motion_end_y)) = {position} {{"),
            );
            line(
                output,
                indent + 2,
                "let motion_start = state.position(state.current_instance()).unwrap_or((0.0, 0.0));",
            );
            line(
                output,
                indent + 2,
                &format!(
                    "script.frames.last_mut().unwrap().glide = Some(GlideState::new(clock.now_millis(), scratch_to_number({seconds}), motion_start, (motion_end_x, motion_end_y)));"
                ),
            );
            line(output, indent + 1, "} else {");
            line(
                output,
                indent + 2,
                &format!(
                    "script.frames.last_mut().unwrap().pc = {};",
                    pc_literal(next_pc)
                ),
            );
            line(output, indent + 2, "continue;");
            line(output, indent + 1, "}");
        } else {
            line(output, indent + 1, "todo!(\"missing glide target\");");
        }
    } else {
        let x = expression(
            target,
            block.inputs.get("X"),
            variables,
            lists,
            arguments,
            &mut HashSet::new(),
        );
        let y = expression(
            target,
            block.inputs.get("Y"),
            variables,
            lists,
            arguments,
            &mut HashSet::new(),
        );
        line(
            output,
            indent + 1,
            "let motion_start = state.position(state.current_instance()).unwrap_or((0.0, 0.0));",
        );
        line(
            output,
            indent + 1,
            &format!("let motion_destination = (scratch_to_number({x}), scratch_to_number({y}));"),
        );
        line(
            output,
            indent + 1,
            &format!(
                "script.frames.last_mut().unwrap().glide = Some(GlideState::new(clock.now_millis(), scratch_to_number({seconds}), motion_start, motion_destination));"
            ),
        );
    }
    line(output, indent, "}");
    line(
        output,
        indent,
        "let motion_glide = script.frames.last().unwrap().glide.unwrap();",
    );
    line(
        output,
        indent,
        "if let Some((motion_x, motion_y)) = motion_glide.position_at(clock.now_millis()) {",
    );
    line(output, indent + 1, "state.set_xy(motion_x, motion_y);");
    line(
        output,
        indent + 1,
        "if script.yield_at_safe_point(clock) { return ScriptStatus::Yield; }",
    );
    line(output, indent + 1, "continue;");
    line(output, indent, "} else {");
    line(
        output,
        indent + 1,
        "let (motion_x, motion_y) = motion_glide.destination();",
    );
    line(output, indent + 1, "state.set_xy(motion_x, motion_y);");
    line(
        output,
        indent + 1,
        "script.frames.last_mut().unwrap().glide = None;",
    );
    line(
        output,
        indent + 1,
        &format!(
            "script.frames.last_mut().unwrap().pc = {};",
            pc_literal(next_pc)
        ),
    );
    line(output, indent + 1, "continue;");
    line(output, indent, "}");
}

fn procedure_for_call<'a>(
    block: &ScratchBlock,
    procedures: &'a [ProcedureSignature],
) -> Option<&'a ProcedureSignature> {
    let proccode = block.mutation.as_object()?.get("proccode")?.as_str()?;
    procedures
        .iter()
        .find(|procedure| procedure.proccode == proccode)
}

// resolve direct variable inputs so assignments copy the current dynamic value.
fn variable_input(
    target: &ScratchTarget,
    input: Option<&Value>,
    variables: &HashMap<String, VariableInfo>,
    lists: &HashMap<String, ListInfo>,
    arguments: &HashMap<String, String>,
) -> Option<String> {
    match input.and_then(parse_input)? {
        InputValue::VariableReference(reference) => variables
            .get(reference.id)
            .map(VariableInfo::read_expression),
        InputValue::BlockReference(id) => {
            let reporter = target.blocks.get(id)?;
            if reporter.opcode == "data_variable" {
                variable_field(reporter, "VARIABLE", variables)
                    .map(|variable| variable.read_expression())
            } else if matches!(
                reporter.opcode.as_str(),
                "argument_reporter_string_number" | "argument_reporter_boolean"
            ) {
                field_value(reporter, "VALUE")
                    .and_then(Value::as_str)
                    .and_then(|name| arguments.get(name).cloned())
            } else if reporter.opcode == "data_itemoflist" {
                let list = list_field(reporter, "LIST", lists)?;
                let index = expression(
                    target,
                    reporter.inputs.get("INDEX"),
                    variables,
                    lists,
                    arguments,
                    &mut HashSet::new(),
                );
                Some(format!("state.list({}).get({index})", list.slot))
            } else {
                None
            }
        }
        _ => None,
    }
}

// return identity and static numeric proof for direct scalar variable inputs.
fn variable_input_info(
    target: &ScratchTarget,
    input: Option<&Value>,
    variables: &HashMap<String, VariableInfo>,
) -> Option<VariableInfo> {
    match input.and_then(parse_input)? {
        InputValue::VariableReference(reference) => variables.get(reference.id).cloned(),
        InputValue::BlockReference(id) => {
            let reporter = target.blocks.get(id)?;
            (reporter.opcode == "data_variable")
                .then(|| variable_field(reporter, "VARIABLE", variables))?
        }
        _ => None,
    }
}

fn owned_input(
    target: &ScratchTarget,
    input: Option<&Value>,
    variables: &HashMap<String, VariableInfo>,
    lists: &HashMap<String, ListInfo>,
    arguments: &HashMap<String, String>,
) -> String {
    if let Some(source) = variable_input(target, input, variables, lists, arguments) {
        format!("({source}).clone()")
    } else {
        let value = expression(
            target,
            input,
            variables,
            lists,
            arguments,
            &mut HashSet::new(),
        );
        format!("({value}).into()")
    }
}

// lower fixed pen dropdowns to enum values so repeated blocks avoid string matching.
fn pen_color_parameter_input(
    target: &ScratchTarget,
    input: Option<&Value>,
    variables: &HashMap<String, VariableInfo>,
    lists: &HashMap<String, ListInfo>,
    arguments: &HashMap<String, String>,
) -> String {
    let static_value = match input.and_then(parse_input) {
        None | Some(InputValue::Empty) => Some("color"),
        Some(InputValue::String(value)) => value.as_str(),
        Some(InputValue::BlockReference(id)) => match target.blocks.get(id) {
            Some(block) if block.opcode == "pen_menu_colorParam" => {
                field_value(block, "colorParam").and_then(Value::as_str)
            }
            _ => None,
        },
        Some(InputValue::Number(_))
        | Some(InputValue::VariableReference(_))
        | Some(InputValue::ListReference(_)) => None,
    };

    if let Some(value) = static_value {
        let variant = match value {
            "color" => "Color",
            "saturation" => "Saturation",
            "brightness" => "Brightness",
            "transparency" => "Transparency",
            _ => "Unsupported",
        };
        return format!("crate::runtime::PenColorParameter::{variant}");
    }

    let value = expression(
        target,
        input,
        variables,
        lists,
        arguments,
        &mut HashSet::new(),
    );
    format!("scratch_string({value})")
}

// preserve hsv state whenever any block on this target can read or change it.
fn target_has_pen_hsv_controls(target: &ScratchTarget) -> bool {
    target.blocks.values().any(|block| {
        matches!(
            block.opcode.as_str(),
            "pen_changePenColorParamBy"
                | "pen_setPenColorParamTo"
                | "pen_changePenHueBy"
                | "pen_setPenHueToNumber"
                | "pen_changePenShadeBy"
                | "pen_setPenShadeToNumber"
        )
    })
}

fn list_field(
    block: &ScratchBlock,
    field: &str,
    lists: &HashMap<String, ListInfo>,
) -> Option<ListInfo> {
    let data = block.fields.get(field)?.as_array()?;
    lists.get(data.get(1)?.as_str()?).cloned()
}

// resolve a block's variable field ID to its allocated Rust identifier.
pub(super) fn variable_field(
    block: &ScratchBlock,
    field: &str,
    variables: &HashMap<String, VariableInfo>,
) -> Option<VariableInfo> {
    let data = block.fields.get(field)?.as_array()?;
    variables.get(data.get(1)?.as_str()?).cloned()
}

// get the original Scratch variable name displayed by a say reporter.
fn say_variable_name(target: &ScratchTarget, input: Option<&Value>) -> Option<String> {
    match input.and_then(parse_input)? {
        InputValue::VariableReference(reference) => Some(reference.name.to_owned()),
        InputValue::BlockReference(id) => {
            let reporter = target.blocks.get(id)?;
            if reporter.opcode != "data_variable" {
                return None;
            }
            field_value(reporter, "VARIABLE")?
                .as_str()
                .map(str::to_owned)
        }
        _ => None,
    }
}
