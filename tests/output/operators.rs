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
            Self::Script2(script) => step_green_flag_script_sprite1_2(script, state, clock),
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
                    state.list_mut(0).delete_all();
                    script.frames.last_mut().unwrap().pc = 1;
                    continue;
                },
                1 => {
                    let scratch_list_item: ScratchValue = (1.0_f64).into();
                    state.list_mut(0).add(scratch_list_item);
                    script.frames.last_mut().unwrap().pc = 2;
                    continue;
                },
                2 => {
                    let scratch_new_value: ScratchValue = (scratch_add(1.0_f64, 1.0_f64)).into();
                    state.set(1, scratch_new_value);
                    script.frames.last_mut().unwrap().pc = 3;
                    continue;
                },
                3 => {
                    let scratch_list_item: ScratchValue = (2.0_f64).into();
                    state.list_mut(0).add(scratch_list_item);
                    script.frames.last_mut().unwrap().pc = 4;
                    continue;
                },
                4 => {
                    print_value!("add_1_plus_1", state.get(1));
                    script.frames.last_mut().unwrap().pc = 5;
                    continue;
                },
                5 => {
                    let scratch_list_item: ScratchValue = (3.0_f64).into();
                    state.list_mut(0).add(scratch_list_item);
                    script.frames.last_mut().unwrap().pc = 3;
                    continue;
                },
                _ => return ScriptStatus::Complete,
            },
            1 => match pc {
                0 => {
                    let scratch_new_value: ScratchValue = (0.0_f64).into();
                    state.set(45, scratch_new_value);
                    script.frames.last_mut().unwrap().pc = 1;
                    continue;
                },
                1 => {
                let repeat_count = scratch_round(3.0_f64).max(0.0) as usize;
                script.frames.last_mut().unwrap().repeat_counts[0] = repeat_count;
                    if repeat_count == 0 {
                        script.frames.last_mut().unwrap().pc = 4;
                    } else {
                        script.frames.last_mut().unwrap().pc = 3;
                    }
                    continue;
                },
                2 => {
                    let repeat_remaining = script.frames.last().unwrap().repeat_counts[0].saturating_sub(1);
                    script.frames.last_mut().unwrap().repeat_counts[0] = repeat_remaining;
                    let next_pc = if repeat_remaining > 0 { 3 } else { 4 };
                    script.frames.last_mut().unwrap().pc = next_pc;
                    if script.yield_at_safe_point(clock) { return ScriptStatus::Yield; }
                    continue;
                },
                3 => {
                    let scratch_new_value: ScratchValue = scratch_add(state.get(45), scratch_add(state.get(45), scratch_multiply(&script.frames.last().unwrap().arguments[0], &script.frames.last().unwrap().arguments[0]))).into();
                    state.set(45, scratch_new_value);
                    script.frames.last_mut().unwrap().pc = 2;
                    continue;
                },
                4 => {
                    print_value!("warp_math_result", state.get(45));
                    script.frames.last_mut().unwrap().pc = usize::MAX;
                    continue;
                },
                _ => { script.frames.pop(); continue; },
            },
            2 => match pc {
                0 => {
                    let scratch_new_value: ScratchValue = (script.frames.last().unwrap().arguments[0]).clone();
                    state.set(27, scratch_new_value);
                    script.frames.last_mut().unwrap().pc = 1;
                    continue;
                },
                1 => {
                    if scratch_greater_than(state.get(27), 0.0_f64) {
                        script.frames.last_mut().unwrap().pc = 3;
                        continue;
                    }
                    script.frames.last_mut().unwrap().pc = 4;
                    continue;
                },
                2 => {
                    script.frames.last_mut().unwrap().pc = 1;
                    if script.yield_at_safe_point(clock) { return ScriptStatus::Yield; }
                    continue;
                },
                3 => {
                    let scratch_new_value: ScratchValue = scratch_add(state.get(27), -1.0_f64).into();
                    state.set(27, scratch_new_value);
                    script.frames.last_mut().unwrap().pc = 2;
                    continue;
                },
                4 => {
                    print_value!("normal_math_result", state.get(27));
                    script.frames.last_mut().unwrap().pc = usize::MAX;
                    continue;
                },
                _ => { script.frames.pop(); continue; },
            },
            3 => match pc {
                0 => {
                    print_value!("say", &script.frames.last().unwrap().arguments[0]);
                    script.frames.last_mut().unwrap().pc = usize::MAX;
                    continue;
                },
                _ => { script.frames.pop(); continue; },
            },
            4 => match pc {
                0 => {
                    print_value!("say", &script.frames.last().unwrap().arguments[0]);
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
fn step_green_flag_script_sprite1_2(script: &mut ScriptState, state: &mut ScratchState, clock: &dyn Clock) -> ScriptStatus {
    loop {
        let (function, pc) = match script.frames.last() {
            Some(frame) => (frame.function, frame.pc),
            None => return ScriptStatus::Complete,
        };
        match function {
            0 => match pc {
                0 => {
                    state.list_mut(1).delete_all();
                    script.frames.last_mut().unwrap().pc = 1;
                    continue;
                },
                1 => {
                    let scratch_list_item: ScratchValue = (4.0_f64).into();
                    state.list_mut(0).add(scratch_list_item);
                    script.frames.last_mut().unwrap().pc = 2;
                    continue;
                },
                2 => {
                    let scratch_list_item: ScratchValue = (10.0_f64).into();
                    state.list_mut(1).add(scratch_list_item);
                    script.frames.last_mut().unwrap().pc = 3;
                    continue;
                },
                3 => {
                    let scratch_list_item: ScratchValue = (5.0_f64).into();
                    state.list_mut(0).add(scratch_list_item);
                    script.frames.last_mut().unwrap().pc = 4;
                    continue;
                },
                4 => {
                    let scratch_list_item: ScratchValue = ("apple").into();
                    state.list_mut(1).add(scratch_list_item);
                    script.frames.last_mut().unwrap().pc = 5;
                    continue;
                },
                5 => {
                    let scratch_list_item: ScratchValue = (6.0_f64).into();
                    state.list_mut(0).add(scratch_list_item);
                    script.frames.last_mut().unwrap().pc = 6;
                    continue;
                },
                6 => {
                    let scratch_list_item: ScratchValue = ("start").into();
                    let scratch_list_index: ScratchValue = (1.0_f64).into();
                    state.list_mut(1).insert(scratch_list_item, scratch_list_index);
                    script.frames.last_mut().unwrap().pc = 7;
                    continue;
                },
                7 => {
                    let scratch_list_item: ScratchValue = (7.0_f64).into();
                    state.list_mut(0).add(scratch_list_item);
                    script.frames.last_mut().unwrap().pc = 8;
                    continue;
                },
                8 => {
                    let scratch_list_item: ScratchValue = (20.0_f64).into();
                    let scratch_list_index: ScratchValue = (2.0_f64).into();
                    state.list_mut(1).replace(scratch_list_index, scratch_list_item);
                    script.frames.last_mut().unwrap().pc = 9;
                    continue;
                },
                9 => {
                    let scratch_list_item: ScratchValue = (8.0_f64).into();
                    state.list_mut(0).add(scratch_list_item);
                    script.frames.last_mut().unwrap().pc = 10;
                    continue;
                },
                10 => {
                    let scratch_list_index: ScratchValue = (3.0_f64).into();
                    state.list_mut(1).delete(scratch_list_index);
                    script.frames.last_mut().unwrap().pc = 11;
                    continue;
                },
                11 => {
                    let scratch_list_item: ScratchValue = (9.0_f64).into();
                    state.list_mut(0).add(scratch_list_item);
                    script.frames.last_mut().unwrap().pc = 12;
                    continue;
                },
                12 => {
                    print_value!("say", state.list(1).get(1.0_f64));
                    script.frames.last_mut().unwrap().pc = 13;
                    continue;
                },
                13 => {
                    let scratch_list_item: ScratchValue = (10.0_f64).into();
                    state.list_mut(0).add(scratch_list_item);
                    script.frames.last_mut().unwrap().pc = 14;
                    continue;
                },
                14 => {
                    print_value!("say", state.list(1).length() as f64);
                    script.frames.last_mut().unwrap().pc = 15;
                    continue;
                },
                15 => {
                    let scratch_list_item: ScratchValue = (11.0_f64).into();
                    state.list_mut(0).add(scratch_list_item);
                    script.frames.last_mut().unwrap().pc = 16;
                    continue;
                },
                16 => {
                    print_value!("say", state.list(1).contains(20.0_f64));
                    script.frames.last_mut().unwrap().pc = 17;
                    continue;
                },
                17 => {
                    let scratch_list_item: ScratchValue = (12.0_f64).into();
                    state.list_mut(0).add(scratch_list_item);
                    script.frames.last_mut().unwrap().pc = 18;
                    continue;
                },
                18 => {
                    print_value!("say", state.list(1).item_number("start") as f64);
                    script.frames.last_mut().unwrap().pc = 19;
                    continue;
                },
                19 => {
                    let scratch_list_item: ScratchValue = (13.0_f64).into();
                    state.list_mut(0).add(scratch_list_item);
                    script.frames.last_mut().unwrap().pc = usize::MAX;
                    continue;
                },
                _ => return ScriptStatus::Complete,
            },
            1 => match pc {
                0 => {
                    let scratch_new_value: ScratchValue = (0.0_f64).into();
                    state.set(45, scratch_new_value);
                    script.frames.last_mut().unwrap().pc = 1;
                    continue;
                },
                1 => {
                let repeat_count = scratch_round(3.0_f64).max(0.0) as usize;
                script.frames.last_mut().unwrap().repeat_counts[0] = repeat_count;
                    if repeat_count == 0 {
                        script.frames.last_mut().unwrap().pc = 4;
                    } else {
                        script.frames.last_mut().unwrap().pc = 3;
                    }
                    continue;
                },
                2 => {
                    let repeat_remaining = script.frames.last().unwrap().repeat_counts[0].saturating_sub(1);
                    script.frames.last_mut().unwrap().repeat_counts[0] = repeat_remaining;
                    let next_pc = if repeat_remaining > 0 { 3 } else { 4 };
                    script.frames.last_mut().unwrap().pc = next_pc;
                    if script.yield_at_safe_point(clock) { return ScriptStatus::Yield; }
                    continue;
                },
                3 => {
                    let scratch_new_value: ScratchValue = scratch_add(state.get(45), scratch_add(state.get(45), scratch_multiply(&script.frames.last().unwrap().arguments[0], &script.frames.last().unwrap().arguments[0]))).into();
                    state.set(45, scratch_new_value);
                    script.frames.last_mut().unwrap().pc = 2;
                    continue;
                },
                4 => {
                    print_value!("warp_math_result", state.get(45));
                    script.frames.last_mut().unwrap().pc = usize::MAX;
                    continue;
                },
                _ => { script.frames.pop(); continue; },
            },
            2 => match pc {
                0 => {
                    let scratch_new_value: ScratchValue = (script.frames.last().unwrap().arguments[0]).clone();
                    state.set(27, scratch_new_value);
                    script.frames.last_mut().unwrap().pc = 1;
                    continue;
                },
                1 => {
                    if scratch_greater_than(state.get(27), 0.0_f64) {
                        script.frames.last_mut().unwrap().pc = 3;
                        continue;
                    }
                    script.frames.last_mut().unwrap().pc = 4;
                    continue;
                },
                2 => {
                    script.frames.last_mut().unwrap().pc = 1;
                    if script.yield_at_safe_point(clock) { return ScriptStatus::Yield; }
                    continue;
                },
                3 => {
                    let scratch_new_value: ScratchValue = scratch_add(state.get(27), -1.0_f64).into();
                    state.set(27, scratch_new_value);
                    script.frames.last_mut().unwrap().pc = 2;
                    continue;
                },
                4 => {
                    print_value!("normal_math_result", state.get(27));
                    script.frames.last_mut().unwrap().pc = usize::MAX;
                    continue;
                },
                _ => { script.frames.pop(); continue; },
            },
            3 => match pc {
                0 => {
                    print_value!("say", &script.frames.last().unwrap().arguments[0]);
                    script.frames.last_mut().unwrap().pc = usize::MAX;
                    continue;
                },
                _ => { script.frames.pop(); continue; },
            },
            4 => match pc {
                0 => {
                    print_value!("say", &script.frames.last().unwrap().arguments[0]);
                    script.frames.last_mut().unwrap().pc = usize::MAX;
                    continue;
                },
                _ => { script.frames.pop(); continue; },
            },
            _ => return ScriptStatus::Complete,
        }
    }
}

fn new_green_flag_script_sprite1_2() -> GeneratedScript { GeneratedScript::Script2(ScriptState { frames: vec![ProcedureFrame { function: 0, pc: 0, repeat_counts: vec![0; 0], wait_deadlines: vec![None; 0], arguments: vec![], warp_mode: false }], warp_budget: WarpBudget::new(500) }) }

fn new_scripts() -> Vec<GeneratedScript> { vec![new_green_flag_script_sprite1_1(), new_green_flag_script_sprite1_2()] }

#[allow(dead_code)]
pub struct GeneratedProgram { state: ScratchState, scheduler: ScriptScheduler<GeneratedScript> }

#[allow(dead_code)]
impl GeneratedProgram {
    pub fn new() -> Self { Self::build(None) }
    pub fn new_with_clock(clock: Box<dyn Clock>) -> Self { Self::build(Some(clock)) }
    fn build(clock: Option<Box<dyn Clock>>) -> Self {
    let state = ScratchState::with_project_data(vec![(5.0_f64).into(), (2.0_f64).into(), (8.0_f64).into(), (5.0_f64).into(), (false).into(), (true).into(), (4.0_f64).into(), (2.0_f64).into(), (6.0_f64).into(), (false).into(), ("apple banana").into(), (1.0_f64).into(), (false).into(), (true).into(), (true).into(), (false).into(), ("a").into(), (0.2_f64).into(), (1.0138556845426017_f64).into(), (true).into(), (5.0_f64).into(), (2.0_f64).into(), (30.0_f64).into(), (4.0_f64).into(), (6.0_f64).into(), (0.0_f64).into(), (6.0_f64).into(), (-1.0_f64).into(), (5.0_f64).into(), (true).into(), (false).into(), (false).into(), (true).into(), (7.0_f64).into(), (3.0_f64).into(), (0.5_f64).into(), (0.5_f64).into(), (1.0_f64).into(), (30.000000000000004_f64).into(), (60.00000000000001_f64).into(), (45.0_f64).into(), (2.302585092994046_f64).into(), (1.0_f64).into(), (7.38905609893065_f64).into(), (1000.0_f64).into(), (0.0_f64).into(), (5.0_f64).into()], vec![ScratchVariableMetadata::new("!KOc;W%O4Yj_v]8dqD|]", "absolute_value_of_negative_5", "Stage", true), ScratchVariableMetadata::new("*rR}=_*_~#O%^Wfe-i^p", "add_1_plus_1", "Stage", true), ScratchVariableMetadata::new(",-n(ps9{MDU?yaPB:*AX", "random_1_to_10", "Stage", true), ScratchVariableMetadata::new("-HohZ8~zb83?{$z+F99p", "multiply_1_by_5", "Stage", true), ScratchVariableMetadata::new("0#]u/vH!?$g6/Y:h}5$[", "zero_greater_than_zero", "Stage", true), ScratchVariableMetadata::new("0{jia(J2W6]OBUBsT}vd", "digits_contain_34", "Stage", true), ScratchVariableMetadata::new("2.th4xnyEm#0^=~.MrI2", "mod_4_by_5", "Stage", true), ScratchVariableMetadata::new(";e/Wco}uJz[J9,b;#WlQ", "round_1_5", "Stage", true), ScratchVariableMetadata::new("@.+awex]1ZI=@zI3fg*d", "ceiling_of_5_5", "Stage", true), ScratchVariableMetadata::new("CI):ig}~zm(%$N#y6Tjt", "zero_less_than_zero", "Stage", true), ScratchVariableMetadata::new("ITUrz93KPUt`N,dO.i`8", "join_apple_and_banana", "Stage", true), ScratchVariableMetadata::new("L5c94E6i~3Wngi=xoOM@", "round_1_4", "Stage", true), ScratchVariableMetadata::new("Mim{Gt+O|fQpWuBU=vsr", "banana_less_than_zero", "Stage", true), ScratchVariableMetadata::new("PWn)dZ`[GI[d6C|GyBd3", "banana_greater_than_zero", "Stage", true), ScratchVariableMetadata::new("S./,BZ2K`-8q[q7VPRI1", "five_greater_than_four", "Stage", true), ScratchVariableMetadata::new("Uj]Yh]MozPx]((v7xWz`", "five_less_than_four", "Stage", true), ScratchVariableMetadata::new("XTiLe92muTCfMJrJ_M(~", "first_letter_of_apple", "Stage", true), ScratchVariableMetadata::new("^pDcW=fb8]aKZK)jJ:4n", "divide_1_by_5", "Stage", true), ScratchVariableMetadata::new("e?9M^_O`oxCA8I5+%@):", "random_10_to_negative_10", "Stage", true), ScratchVariableMetadata::new("hcACFnwsjpSXLobe?5qR", "apple_contains_a", "Stage", true), ScratchVariableMetadata::new("loop_demo_index_variable", "square_loop_index", "Stage", true), ScratchVariableMetadata::new("loop_demo_sqrt_variable", "repeated_square_root", "Stage", true), ScratchVariableMetadata::new("loop_demo_sum_variable", "sum_of_squares", "Stage", true), ScratchVariableMetadata::new("loop_demo_until_counter_variable", "until_counter", "Stage", true), ScratchVariableMetadata::new("loop_demo_until_sum_variable", "until_sum", "Stage", true), ScratchVariableMetadata::new("loop_demo_while_counter_variable", "while_counter", "Stage", true), ScratchVariableMetadata::new("loop_demo_while_product_variable", "while_product", "Stage", true), ScratchVariableMetadata::new("normal_math_result_var", "normal_math_result", "Sprite1", false), ScratchVariableMetadata::new("operator_test_var_22", "subtract_9_minus_4", "Stage", true), ScratchVariableMetadata::new("operator_test_var_23", "equal_same_text", "Stage", true), ScratchVariableMetadata::new("operator_test_var_24", "and_false_false", "Stage", true), ScratchVariableMetadata::new("operator_test_var_25", "or_false_true", "Stage", true), ScratchVariableMetadata::new("operator_test_var_26", "not_false", "Stage", true), ScratchVariableMetadata::new("operator_test_var_27", "length_of_scratch", "Stage", true), ScratchVariableMetadata::new("operator_test_var_28", "square_root_of_9", "Stage", true), ScratchVariableMetadata::new("operator_test_var_29", "sine_of_30_degrees", "Stage", true), ScratchVariableMetadata::new("operator_test_var_30", "cosine_of_60_degrees", "Stage", true), ScratchVariableMetadata::new("operator_test_var_31", "tangent_of_45_degrees", "Stage", true), ScratchVariableMetadata::new("operator_test_var_32", "arcsine_of_0_5", "Stage", true), ScratchVariableMetadata::new("operator_test_var_33", "arccosine_of_0_5", "Stage", true), ScratchVariableMetadata::new("operator_test_var_34", "arctangent_of_1", "Stage", true), ScratchVariableMetadata::new("operator_test_var_35", "natural_log_of_10", "Stage", true), ScratchVariableMetadata::new("operator_test_var_36", "log_base_10_of_10", "Stage", true), ScratchVariableMetadata::new("operator_test_var_37", "e_to_the_power_2", "Stage", true), ScratchVariableMetadata::new("operator_test_var_38", "10_to_the_power_3", "Stage", true), ScratchVariableMetadata::new("warp_math_result_var", "warp_math_result", "Sprite1", false), ScratchVariableMetadata::new("z^xm8m%!CqOL_3#eBIfp", "floor_of_5_5", "Stage", true)], vec![ScratchList::new(vec![(1.0_f64).into(), (2.0_f64).into()]), ScratchList::new(vec![("start").into(), (20.0_f64).into()])], vec![ScratchListMetadata::new("]a}Fl@l+VAx!V+d;HXik", "execution order", "Stage", true), ScratchListMetadata::new("operator_list_id", "operator_examples", "Sprite1", false)], vec![false; 2]);
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
