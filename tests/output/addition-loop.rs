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
                    let scratch_new_value: ScratchValue = ("0").into();
                    state.set(0, scratch_new_value);
                    script.frames.last_mut().unwrap().pc = 1;
                    continue;
                },
                1 => {
                    script.frames.last_mut().unwrap().pc = 3;
                    continue;
                },
                2 => {
                    script.frames.last_mut().unwrap().pc = 1;
                    if script.yield_at_safe_point(clock) { return ScriptStatus::Yield; }
                    continue;
                },
                3 => {
                    let scratch_new_value: ScratchValue = scratch_add(state.get(0), 1.0_f64).into();
                    state.set(0, scratch_new_value);
                    script.frames.last_mut().unwrap().pc = 2;
                    continue;
                },
                _ => return ScriptStatus::Complete,
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
                    let wait_deadline = if let Some(deadline) = script.frames.last().unwrap().wait_deadlines[0] { deadline } else { let duration_ms = (scratch_to_number(1.0_f64).max(0.0) * 1000.0).ceil() as u64; let deadline = clock.now_millis().saturating_add(duration_ms); script.frames.last_mut().unwrap().wait_deadlines[0] = Some(deadline); deadline };
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
    let state = ScratchState::with_project_data(vec![(0.0_f64).into()], vec![ScratchVariableMetadata::new("`jEk@4|i[#Fk?(8x)AV.-my variable", "my variable", "Stage", true)], vec![ScratchList::new(vec![])], vec![ScratchListMetadata::new("K/Ug=Ei|I(=SFj*93J|G", "output", "Stage", true)], vec![false; 1]);
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
