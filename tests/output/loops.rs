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
    Script1(ScriptState),
    Script2(ScriptState),
}

impl ResumableScript for GeneratedScript {
    fn step(&mut self, state: &mut ScratchState, clock: &dyn Clock) -> ScriptStatus {
        match self {
            Self::Script1(script) => step_green_flag_script_stage_1(script, state, clock),
            Self::Script2(script) => step_green_flag_script_stage_2(script, state, clock),
        }
    }
}

#[allow(dead_code, unused_variables, unused_mut, unused_assignments, unreachable_code)]
fn step_green_flag_script_stage_1(script: &mut ScriptState, state: &mut ScratchState, clock: &dyn Clock) -> ScriptStatus {
    loop {
        let (function, pc) = match script.frames.last() {
            Some(frame) => (frame.function, frame.pc),
            None => return ScriptStatus::Complete,
        };
        match function {
            0 => match pc {
                0 => {
                    let mut scratch_local_2 = scratch_to_number(state.get(2));
                    let scratch_warp_mode = script.frames.last().is_some_and(|frame| frame.warp_mode);
                    if !scratch_warp_mode { script.warp_budget.reset(); }
                    let mut repeat_remaining = script.frames.last().unwrap().repeat_counts[0];
                    if repeat_remaining == 0 {
                        repeat_remaining = scratch_round(0.0_f64).max(0.0) as usize;
                        script.frames.last_mut().unwrap().repeat_counts[0] = repeat_remaining;
                    }
                    if repeat_remaining == 0 {
                        script.frames.last_mut().unwrap().pc = 3;
                        continue;
                    }
                    loop {
                        scratch_local_2 = scratch_add(scratch_local_2, 100.0_f64);
                        repeat_remaining -= 1;
                        script.frames.last_mut().unwrap().repeat_counts[0] = repeat_remaining;
                        if !scratch_warp_mode || script.warp_budget.expired(clock) {
                            script.frames.last_mut().unwrap().pc = if repeat_remaining > 0 { 0 } else { 3 };
                            state.set(2, scratch_local_2.into());
                            return ScriptStatus::Yield;
                        }
                        if repeat_remaining == 0 { break; }
                    }
                    state.set(2, scratch_local_2.into());
                    script.frames.last_mut().unwrap().pc = 3;
                    continue;
                },
                3 => {
                let repeat_count = scratch_round(state.get(0)).max(0.0) as usize;
                script.frames.last_mut().unwrap().repeat_counts[1] = repeat_count;
                    if repeat_count == 0 {
                        script.frames.last_mut().unwrap().pc = 8;
                    } else {
                        script.frames.last_mut().unwrap().pc = 5;
                    }
                    continue;
                },
                4 => {
                    let repeat_remaining = script.frames.last().unwrap().repeat_counts[1].saturating_sub(1);
                    script.frames.last_mut().unwrap().repeat_counts[1] = repeat_remaining;
                    let next_pc = if repeat_remaining > 0 { 5 } else { 8 };
                    script.frames.last_mut().unwrap().pc = next_pc;
                    if script.yield_at_safe_point(clock) { return ScriptStatus::Yield; }
                    continue;
                },
                5 => {
                    let mut scratch_local_2 = scratch_to_number(state.get(2));
                    let scratch_warp_mode = script.frames.last().is_some_and(|frame| frame.warp_mode);
                    if !scratch_warp_mode { script.warp_budget.reset(); }
                    let mut repeat_remaining = script.frames.last().unwrap().repeat_counts[2];
                    if repeat_remaining == 0 {
                        repeat_remaining = scratch_round(2.0_f64).max(0.0) as usize;
                        script.frames.last_mut().unwrap().repeat_counts[2] = repeat_remaining;
                    }
                    if repeat_remaining == 0 {
                        script.frames.last_mut().unwrap().pc = 4;
                        continue;
                    }
                    loop {
                        scratch_local_2 = scratch_add(scratch_local_2, 1.0_f64);
                        repeat_remaining -= 1;
                        script.frames.last_mut().unwrap().repeat_counts[2] = repeat_remaining;
                        if !scratch_warp_mode || script.warp_budget.expired(clock) {
                            script.frames.last_mut().unwrap().pc = if repeat_remaining > 0 { 5 } else { 4 };
                            state.set(2, scratch_local_2.into());
                            return ScriptStatus::Yield;
                        }
                        if repeat_remaining == 0 { break; }
                    }
                    state.set(2, scratch_local_2.into());
                    script.frames.last_mut().unwrap().pc = 4;
                    continue;
                },
                8 => {
                    if !(scratch_bool(state.get(1))) {
                        script.frames.last_mut().unwrap().pc = 10;
                        continue;
                    }
                    script.frames.last_mut().unwrap().pc = 11;
                    continue;
                },
                9 => {
                    script.frames.last_mut().unwrap().pc = 8;
                    if script.yield_at_safe_point(clock) { return ScriptStatus::Yield; }
                    continue;
                },
                10 => {
                    let scratch_new_value: ScratchValue = scratch_add(state.get(2), 1.0_f64).into();
                    state.set(2, scratch_new_value);
                    script.frames.last_mut().unwrap().pc = 9;
                    continue;
                },
                11 => {
                    if scratch_not(scratch_bool(state.get(1))) {
                        script.frames.last_mut().unwrap().pc = 13;
                        continue;
                    }
                    script.frames.last_mut().unwrap().pc = usize::MAX;
                    continue;
                },
                12 => {
                    script.frames.last_mut().unwrap().pc = 11;
                    if script.yield_at_safe_point(clock) { return ScriptStatus::Yield; }
                    continue;
                },
                13 => {
                    let scratch_new_value: ScratchValue = scratch_add(state.get(2), 1.0_f64).into();
                    state.set(2, scratch_new_value);
                    script.frames.last_mut().unwrap().pc = 12;
                    continue;
                },
                _ => return ScriptStatus::Complete,
            },
            _ => return ScriptStatus::Complete,
        }
    }
}

fn new_green_flag_script_stage_1() -> GeneratedScript { GeneratedScript::Script1(ScriptState { frames: vec![ProcedureFrame { function: 0, pc: 0, repeat_counts: vec![0; 3], wait_deadlines: vec![None; 0], arguments: vec![], warp_mode: false }], warp_budget: WarpBudget::new(500) }) }

#[allow(dead_code, unused_variables, unused_mut, unused_assignments, unreachable_code)]
fn step_green_flag_script_stage_2(script: &mut ScriptState, state: &mut ScratchState, clock: &dyn Clock) -> ScriptStatus {
    loop {
        let (function, pc) = match script.frames.last() {
            Some(frame) => (frame.function, frame.pc),
            None => return ScriptStatus::Complete,
        };
        match function {
            0 => match pc {
                0 => {
                    let scratch_new_value: ScratchValue = (true).into();
                    state.set(1, scratch_new_value);
                    let scratch_new_value: ScratchValue = (state.get(1)).clone();
                    state.set(0, scratch_new_value);
                    script.frames.last_mut().unwrap().pc = 2;
                    continue;
                },
                2 => {
                    let mut scratch_local_2 = scratch_to_number(state.get(2));
                    let scratch_warp_mode = script.frames.last().is_some_and(|frame| frame.warp_mode);
                    if !scratch_warp_mode { script.warp_budget.reset(); }
                    loop {
                        scratch_local_2 = scratch_add(scratch_local_2, 1.0_f64);
                        if !scratch_warp_mode || script.warp_budget.expired(clock) {
                            script.frames.last_mut().unwrap().pc = 2;
                            state.set(2, scratch_local_2.into());
                            return ScriptStatus::Yield;
                        }
                    }
                    state.set(2, scratch_local_2.into());
                    script.frames.last_mut().unwrap().pc = usize::MAX;
                    continue;
                },
                _ => return ScriptStatus::Complete,
            },
            _ => return ScriptStatus::Complete,
        }
    }
}

fn new_green_flag_script_stage_2() -> GeneratedScript { GeneratedScript::Script2(ScriptState { frames: vec![ProcedureFrame { function: 0, pc: 0, repeat_counts: vec![0; 0], wait_deadlines: vec![None; 0], arguments: vec![], warp_mode: false }], warp_budget: WarpBudget::new(500) }) }

fn new_scripts() -> Vec<GeneratedScript> { vec![new_green_flag_script_stage_1(), new_green_flag_script_stage_2()] }

#[allow(dead_code)]
pub struct GeneratedProgram { state: ScratchState, scheduler: ScriptScheduler<GeneratedScript> }

#[allow(dead_code)]
impl GeneratedProgram {
    pub fn new() -> Self { Self::build(None) }
    pub fn new_with_clock(clock: Box<dyn Clock>) -> Self { Self::build(Some(clock)) }
    fn build(clock: Option<Box<dyn Clock>>) -> Self {
    let state = ScratchState::with_project_data(vec![("3").into(), (false).into(), (0.0_f64).into()], vec![ScratchVariableMetadata::new("count_id", "count", "Stage", true), ScratchVariableMetadata::new("done_id", "done", "Stage", true), ScratchVariableMetadata::new("progress_id", "progress", "Stage", true)], vec![], vec![], vec![false; 0]);
let scripts = new_scripts();
let scheduler = match clock { Some(clock) => ScriptScheduler::with_clock(scripts, clock), None => ScriptScheduler::new(scripts) };
    Self { state, scheduler }
    }

    pub fn step(&mut self) -> bool { self.scheduler.step(&mut self.state) }
    pub fn advance(&mut self) -> SchedulerStatus { self.scheduler.advance(&mut self.state) }
    pub fn duration_until(&self, deadline: u64) -> std::time::Duration { self.scheduler.duration_until(deadline) }
    pub fn restart(&mut self) { self.scheduler.restart(new_scripts()); }
    pub fn is_complete(&self) -> bool { self.scheduler.active_scripts() == 0 }
    pub fn state(&self) -> &ScratchState { &self.state }
    pub fn run_until_complete(&mut self) { self.scheduler.run_until_complete(&mut self.state); }
}

pub fn run() { let mut program = GeneratedProgram::new(); program.run_until_complete(); }
