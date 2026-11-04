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
        expression, expression_argument, local_expression, local_to_bool, local_to_number,
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

// emit a resumable step function for one green flag script.
pub(super) fn generate_resumable_script(
    target: &ScratchTarget,
    first: Option<&str>,
    variables: &HashMap<String, VariableInfo>,
    lists: &HashMap<String, ListInfo>,
    procedures: &[ProcedureSignature],
    function_name: &str,
    fast_math: bool,
    output: &mut String,
) -> (usize, usize) {
    let mut graph = ScriptGraph::new();
    graph.add_chain(target, first, usize::MAX);
    let start_pc = first
        .and_then(|id| graph.block_pcs.get(id).copied())
        .unwrap_or(usize::MAX);
    let mut procedure_graphs = Vec::new();
    let mut procedure_entries = HashMap::new();
    for procedure in procedures {
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
            line(
                output,
                4,
                &format!("let repeat_count = scratch_round({times}).max(0.0) as usize;"),
            );
            line(
                output,
                4,
                &format!("script.frames.last_mut().unwrap().repeat_counts[{slot}] = repeat_count;"),
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
                indent + 1,
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
                "if state.redraw_requested() { return ScriptStatus::Yield; }",
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

    let values = procedure
        .argument_ids
        .iter()
        .enumerate()
        .map(|(index, argument_id)| {
            let input = block.inputs.get(argument_id);
            if input.is_none() {
                return format!(
                    "todo!({})",
                    quote(&format!("missing procedure argument {index}"))
                );
            }
            let value = expression(
                target,
                input,
                variables,
                lists,
                arguments,
                &mut HashSet::new(),
            );
            format!("({value}).to_owned().into()")
        })
        .collect::<Vec<_>>();
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
                "state.set_pen_rgb_color(scratch_pen_color);",
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
        "data_addtolist" => {
            if let Some(list) = list_field(block, "LIST", lists) {
                line(
                    output,
                    indent,
                    &format!(
                        "let scratch_list_before = state.list_before_change({});",
                        list.slot
                    ),
                );
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
                    &format!("state.list_mut({}).add(scratch_list_item);", list.slot),
                );
                line(
                    output,
                    indent,
                    &format!("state.list_changed({}, scratch_list_before);", list.slot),
                );
            } else {
                line(output, indent, "todo!(\"missing list reference\");");
            }
        }
        "data_deleteoflist" => {
            if let Some(list) = list_field(block, "LIST", lists) {
                line(
                    output,
                    indent,
                    &format!(
                        "let scratch_list_before = state.list_before_change({});",
                        list.slot
                    ),
                );
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
                    &format!("state.list_mut({}).delete(scratch_list_index);", list.slot),
                );
                line(
                    output,
                    indent,
                    &format!("state.list_changed({}, scratch_list_before);", list.slot),
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
                        "let scratch_list_before = state.list_before_change({});",
                        list.slot
                    ),
                );
                line(
                    output,
                    indent,
                    &format!("state.list_mut({}).delete_all();", list.slot),
                );
                line(
                    output,
                    indent,
                    &format!("state.list_changed({}, scratch_list_before);", list.slot),
                );
            } else {
                line(output, indent, "todo!(\"missing list reference\");");
            }
        }
        "data_insertatlist" | "data_replaceitemoflist" => {
            if let Some(list) = list_field(block, "LIST", lists) {
                line(
                    output,
                    indent,
                    &format!(
                        "let scratch_list_before = state.list_before_change({});",
                        list.slot
                    ),
                );
                let input = block.inputs.get("ITEM");
                let item = owned_input(target, input, variables, lists, arguments);
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
                            "state.list_mut({}).insert(scratch_list_item, scratch_list_index);",
                            list.slot
                        ),
                    );
                } else {
                    line(
                        output,
                        indent,
                        &format!(
                            "state.list_mut({}).replace(scratch_list_index, scratch_list_item);",
                            list.slot
                        ),
                    );
                }
                line(
                    output,
                    indent,
                    &format!("state.list_changed({}, scratch_list_before);", list.slot),
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
                if let Some(source) = variable_input(target, input, variables, lists, arguments) {
                    line(
                        output,
                        indent,
                        &format!("let scratch_new_value: ScratchValue = ({source}).clone();"),
                    );
                } else {
                    line(
                        output,
                        indent,
                        &format!("let scratch_new_value: ScratchValue = ({value}).into();"),
                    );
                }
                line(
                    output,
                    indent,
                    &variable.write_statement("scratch_new_value"),
                );
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
                if variable.is_stage {
                    line(
                        output,
                        indent,
                        &format!("let scratch_change = scratch_to_number({value});"),
                    );
                    line(
                        output,
                        indent,
                        &format!("state.change_shared({}, scratch_change);", variable.slot),
                    );
                } else {
                    line(
                        output,
                        indent,
                        &format!("let scratch_change = scratch_to_number({value});"),
                    );
                    line(
                        output,
                        indent,
                        &format!("state.change_local_by({}, scratch_change);", variable.slot),
                    );
                }
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
