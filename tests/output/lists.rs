#[allow(unused_imports)]
use crate::runtime::{
    scratch_acos_degrees, scratch_add, scratch_and, scratch_asin_degrees, scratch_atan_degrees,
    scratch_bool, scratch_contains, scratch_cos, scratch_divide, scratch_equals,
    scratch_greater_than, scratch_join, scratch_length, scratch_less_than, scratch_letter,
    scratch_mod, scratch_multiply, scratch_not, scratch_or, scratch_random, scratch_round,
    scratch_sin, scratch_subtract, scratch_tan, scratch_to_number, Clock, ResumableScript,
    ScratchList, ScratchListMetadata, ScratchState, ScratchValue, ScratchVariableMetadata, SchedulerStatus, ScriptScheduler, ScriptStatus, WarpBudget,
};

#[allow(unused_macros)]
macro_rules! print_value {
    ($name:expr, $value:expr) => {
        println!("{}: {:?}", $name, $value);
    };
}

#[allow(dead_code)]
struct ProcedureFrame { function: usize, pc: usize, repeat_counts: Vec<usize>, wait_deadlines: Vec<Option<u64>>, arguments: Vec<ScratchValue>, warp_mode: bool }
struct ScriptState { frames: Vec<ProcedureFrame>, warp_budget: WarpBudget }

impl ScriptState {
    fn warp_budget_expired(&mut self, clock: &dyn Clock) -> bool {
        if !self.frames.last().is_some_and(|frame| frame.warp_mode) { self.warp_budget.reset(); return false; }
        self.warp_budget.expired(clock)
    }
    fn yield_at_safe_point(&mut self, clock: &dyn Clock) -> bool {
        if !self.frames.last().is_some_and(|frame| frame.warp_mode) { self.warp_budget.reset(); return true; }
        self.warp_budget.expired(clock)
    }
}

enum GeneratedScript {
    Script1(usize, ScriptState),
    Script2(usize, ScriptState),
    Script3(usize, ScriptState),
    Script4(usize, ScriptState),
}

impl ResumableScript for GeneratedScript {
    fn step(&mut self, state: &mut ScratchState, clock: &dyn Clock) -> ScriptStatus {
        match self {
            Self::Script1(instance, script) => { state.set_current_instance(*instance); step_green_flag_script_sprite1_1(script, state, clock) },
            Self::Script2(instance, script) => { state.set_current_instance(*instance); step_green_flag_script_sprite1_2(script, state, clock) },
            Self::Script3(instance, script) => { state.set_current_instance(*instance); step_green_flag_script_sprite1_3(script, state, clock) },
            Self::Script4(instance, script) => { state.set_current_instance(*instance); step_green_flag_script_sprite1_4(script, state, clock) },
        }
    }
    fn target_instance(&self) -> usize { match self {
            Self::Script1(instance, _) => *instance,
            Self::Script2(instance, _) => *instance,
            Self::Script3(instance, _) => *instance,
            Self::Script4(instance, _) => *instance,
    } }
}

#[allow(dead_code, unused_variables, unused_mut, unused_assignments, unreachable_code)]
fn step_green_flag_script_sprite1_1(script: &mut ScriptState, state: &mut ScratchState, clock: &dyn Clock) -> ScriptStatus {
    loop {
        let (function, pc) = match script.frames.last() {
            Some(frame) => (frame.function, frame.pc),
            None => return ScriptStatus::Complete,
        };
        match function {
            0 => match pc {
                0 => {
                    state.set_list_visible(1, true);
                    let scratch_list_item: ScratchValue = ("A").into();
                    state.list_mut(1).add(scratch_list_item);
                    script.frames.last_mut().unwrap().pc = 2;
                    continue;
                },
                2 => {
                    let wait_started = script.frames.last().unwrap().wait_deadlines[0].is_none();
                    let wait_deadline = if let Some(deadline) = script.frames.last().unwrap().wait_deadlines[0] { deadline } else { let duration_ms = (scratch_to_number(0.01_f64).max(0.0) * 1000.0).ceil() as u64; let deadline = clock.now_millis().saturating_add(duration_ms); script.frames.last_mut().unwrap().wait_deadlines[0] = Some(deadline); deadline };
                    if wait_started {
                        script.frames.last_mut().unwrap().pc = 2;
                        if script.frames.last().unwrap().warp_mode {
                            if script.warp_budget_expired(clock) { return ScriptStatus::Yield; }
                            continue;
                        }
                        return ScriptStatus::WaitUntil(wait_deadline);
                    }
                    if clock.now_millis() < wait_deadline {
                        if script.frames.last().unwrap().warp_mode {
                            if script.warp_budget_expired(clock) { return ScriptStatus::Yield; }
                            continue;
                        }
                        return ScriptStatus::WaitUntil(wait_deadline);
                    }
                    script.frames.last_mut().unwrap().wait_deadlines[0] = None;
                    script.frames.last_mut().unwrap().pc = 3;
                    continue;
                },
                3 => {
                    let scratch_list_item: ScratchValue = ("B").into();
                    let scratch_list_index: ScratchValue = (1.0_f64).into();
                    state.list_mut(1).insert(scratch_list_item, scratch_list_index);
                    let scratch_list_item: ScratchValue = ("X").into();
                    let scratch_list_index: ScratchValue = (2.0_f64).into();
                    state.list_mut(1).replace(scratch_list_index, scratch_list_item);
                    let scratch_list_index: ScratchValue = (3.0_f64).into();
                    state.list_mut(1).delete(scratch_list_index);
                    let scratch_list_item: ScratchValue = ("sprite-only").into();
                    state.list_mut(0).add(scratch_list_item);
                    state.set_list_visible(0, false);
                    script.frames.last_mut().unwrap().pc = usize::MAX;
                    continue;
                },
                _ => return ScriptStatus::Complete,
            },
            _ => return ScriptStatus::Complete,
        }
    }
}

fn new_green_flag_script_sprite1_1_for(instance: usize) -> GeneratedScript { GeneratedScript::Script1(instance, ScriptState { frames: vec![ProcedureFrame { function: 0, pc: 0, repeat_counts: vec![0; 0], wait_deadlines: vec![None; 1], arguments: vec![], warp_mode: false }], warp_budget: WarpBudget::new(500) }) }

fn new_green_flag_script_sprite1_1() -> GeneratedScript { new_green_flag_script_sprite1_1_for(1) }

#[allow(dead_code, unused_variables, unused_mut, unused_assignments, unreachable_code)]
fn step_green_flag_script_sprite1_2(script: &mut ScriptState, state: &mut ScratchState, clock: &dyn Clock) -> ScriptStatus {
    loop {
        let (function, pc) = match script.frames.last() {
            Some(frame) => (frame.function, frame.pc),
            None => return ScriptStatus::Complete,
        };
        match function {
            0 => match pc {
                0 => {
                    let wait_started = script.frames.last().unwrap().wait_deadlines[0].is_none();
                    let wait_deadline = if let Some(deadline) = script.frames.last().unwrap().wait_deadlines[0] { deadline } else { let duration_ms = (scratch_to_number(0.005_f64).max(0.0) * 1000.0).ceil() as u64; let deadline = clock.now_millis().saturating_add(duration_ms); script.frames.last_mut().unwrap().wait_deadlines[0] = Some(deadline); deadline };
                    if wait_started {
                        script.frames.last_mut().unwrap().pc = 0;
                        if script.frames.last().unwrap().warp_mode {
                            if script.warp_budget_expired(clock) { return ScriptStatus::Yield; }
                            continue;
                        }
                        return ScriptStatus::WaitUntil(wait_deadline);
                    }
                    if clock.now_millis() < wait_deadline {
                        if script.frames.last().unwrap().warp_mode {
                            if script.warp_budget_expired(clock) { return ScriptStatus::Yield; }
                            continue;
                        }
                        return ScriptStatus::WaitUntil(wait_deadline);
                    }
                    script.frames.last_mut().unwrap().wait_deadlines[0] = None;
                    script.frames.last_mut().unwrap().pc = 1;
                    continue;
                },
                1 => {
                    let scratch_new_value: ScratchValue = (state.list(1).contains("A")).into();
                    state.set(4, scratch_new_value);
                    script.frames.last_mut().unwrap().pc = usize::MAX;
                    continue;
                },
                _ => return ScriptStatus::Complete,
            },
            _ => return ScriptStatus::Complete,
        }
    }
}

fn new_green_flag_script_sprite1_2_for(instance: usize) -> GeneratedScript { GeneratedScript::Script2(instance, ScriptState { frames: vec![ProcedureFrame { function: 0, pc: 0, repeat_counts: vec![0; 0], wait_deadlines: vec![None; 1], arguments: vec![], warp_mode: false }], warp_budget: WarpBudget::new(500) }) }

fn new_green_flag_script_sprite1_2() -> GeneratedScript { new_green_flag_script_sprite1_2_for(1) }

#[allow(dead_code, unused_variables, unused_mut, unused_assignments, unreachable_code)]
fn step_green_flag_script_sprite1_3(script: &mut ScriptState, state: &mut ScratchState, clock: &dyn Clock) -> ScriptStatus {
    loop {
        let (function, pc) = match script.frames.last() {
            Some(frame) => (frame.function, frame.pc),
            None => return ScriptStatus::Complete,
        };
        match function {
            0 => match pc {
                0 => {
                    let scratch_new_value: ScratchValue = (state.list(1).contents()).into();
                    state.set(0, scratch_new_value);
                    script.frames.last_mut().unwrap().pc = 1;
                    continue;
                },
                1 => {
                    let wait_started = script.frames.last().unwrap().wait_deadlines[0].is_none();
                    let wait_deadline = if let Some(deadline) = script.frames.last().unwrap().wait_deadlines[0] { deadline } else { let duration_ms = (scratch_to_number(0.02_f64).max(0.0) * 1000.0).ceil() as u64; let deadline = clock.now_millis().saturating_add(duration_ms); script.frames.last_mut().unwrap().wait_deadlines[0] = Some(deadline); deadline };
                    if wait_started {
                        script.frames.last_mut().unwrap().pc = 1;
                        if script.frames.last().unwrap().warp_mode {
                            if script.warp_budget_expired(clock) { return ScriptStatus::Yield; }
                            continue;
                        }
                        return ScriptStatus::WaitUntil(wait_deadline);
                    }
                    if clock.now_millis() < wait_deadline {
                        if script.frames.last().unwrap().warp_mode {
                            if script.warp_budget_expired(clock) { return ScriptStatus::Yield; }
                            continue;
                        }
                        return ScriptStatus::WaitUntil(wait_deadline);
                    }
                    script.frames.last_mut().unwrap().wait_deadlines[0] = None;
                    script.frames.last_mut().unwrap().pc = 2;
                    continue;
                },
                2 => {
                    let scratch_new_value: ScratchValue = (state.list(1).contents()).into();
                    state.set(0, scratch_new_value);
                    script.frames.last_mut().unwrap().pc = usize::MAX;
                    continue;
                },
                _ => return ScriptStatus::Complete,
            },
            _ => return ScriptStatus::Complete,
        }
    }
}

fn new_green_flag_script_sprite1_3_for(instance: usize) -> GeneratedScript { GeneratedScript::Script3(instance, ScriptState { frames: vec![ProcedureFrame { function: 0, pc: 0, repeat_counts: vec![0; 0], wait_deadlines: vec![None; 1], arguments: vec![], warp_mode: false }], warp_budget: WarpBudget::new(500) }) }

fn new_green_flag_script_sprite1_3() -> GeneratedScript { new_green_flag_script_sprite1_3_for(1) }

#[allow(dead_code, unused_variables, unused_mut, unused_assignments, unreachable_code)]
fn step_green_flag_script_sprite1_4(script: &mut ScriptState, state: &mut ScratchState, clock: &dyn Clock) -> ScriptStatus {
    loop {
        let (function, pc) = match script.frames.last() {
            Some(frame) => (frame.function, frame.pc),
            None => return ScriptStatus::Complete,
        };
        match function {
            0 => match pc {
                0 => {
                    let scratch_new_value: ScratchValue = (state.list(1).length() as f64).into();
                    state.set(2, scratch_new_value);
                    let scratch_new_value: ScratchValue = (state.list(1).get(1.0_f64)).clone();
                    state.set(1, scratch_new_value);
                    let scratch_new_value: ScratchValue = (state.list(1).item_number("seed") as f64).into();
                    state.set(3, scratch_new_value);
                    script.frames.last_mut().unwrap().pc = usize::MAX;
                    continue;
                },
                _ => return ScriptStatus::Complete,
            },
            _ => return ScriptStatus::Complete,
        }
    }
}

fn new_green_flag_script_sprite1_4_for(instance: usize) -> GeneratedScript { GeneratedScript::Script4(instance, ScriptState { frames: vec![ProcedureFrame { function: 0, pc: 0, repeat_counts: vec![0; 0], wait_deadlines: vec![None; 0], arguments: vec![], warp_mode: false }], warp_budget: WarpBudget::new(500) }) }

fn new_green_flag_script_sprite1_4() -> GeneratedScript { new_green_flag_script_sprite1_4_for(1) }

fn new_scripts() -> Vec<GeneratedScript> { vec![new_green_flag_script_sprite1_1(), new_green_flag_script_sprite1_2(), new_green_flag_script_sprite1_3(), new_green_flag_script_sprite1_4()] }

fn new_clone_scripts(target: usize, instance: usize) -> Vec<GeneratedScript> { match target { _ => Vec::new() } }

#[allow(dead_code)]
pub struct GeneratedProgram { state: ScratchState, scheduler: ScriptScheduler<GeneratedScript> }

#[allow(dead_code)]
impl GeneratedProgram {
    pub fn new() -> Self { Self::build(None) }
    pub fn new_with_clock(clock: Box<dyn Clock>) -> Self { Self::build(Some(clock)) }
    fn build(clock: Option<Box<dyn Clock>>) -> Self {
    let mut state = ScratchState::with_project_data(vec![("").into(), ("").into(), (0.0_f64).into(), (0.0_f64).into(), (false).into()], vec![ScratchVariableMetadata::for_target("contents-id", "contents", "Stage", 0, true), ScratchVariableMetadata::for_target("item-id", "first item", "Stage", 0, true), ScratchVariableMetadata::for_target("length-id", "length", "Stage", 0, true), ScratchVariableMetadata::for_target("number-id", "item number", "Stage", 0, true), ScratchVariableMetadata::for_target("observed-id", "saw A", "Stage", 0, true)], vec![ScratchList::new(vec![(1.0_f64).into(), ("1").into(), (true).into()]), ScratchList::new(vec![("seed").into()])], vec![ScratchListMetadata::for_target("sprite-list", "items", "Sprite1", 1, false), ScratchListMetadata::for_target("stage-list", "items", "Stage", 0, true)], vec![false; 2]);
    state.register_target(0, "Stage", true);
    state.register_target(1, "Sprite1", false);
let scripts = new_scripts();
let scheduler = match clock { Some(clock) => ScriptScheduler::with_clock(scripts, clock), None => ScriptScheduler::new(scripts) };
    Self { state, scheduler }
    }

    pub fn step(&mut self) -> bool { self.advance() == SchedulerStatus::Progressed }
    pub fn advance(&mut self) -> SchedulerStatus { let status = self.scheduler.advance(&mut self.state); let deleted = self.state.current_instance(); if !self.state.instance_is_alive(deleted) { self.scheduler.remove_target_scripts(deleted); } for instance in self.state.take_pending_clones() { if let Some(target) = self.state.instance_target(instance) { self.scheduler.add_scripts(new_clone_scripts(target, instance)); } } status }
    pub fn duration_until(&self, deadline: u64) -> std::time::Duration { self.scheduler.duration_until(deadline) }
    pub fn restart(&mut self) { self.state.remove_clones(); self.scheduler.restart(new_scripts()); }
    pub fn is_complete(&self) -> bool { self.scheduler.active_scripts() == 0 }
    pub fn state(&self) -> &ScratchState { &self.state }
    pub fn run_until_complete(&mut self) { loop { match self.advance() { SchedulerStatus::Progressed => {}, SchedulerStatus::WaitingUntil(deadline) => std::thread::sleep(self.duration_until(deadline)), SchedulerStatus::Complete => break } } }
}

pub fn run() { let mut program = GeneratedProgram::new(); program.run_until_complete(); }
