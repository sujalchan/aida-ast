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
    expressions::{constant_numeric_input, expression},
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

impl VariableInfo {
    pub(super) fn read_expression(&self) -> String {
        let method = if self.is_stage { "get_shared" } else { "get" };
        format!("state.{method}({})", self.slot)
    }

    pub(super) fn write_statement(&self, value: &str) -> String {
        let method = if self.is_stage { "set_shared" } else { "set" };
        format!("state.{method}({}, {value});", self.slot)
    }
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
    generate_with_options(project, false)
}

pub fn generate_with_options(project: &ScratchProject, fast_math: bool) -> String {
    let variables = allocate_variables(project);
    let lists = allocate_lists(project);
    let uses_broadcasts = project.targets.iter().any(|target| {
        target.blocks.values().any(|block| {
            matches!(
                block.opcode.as_str(),
                "event_broadcast" | "event_broadcastandwait" | "event_whenbroadcastreceived"
            )
        })
    });
    let uses_clones = project.targets.iter().any(|target| {
        target.blocks.values().any(|block| {
            matches!(
                block.opcode.as_str(),
                "control_create_clone_of" | "control_delete_this_clone" | "control_start_as_clone"
            )
        })
    });
    // omit audio polling entirely from projects without executable sound blocks.
    let uses_sound = project.targets.iter().any(|target| {
        target.blocks.values().any(|block| {
            matches!(
                block.opcode.as_str(),
                "sound_play"
                    | "sound_playuntildone"
                    | "sound_stopallsounds"
                    | "sound_seteffectto"
                    | "sound_changeeffectby"
                    | "sound_cleareffects"
                    | "sound_changevolumeby"
                    | "sound_setvolumeto"
                    | "sound_volume"
            )
        })
    });
    // emit renderer input and edge polling only for projects that use those hats.
    let uses_click_hats = project.targets.iter().any(|target| {
        target.blocks.values().any(|block| {
            matches!(
                block.opcode.as_str(),
                "event_whenthisspriteclicked" | "event_whenstageclicked"
            )
        })
    });
    let uses_edge_hats = project.targets.iter().any(|target| {
        target
            .blocks
            .values()
            .any(|block| block.opcode == "event_whengreaterthan")
    });
    let uses_backdrop_switches = project.targets.iter().any(|target| {
        target.blocks.values().any(|block| {
            matches!(
                block.opcode.as_str(),
                "looks_switchbackdropto" | "looks_nextbackdrop"
            )
        })
    });
    let uses_timed_speech = project.targets.iter().any(|target| {
        target.blocks.values().any(|block| {
            matches!(
                block.opcode.as_str(),
                "looks_sayforsecs" | "looks_thinkforsecs"
            )
        })
    });
    // timer hats share the same project clock used by Scratch timer reporters.
    let uses_project_timer = project.targets.iter().any(|target| {
        target.blocks.values().any(|block| {
            matches!(
                block.opcode.as_str(),
                "sensing_timer" | "sensing_resettimer"
            ) || (block.opcode == "event_whengreaterthan"
                && block
                    .fields
                    .get("WHENGREATERTHANMENU")
                    .and_then(serde_json::Value::as_array)
                    .and_then(|field| field.first())
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|value| value.eq_ignore_ascii_case("TIMER")))
        })
    }) || project
        .monitors
        .iter()
        .any(|monitor| monitor.opcode == "sensing_timer");
    let has_visible_timer_monitor = project
        .monitors
        .iter()
        .any(|monitor| monitor.opcode == "sensing_timer" && monitor.visible);
    let has_live_sensing_monitors = project.monitors.iter().any(|monitor| {
        monitor.visible
            && matches!(
                monitor.opcode.as_str(),
                "sensing_current" | "sensing_dayssince2000"
            )
    });
    let has_sound_volume_monitor = project
        .monitors
        .iter()
        .any(|monitor| monitor.opcode == "sound_volume");
    let has_sound_volume_setter = project.targets.iter().any(|target| {
        target.blocks.values().any(|block| {
            matches!(
                block.opcode.as_str(),
                "sound_setvolumeto" | "sound_changevolumeby"
            )
        })
    });
    let expire_bubbles = if uses_timed_speech {
        "self.state.expire_speech_bubbles(self.scheduler.now_millis()); "
    } else {
        ""
    };
    let mut output = String::new();

    generate_green_flag_functions(project, &variables, &lists, fast_math, &mut output);
    generate_scripts(project, &variables, &lists, &mut output);

    // refresh volume monitor snapshots only for projects that can change volume.
    if has_sound_volume_setter {
        output.push_str(&format!(
            "const REFRESH_SOUND_VOLUME_MONITOR: bool = {has_sound_volume_monitor};\n"
        ));
    }

    let edge_state_field = if uses_edge_hats {
        ", edge_hat_previous: EdgeHatState"
    } else {
        ""
    };
    let edge_state_init = if uses_edge_hats {
        ", edge_hat_previous: EdgeHatState::default()"
    } else {
        ""
    };
    output.push_str(&format!("#[allow(dead_code)]\npub struct GeneratedProgram {{ state: ScratchState, scheduler: ScriptScheduler<GeneratedScript>, frame_order: Vec<u64>{edge_state_field} }}\n\n#[allow(dead_code)]\nimpl GeneratedProgram {{\n    pub fn new() -> Self {{ Self::build(None) }}\n    pub fn new_with_clock(clock: Box<dyn Clock>) -> Self {{ Self::build(Some(clock)) }}\n    pub fn uses_sound() -> bool {{ {uses_sound} }}\n    pub fn set_sound_library(&mut self, library: SoundLibrary) {{ self.state.set_sound_library(library); }}\n    pub fn enable_realtime_audio(&mut self) -> Result<(), String> {{ self.state.set_realtime_audio(RealtimeAudio::open()?); Ok(()) }}\n    fn build(clock: Option<Box<dyn Clock>>) -> Self {{\n"));

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
            "ScratchVariableMetadata::for_target({}, {}, {}, {}, {}).with_monitor_visible({})",
            quote(id),
            quote(&info.name),
            quote(&info.target),
            info.target_index,
            info.is_stage,
            project
                .monitors
                .iter()
                .any(|monitor| monitor.visible && monitor.id == *id)
        );
    }
    let mut initial_lists = vec![String::new(); lists.len()];
    let mut list_metadata = vec![String::new(); lists.len()];
    for (id, info) in &lists {
        let list = project
            .targets
            .get(info.target_index)
            .and_then(|target| target.lists.get(id))
            .expect("every allocated list has saved contents");
        let serialized_items = serde_json::to_string(&list.items)
            .expect("Scratch list values can be serialized as JSON");
        initial_lists[info.slot] = format!("ScratchList::from_json({})", quote(&serialized_items));
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
            "let mut state = ScratchState::with_project_data(vec![{}], vec![{}], vec![{}], vec![{}], vec![{}]);",
            initial_values.join(", "),
            variable_metadata.join(", "),
            initial_lists.join(", "),
            list_metadata.join(", "),
            lists
                .iter()
                .map(|(id, _)| project
                    .monitors
                    .iter()
                    .any(|monitor| monitor.visible && monitor.id == *id)
                    .to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ),
    );
    if has_visible_timer_monitor {
        line(
            &mut output,
            1,
            "state.set_project_timer_monitor_visible(true);",
        );
    }

    if let Some(stage) = project.targets.iter().find(|target| target.is_stage) {
        let names = stage
            .costumes
            .iter()
            .map(|costume| format!("{}.to_owned()", quote(&costume.name)))
            .collect::<Vec<_>>()
            .join(", ");
        line(
            &mut output,
            1,
            &format!(
                "state.set_backdrops(vec![{names}], {});",
                stage.current_costume
            ),
        );
    }

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
        line(
            &mut output,
            1,
            &format!(
                "state.set_original_volume({target_index}, {}_f64);",
                target.volume
            ),
        );
        line(
            &mut output,
            1,
            &format!(
                "state.set_original_xy({target_index}, {}_f64, {}_f64);",
                target.x, target.y
            ),
        );
        line(
            &mut output,
            1,
            &format!(
                "state.set_original_visual({target_index}, {}_f64, {}_f64, {});",
                target.direction, target.size, target.visible
            ),
        );
        let costume_names = target
            .costumes
            .iter()
            .map(|costume| format!("{}.to_owned()", quote(&costume.name)))
            .collect::<Vec<_>>()
            .join(", ");
        line(
            &mut output,
            1,
            &format!(
                "state.set_target_costumes({target_index}, vec![{costume_names}], {});",
                target.current_costume
            ),
        );
    }

    output.push_str("let scripts = new_scripts();\nlet scheduler = match clock { Some(clock) => ScriptScheduler::with_clock(scripts, clock), None => ScriptScheduler::new(scripts) };\n");
    if uses_project_timer {
        output.push_str("state.reset_project_timer(scheduler.now_millis());\n");
    }
    output.push_str(&format!(
        "    Self {{ state, scheduler, frame_order: Vec::new(){edge_state_init} }}\n    }}\n\n"
    ));
    output.push_str(
        "    pub fn step(&mut self) -> bool { self.advance() == SchedulerStatus::Progressed }\n",
    );
    if uses_edge_hats {
        output.push_str("    fn update_edge_hats(&mut self) { let starters = poll_edge_hats(&mut self.state, self.scheduler.now_millis(), &mut self.edge_hat_previous); self.scheduler.start_edge_hats(starters); }\n");
    }
    output.push_str("    fn process_after_step(&mut self) {");
    if uses_broadcasts {
        output.push_str(" if let Some((sender, message, wait)) = self.scheduler.take_broadcast() { let receivers = new_broadcast_scripts(message, &self.state); self.scheduler.broadcast(sender, receivers, wait); }");
    }
    if uses_clones {
        output.push_str(" let deleted = self.state.current_instance(); if !self.state.instance_is_alive(deleted) { self.scheduler.remove_target_scripts(deleted); } for instance in self.state.take_pending_clones() { if let Some(target) = self.state.instance_target(instance) { self.scheduler.add_scripts(new_clone_scripts(target, instance)); } }");
    }
    if uses_backdrop_switches {
        output.push_str(" for backdrop in self.state.take_pending_backdrop_events() { let receivers = new_backdrop_scripts(&backdrop, &self.state); self.scheduler.add_scripts(receivers); }");
    }
    output.push_str(" }\n");
    let edge_poll = if uses_edge_hats {
        "self.update_edge_hats(); "
    } else {
        ""
    };
    let sound_poll = if uses_sound {
        "self.state.advance_sounds(self.scheduler.now_millis()); for sound_wait in self.state.drain_finished_sounds() { self.scheduler.signal(sound_wait); } "
    } else {
        ""
    };
    let sound_complete = if uses_sound {
        "if status == SchedulerStatus::Complete && self.state.has_active_sounds() { SchedulerStatus::WaitingForSignal } else { status }"
    } else {
        "status"
    };
    output.push_str("    pub fn advance(&mut self) -> SchedulerStatus { ");
    output.push_str(sound_poll);
    output.push_str(edge_poll);
    output.push_str(
        "let status = self.scheduler.advance(&mut self.state); self.process_after_step(); ",
    );
    output.push_str(sound_complete);
    output.push_str(" }\n");
    if uses_broadcasts || uses_clones || uses_backdrop_switches {
        output.push_str(&format!("    pub fn advance_frame(&mut self) -> SchedulerStatus {{ {sound_poll}{expire_bubbles}{edge_poll}self.scheduler.script_ids_in_frame_order_into(&mut self.frame_order); let mut cursor = 0; let mut progressed = false; while cursor < self.frame_order.len() {{ let id = self.frame_order[cursor]; cursor += 1; if let Some(status) = self.scheduler.advance_script_id(id, &mut self.state) {{ if status == SchedulerStatus::Complete {{ return if self.state.has_active_sounds() {{ SchedulerStatus::WaitingForSignal }} else {{ SchedulerStatus::Complete }}; }} progressed = true; self.process_after_step(); }} }} let status = if progressed {{ SchedulerStatus::Progressed }} else {{ self.scheduler.idle_status() }}; {sound_complete} }}\n"));
    } else {
        output.push_str(&format!("    pub fn advance_frame(&mut self) -> SchedulerStatus {{ {sound_poll}{expire_bubbles}{edge_poll}self.scheduler.script_ids_in_frame_order_into(&mut self.frame_order); let mut cursor = 0; let mut progressed = false; while cursor < self.frame_order.len() {{ let id = self.frame_order[cursor]; cursor += 1; if let Some(status) = self.scheduler.advance_script_id(id, &mut self.state) {{ if status == SchedulerStatus::Complete {{ return if self.state.has_active_sounds() {{ SchedulerStatus::WaitingForSignal }} else {{ SchedulerStatus::Complete }}; }} progressed = true; }} }} let status = if progressed {{ SchedulerStatus::Progressed }} else {{ self.scheduler.idle_status() }}; {sound_complete} }}\n"));
    }
    output.push_str("    pub fn duration_until(&self, deadline: u64) -> std::time::Duration { self.scheduler.duration_until(deadline) }\n");
    if uses_project_timer {
        let clear_edges = if uses_edge_hats {
            "self.edge_hat_previous.clear(); "
        } else {
            ""
        };
        let restart_sounds = if uses_sound {
            "self.state.restart_sounds(self.scheduler.now_millis()); "
        } else {
            ""
        };
        output.push_str(&format!("    pub fn restart(&mut self) {{ {clear_edges}{restart_sounds}self.state.remove_clones(); self.scheduler.restart(new_scripts()); self.state.reset_project_timer(self.scheduler.now_millis()); }}\n"));
    } else {
        let clear_edges = if uses_edge_hats {
            "self.edge_hat_previous.clear(); "
        } else {
            ""
        };
        let restart_sounds = if uses_sound {
            "self.state.restart_sounds(self.scheduler.now_millis()); "
        } else {
            ""
        };
        output.push_str(&format!("    pub fn restart(&mut self) {{ {clear_edges}{restart_sounds}self.state.remove_clones(); self.scheduler.restart(new_scripts()); }}\n"));
    }
    // stopping scripts leaves edge predicates intact so true hats cannot fire again immediately.
    output
        .push_str("    pub fn stop_all(&mut self) { self.scheduler.stop_all(&mut self.state); }\n");
    if uses_sound {
        output.push_str("    pub fn is_complete(&self) -> bool { self.scheduler.active_scripts() == 0 && !self.state.has_active_sounds() }\n");
    } else {
        output.push_str(
            "    pub fn is_complete(&self) -> bool { self.scheduler.active_scripts() == 0 }\n",
        );
    }
    output.push_str(
        "    pub fn active_script_count(&self) -> usize { self.scheduler.active_scripts() }\n",
    );
    output.push_str("    pub fn state(&self) -> &ScratchState { &self.state }\n");
    output.push_str("    pub fn set_username(&mut self, username: String) { self.state.set_username(username); }\n");
    output.push_str(&format!(
        "    pub fn has_live_sensing_monitors(&self) -> bool {{ {has_live_sensing_monitors} }}\n"
    ));
    output.push_str("    pub fn set_target_fence_geometry(&mut self, target: usize, geometry: SpriteFenceGeometry) { self.state.set_target_fence_geometry(target, geometry); }\n");
    output.push_str("    pub fn begin_tick(&mut self) { self.state.begin_tick(); }\n");
    output
        .push_str("    pub fn redraw_requested(&self) -> bool { self.state.redraw_requested() }\n");
    output.push_str("    pub fn presentation_requested(&self) -> bool { self.state.presentation_requested() }\n");
    output.push_str(
        "    pub fn presentation_consumed(&mut self) { self.state.presentation_consumed(); }\n",
    );
    // pass key events through the scheduler's non-restarting per-instance hat gate.
    if uses_click_hats {
        output.push_str("    pub fn apply_input(&mut self, event: RuntimeInputEvent) { match event { RuntimeInputEvent::PointerButton { pressed, inside_stage, target_instance, draggable, was_dragged, monitor_interaction } => { if pointer_click_activates(pressed, inside_stage, draggable, was_dragged, monitor_interaction) { self.scheduler.start_click_hats(new_click_scripts(target_instance, &self.state)); } }, event => if let Some(key) = self.state.apply_input(event) { let scripts = new_keypress_scripts(&key, &self.state); self.scheduler.start_key_hats(scripts); } } }\n");
    } else {
        output.push_str("    pub fn apply_input(&mut self, event: RuntimeInputEvent) { if let Some(key) = self.state.apply_input(event) { let scripts = new_keypress_scripts(&key, &self.state); self.scheduler.start_key_hats(scripts); } }\n");
    }
    output.push_str(
        "    pub fn set_loudness(&mut self, value: f64) { self.state.set_loudness(value); }\n",
    );
    let monitor_reporters = reporter_monitor_values(project, &variables, &lists);
    let snapshot_base = if uses_project_timer {
        "self.state.observable_snapshot_with_timer(tick, self.scheduler.now_millis())"
    } else {
        "self.state.observable_snapshot(tick)"
    };
    // reporter monitors with connected inputs temporarily use their declared target context.
    output.push_str(&format!("    pub fn observable_snapshot(&mut self, tick: usize) -> RuntimeSnapshot {{ let mut snapshot = {snapshot_base};\n"));
    if !monitor_reporters.is_empty() {
        output.push_str(&format!(
            "        snapshot.reporters = Vec::with_capacity({});\n",
            monitor_reporters.len()
        ));
        for (key, expression, target_context) in monitor_reporters {
            if let Some(target_index) = target_context {
                line(
                    &mut output,
                    2,
                    &format!(
                        "{{ let previous_instance = self.state.current_instance(); let value: ScratchValue = if let Some(instance) = self.state.original_instance({target_index}) {{ self.state.set_current_instance(instance); let value = {{ let state = &self.state; let _ = state; ({expression}).into() }}; self.state.set_current_instance(previous_instance); value }} else {{ let state = &self.state; let _ = state; ({expression}).into() }}; snapshot.reporters.push(ReporterSnapshot {{ key: {}.to_owned(), value }}); }}",
                        quote(&key)
                    ),
                );
            } else {
                line(
                    &mut output,
                    2,
                    &format!(
                        "{{ let state = &self.state; let _ = state; let value: ScratchValue = ({}).into(); snapshot.reporters.push(ReporterSnapshot {{ key: {}.to_owned(), value }}); }}",
                        expression,
                        quote(&key),
                    ),
                );
            }
        }
    }
    output.push_str("        snapshot }\n");
    output.push_str("    pub fn take_pending_pen_commands(&mut self) -> Vec<PenCommand> { self.state.take_pending_pen_commands() }\n");
    output.push_str("    pub fn set_observable_event_sink(&mut self, sink: Option<std::sync::mpsc::Sender<ObservableEvent>>) { self.state.set_event_sink(sink); }\n");
    output.push_str("    pub fn run_until_complete(&mut self) { loop { match self.advance_frame() { SchedulerStatus::Progressed => {}, SchedulerStatus::WaitingUntil(deadline) => std::thread::sleep(self.duration_until(deadline)), SchedulerStatus::WaitingForSignal => std::thread::sleep(std::time::Duration::from_millis(8)), SchedulerStatus::Complete => break } } }\n}\n\n");
    output.push_str("#[allow(dead_code)]\npub fn run() { let mut program = GeneratedProgram::new(); program.run_until_complete(); }\n");
    let identifiers = rust_identifiers(&output);
    let imports = runtime_imports(&identifiers);
    let print_value_macro = if identifiers.contains("print_value") {
        "macro_rules! print_value {\n    ($state:expr, $name:expr, $value:expr) => {{\n        let value = $value;\n        $state.emit_observable_output($name, &value);\n    }};\n}\n\n"
    } else {
        ""
    };
    format!("{imports}{print_value_macro}{output}")
}

fn runtime_imports(identifiers: &HashSet<String>) -> String {
    const IMPORTS: &[&str] = &[
        "BubbleKind",
        "scratch_acos_degrees",
        "fast_acos_degrees",
        "fast_asin_degrees",
        "fast_atan_degrees",
        "fast_cos",
        "fast_sin",
        "fast_tan",
        "scratch_add",
        "scratch_and",
        "scratch_asin_degrees",
        "scratch_atan_degrees",
        "scratch_bool",
        "scratch_contains",
        "scratch_cos",
        "scratch_divide",
        "scratch_equals",
        "scratch_greater_than",
        "scratch_join",
        "scratch_length",
        "scratch_less_than",
        "scratch_letter",
        "scratch_mod",
        "scratch_multiply",
        "scratch_not",
        "scratch_number_operand",
        "scratch_or",
        "scratch_random",
        "scratch_random_constant",
        "with_scratch_random",
        "scratch_round",
        "scratch_sin",
        "scratch_string",
        "scratch_subtract",
        "scratch_tan",
        "scratch_to_number",
        "Clock",
        "GlideState",
        "ObservableEvent",
        "PenCommand",
        "RealtimeAudio",
        "ResumableScript",
        "ScratchList",
        "ScratchListMetadata",
        "ScratchState",
        "ScratchSoundEffect",
        "SoundLibrary",
        "SpriteFenceGeometry",
        "ScratchValue",
        "ScratchVariableMetadata",
        "RuntimeSnapshot",
        "ReporterSnapshot",
        "RuntimeInputEvent",
        "SchedulerStatus",
        "ScriptScheduler",
        "ScriptStatus",
        "EdgeHatState",
        "pointer_click_activates",
        "WarpBudget",
    ];
    let used = IMPORTS
        .iter()
        .copied()
        .filter(|name| identifiers.contains(*name))
        .collect::<Vec<_>>();
    format!("use crate::runtime::{{{}}};\n\n", used.join(", "))
}

fn reporter_monitor_values(
    project: &ScratchProject,
    variables: &HashMap<String, VariableInfo>,
    lists: &HashMap<String, ListInfo>,
) -> Vec<(String, String, Option<usize>)> {
    project
        .monitors
        .iter()
        .filter_map(|monitor| {
            let monitor_target = project.targets.iter().enumerate().find(|(_, target)| {
                monitor
                    .sprite_name
                    .as_deref()
                    .map_or(target.is_stage, |sprite| target.name == sprite)
            });
            let mut target_context = None;
            let expression = match monitor.opcode.as_str() {
                "sensing_current" => {
                    let method = match monitor
                        .params
                        .get("CURRENTMENU")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_ascii_uppercase()
                        .as_str()
                    {
                        "YEAR" => "current_year",
                        "MONTH" => "current_month",
                        "DATE" => "current_date",
                        "DAYOFWEEK" => "current_day_of_week",
                        "HOUR" => "current_hour",
                        "MINUTE" => "current_minute",
                        "SECOND" => "current_second",
                        _ => "",
                    };
                    if method.is_empty() {
                        "0.0".to_owned()
                    } else {
                        format!("self.state.{method}()")
                    }
                }
                "sensing_dayssince2000" => "self.state.days_since_2000()".to_owned(),
                "sensing_online" => "false".to_owned(),
                "sensing_username" => "self.state.username()".to_owned(),
                "sensing_mousex" => "self.state.mouse_x()".to_owned(),
                "sensing_mousey" => "self.state.mouse_y()".to_owned(),
                "sensing_mousedown" => "self.state.mouse_down()".to_owned(),
                "sensing_loudness" => "self.state.loudness()".to_owned(),
                "sound_volume" => format!(
                    "self.state.sound_volume_for_target({})",
                    monitor_target.map_or(0, |(index, _)| index)
                ),
                "sensing_keypressed" => {
                    // a nonstandard saved monitor may point back to its full reporter block.
                    let active_block = monitor_target.and_then(|(index, target)| {
                        target
                            .blocks
                            .get(&monitor.id)
                            .map(|block| (index, target, block))
                    });
                    if let Some((index, target, block)) = active_block {
                        target_context = Some(index);
                        format!(
                            "state.key_pressed({})",
                            expression(
                                target,
                                block.inputs.get("KEY_OPTION"),
                                variables,
                                lists,
                                &HashMap::new(),
                                &mut HashSet::new(),
                            )
                        )
                    } else {
                        let key = monitor
                            .params
                            .get("KEY_OPTION")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or_default();
                        format!("state.key_pressed({})", quote(key))
                    }
                }
                // these reporters have their own snapshot paths.
                "data_variable"
                | "data_listcontents"
                | "sensing_timer"
                | "motion_xposition"
                | "motion_yposition"
                | "motion_direction"
                | "looks_size"
                | "looks_backdropnumbername"
                | "looks_costumenumbername" => return None,
                _ => return None,
            };
            let target_name = monitor_target.map(|(_, target)| target.name.as_str());
            let key = target_name.map_or_else(
                || format!("__reporter/{}/{}", monitor.opcode, monitor.id),
                |target| format!("__reporter/{target}/{}/{}", monitor.opcode, monitor.id),
            );
            Some((key, expression, target_context))
        })
        .collect()
}

fn rust_identifiers(source: &str) -> HashSet<String> {
    let characters = source.chars().collect::<Vec<_>>();
    let mut identifiers = HashSet::new();
    let mut index = 0;
    while index < characters.len() {
        if characters[index] == '"' {
            index += 1;
            while index < characters.len() {
                if characters[index] == '\\' {
                    index += 2;
                } else if characters[index] == '"' {
                    index += 1;
                    break;
                } else {
                    index += 1;
                }
            }
            continue;
        }
        if characters[index] == '/' && characters.get(index + 1) == Some(&'/') {
            index += 2;
            while index < characters.len() && characters[index] != '\n' {
                index += 1;
            }
            continue;
        }
        if characters[index] == '/' && characters.get(index + 1) == Some(&'*') {
            index += 2;
            while index + 1 < characters.len()
                && !(characters[index] == '*' && characters[index + 1] == '/')
            {
                index += 1;
            }
            index = (index + 2).min(characters.len());
            continue;
        }
        if characters[index] == '_' || characters[index].is_ascii_alphabetic() {
            let start = index;
            index += 1;
            while index < characters.len()
                && (characters[index] == '_' || characters[index].is_ascii_alphanumeric())
            {
                index += 1;
            }
            identifiers.insert(characters[start..index].iter().collect());
            continue;
        }
        index += 1;
    }
    identifiers
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
