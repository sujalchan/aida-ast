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
            Self::Script1(script) => step_green_flag_script_sprite1_1(script, state, clock),
            Self::Script2(script) => step_green_flag_script_sprite2_2(script, state, clock),
        }
    }
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
                    let scratch_new_value: ScratchValue = (0.0_f64).into();
                    state.set(1, scratch_new_value);
                    let scratch_new_value: ScratchValue = (1.0_f64).into();
                    state.set(2, scratch_new_value);
                    let scratch_new_value: ScratchValue = (0.0_f64).into();
                    state.set(4, scratch_new_value);
                    let scratch_new_value: ScratchValue = (0.0_f64).into();
                    state.set(0, scratch_new_value);
                    let scratch_new_value: ScratchValue = (1.0_f64).into();
                    state.set(3, scratch_new_value);
                    script.frames.last_mut().unwrap().pc = 5;
                    continue;
                },
                5 => {
                    let inherited_warp = script.frames.last().is_some_and(|frame| frame.warp_mode);
                    script.frames.last_mut().unwrap().pc = usize::MAX;
                    script.frames.push(ProcedureFrame { function: 1, pc: 0, repeat_counts: vec![0; 0], wait_deadlines: vec![None; 0], arguments: vec![], warp_mode: inherited_warp || true });
                    let recursive_call = script.frames.iter().filter(|frame| frame.function == script.frames.last().unwrap().function).count() > 1;
                    if recursive_call && !script.frames.last().unwrap().warp_mode { return ScriptStatus::Yield; }
                    if script.frames.last().unwrap().warp_mode && script.warp_budget_expired(clock) { return ScriptStatus::Yield; }
                    continue;
                },
                _ => return ScriptStatus::Complete,
            },
            1 => match pc {
                0 => {
                    let mut scratch_local_0 = scratch_to_number(state.get(0));
                    let mut scratch_local_1 = scratch_to_number(state.get(1));
                    let mut scratch_local_2 = scratch_to_number(state.get(2));
                    let mut scratch_local_3 = scratch_to_number(state.get(3));
                    let mut scratch_local_4 = scratch_to_number(state.get(4));
                    let scratch_warp_mode = script.frames.last().is_some_and(|frame| frame.warp_mode);
                    if !scratch_warp_mode { script.warp_budget.reset(); }
                    loop {
                        if scratch_local_3 == 0.0_f64 { break; }
                        scratch_local_4 = scratch_add(scratch_local_4, scratch_divide(1.0_f64, scratch_multiply(scratch_local_2, scratch_local_2)));
                        scratch_local_0 = (scratch_to_number(scratch_multiply(6.0_f64, scratch_local_4))).sqrt();
                        scratch_local_1 += 1.0_f64;
                        scratch_local_2 += 1.0_f64;
                        if !scratch_warp_mode || script.warp_budget.expired(clock) {
                            script.frames.last_mut().unwrap().pc = 0;
                            state.set(0, scratch_local_0.into());
                            state.set(1, scratch_local_1.into());
                            state.set(2, scratch_local_2.into());
                            state.set(4, scratch_local_4.into());
                            return ScriptStatus::Yield;
                        }
                    }
                    state.set(0, scratch_local_0.into());
                    state.set(1, scratch_local_1.into());
                    state.set(2, scratch_local_2.into());
                    state.set(4, scratch_local_4.into());
                    script.frames.last_mut().unwrap().pc = usize::MAX;
                    continue;
                },
                _ => { script.frames.pop(); continue; },
            },
            _ => return ScriptStatus::Complete,
        }
    }
}

fn new_green_flag_script_sprite1_1() -> GeneratedScript { GeneratedScript::Script1(ScriptState { frames: vec![ProcedureFrame { function: 0, pc: 0, repeat_counts: vec![0; 0], wait_deadlines: vec![None; 0], arguments: vec![], warp_mode: false }], warp_budget: WarpBudget::new(500) }) }

#[allow(dead_code, unused_variables, unused_mut, unused_assignments, unreachable_code)]
fn step_green_flag_script_sprite2_2(script: &mut ScriptState, state: &mut ScratchState, clock: &dyn Clock) -> ScriptStatus {
    loop {
        let (function, pc) = match script.frames.last() {
            Some(frame) => (frame.function, frame.pc),
            None => return ScriptStatus::Complete,
        };
        match function {
            0 => match pc {
                0 => {
                    let wait_started = script.frames.last().unwrap().wait_deadlines[0].is_none();
                    let wait_deadline = if let Some(deadline) = script.frames.last().unwrap().wait_deadlines[0] { deadline } else { let duration_ms = (scratch_to_number(10.0_f64).max(0.0) * 1000.0).ceil() as u64; let deadline = clock.now_millis().saturating_add(duration_ms); script.frames.last_mut().unwrap().wait_deadlines[0] = Some(deadline); deadline };
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
                    let scratch_new_value: ScratchValue = (0.0_f64).into();
                    state.set(3, scratch_new_value);
                    let scratch_list_item: ScratchValue = (state.get(1)).clone();
                    state.list_mut(0).add(scratch_list_item);
                    let scratch_list_item: ScratchValue = (state.get(0)).clone();
                    state.list_mut(0).add(scratch_list_item);
                    print_value!("say", state.list(0).contents());
                    script.frames.last_mut().unwrap().pc = usize::MAX;
                    continue;
                },
                _ => return ScriptStatus::Complete,
            },
            _ => return ScriptStatus::Complete,
        }
    }
}

fn new_green_flag_script_sprite2_2() -> GeneratedScript { GeneratedScript::Script2(ScriptState { frames: vec![ProcedureFrame { function: 0, pc: 0, repeat_counts: vec![0; 0], wait_deadlines: vec![None; 1], arguments: vec![], warp_mode: false }], warp_budget: WarpBudget::new(500) }) }

fn new_scripts() -> Vec<GeneratedScript> { vec![new_green_flag_script_sprite1_1(), new_green_flag_script_sprite2_2()] }

#[allow(dead_code)]
pub struct GeneratedProgram { state: ScratchState, scheduler: ScriptScheduler<GeneratedScript> }

#[allow(dead_code)]
impl GeneratedProgram {
    pub fn new() -> Self { Self::build(None) }
    pub fn new_with_clock(clock: Box<dyn Clock>) -> Self { Self::build(Some(clock)) }
    fn build(clock: Option<Box<dyn Clock>>) -> Self {
    let state = ScratchState::with_project_data(vec![(3.14159264498239_f64).into(), (2201753300.0_f64).into(), (2201753301.0_f64).into(), (0.0_f64).into(), (1.644934057834575_f64).into()], vec![ScratchVariableMetadata::new("pi_approximation", "pi approximation", "Stage", true), ScratchVariableMetadata::new("pi_iteration_count", "pi iteration count", "Stage", true), ScratchVariableMetadata::new("pi_series_index", "pi series index", "Stage", true), ScratchVariableMetadata::new("pi_series_running", "pi series running", "Stage", true), ScratchVariableMetadata::new("pi_series_sum", "pi series sum", "Stage", true)], vec![ScratchList::new(vec![])], vec![ScratchListMetadata::new("-6MD/8R-Zs-[{)xe]`NR", "pi test output", "Stage", true)], vec![false; 1]);
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
