#[allow(unused_imports)]
use crate::runtime::{
    scratch_acos_degrees, scratch_add, scratch_and, scratch_asin_degrees, scratch_atan_degrees,
    scratch_bool, scratch_contains, scratch_cos, scratch_divide, scratch_equals,
    scratch_greater_than, scratch_join, scratch_length, scratch_less_than, scratch_letter,
    scratch_mod, scratch_multiply, scratch_not, scratch_or, scratch_random, scratch_round,
    scratch_sin, scratch_subtract, scratch_tan, scratch_to_number, ScratchValue,
};

#[allow(unused_macros)]
macro_rules! print_value {
    ($name:expr, $value:expr) => {
        println!("{}: {:?}", $name, $value);
    };
}

#[allow(dead_code, unused_variables, unused_mut, unused_assignments)]
fn normal_custom_block_math(input: ScratchValue, state_normal_math_result: &mut ScratchValue) {
    let mut normal_math_result = state_normal_math_result.clone();
    normal_math_result = input.clone();
    while scratch_greater_than(&normal_math_result, 0.0_f64) {
        normal_math_result = scratch_add(&normal_math_result, -1.0_f64).into();
    }
    print_value!("normal_math_result", &normal_math_result);
    *state_normal_math_result = normal_math_result;
}

#[allow(dead_code, unused_variables, unused_mut, unused_assignments)]
fn warp_math(input: ScratchValue, state_warp_math_result: &mut ScratchValue) {
    let mut warp_math_result = state_warp_math_result.clone();
    warp_math_result = (0.0_f64).into();
    for _scratch_repeat_index in 0..(scratch_round(3.0_f64).max(0.0) as usize) {
        warp_math_result = scratch_add(&warp_math_result, scratch_add(&warp_math_result, scratch_multiply(&input, &input))).into();
    }
    print_value!("warp_math_result", &warp_math_result);
    *state_warp_math_result = warp_math_result;
}

#[allow(unused_mut, unused_variables, unused_assignments)]
pub fn run() {
    let mut _10_to_the_power_3: ScratchValue = (1000.0_f64).into();
    let mut absolute_value_of_negative_5: ScratchValue = (5.0_f64).into();
    let mut add_1_plus_1: ScratchValue = (2.0_f64).into();
    let mut and_false_false: ScratchValue = (false).into();
    let mut apple_contains_a: ScratchValue = (true).into();
    let mut arccosine_of_0_5: ScratchValue = (60.00000000000001_f64).into();
    let mut arcsine_of_0_5: ScratchValue = (30.000000000000004_f64).into();
    let mut arctangent_of_1: ScratchValue = (45.0_f64).into();
    let mut banana_greater_than_zero: ScratchValue = (true).into();
    let mut banana_less_than_zero: ScratchValue = (false).into();
    let mut ceiling_of_5_5: ScratchValue = (6.0_f64).into();
    let mut cosine_of_60_degrees: ScratchValue = (0.5_f64).into();
    let mut digits_contain_34: ScratchValue = (true).into();
    let mut divide_1_by_5: ScratchValue = (0.2_f64).into();
    let mut e_to_the_power_2: ScratchValue = (7.38905609893065_f64).into();
    let mut equal_same_text: ScratchValue = (true).into();
    let mut first_letter_of_apple: ScratchValue = ("a").into();
    let mut five_greater_than_four: ScratchValue = (true).into();
    let mut five_less_than_four: ScratchValue = (false).into();
    let mut floor_of_5_5: ScratchValue = (5.0_f64).into();
    let mut join_apple_and_banana: ScratchValue = ("apple banana").into();
    let mut length_of_scratch: ScratchValue = (7.0_f64).into();
    let mut log_base_10_of_10: ScratchValue = (1.0_f64).into();
    let mut mod_4_by_5: ScratchValue = (4.0_f64).into();
    let mut multiply_1_by_5: ScratchValue = (5.0_f64).into();
    let mut natural_log_of_10: ScratchValue = (2.302585092994046_f64).into();
    let mut not_false: ScratchValue = (true).into();
    let mut or_false_true: ScratchValue = (false).into();
    let mut random_10_to_negative_10: ScratchValue = (-8.222793281140655_f64).into();
    let mut random_1_to_10: ScratchValue = (8.0_f64).into();
    let mut repeated_square_root: ScratchValue = (2.0_f64).into();
    let mut round_1_4: ScratchValue = (1.0_f64).into();
    let mut round_1_5: ScratchValue = (2.0_f64).into();
    let mut sine_of_30_degrees: ScratchValue = (0.5_f64).into();
    let mut square_loop_index: ScratchValue = (5.0_f64).into();
    let mut square_root_of_9: ScratchValue = (3.0_f64).into();
    let mut subtract_9_minus_4: ScratchValue = (5.0_f64).into();
    let mut sum_of_squares: ScratchValue = (30.0_f64).into();
    let mut tangent_of_45_degrees: ScratchValue = (1.0_f64).into();
    let mut until_counter: ScratchValue = (4.0_f64).into();
    let mut until_sum: ScratchValue = (6.0_f64).into();
    let mut while_counter: ScratchValue = (0.0_f64).into();
    let mut while_product: ScratchValue = (6.0_f64).into();
    let mut zero_greater_than_zero: ScratchValue = (false).into();
    let mut zero_less_than_zero: ScratchValue = (false).into();
    let mut normal_math_result: ScratchValue = (0.0_f64).into();
    let mut warp_math_result: ScratchValue = (175.0_f64).into();
    add_1_plus_1 = (scratch_add(1.0_f64, 1.0_f64)).into();
    print_value!("add_1_plus_1", &add_1_plus_1);
    divide_1_by_5 = (scratch_divide(1.0_f64, 5.0_f64)).into();
    print_value!("divide_1_by_5", &divide_1_by_5);
    multiply_1_by_5 = (scratch_multiply(1.0_f64, 5.0_f64)).into();
    print_value!("multiply_1_by_5", &multiply_1_by_5);
    mod_4_by_5 = (scratch_mod(4.0_f64, 5.0_f64)).into();
    print_value!("mod_4_by_5", &mod_4_by_5);
    random_1_to_10 = (scratch_random("1", "10")).into();
    print_value!("random_1_to_10", &random_1_to_10);
    random_10_to_negative_10 = (scratch_random("10", "-10.0")).into();
    print_value!("random_10_to_negative_10", &random_10_to_negative_10);
    banana_greater_than_zero = (scratch_greater_than("banana", "0.0")).into();
    print_value!("banana_greater_than_zero", &banana_greater_than_zero);
    banana_less_than_zero = (scratch_less_than("banana", "0.0")).into();
    print_value!("banana_less_than_zero", &banana_less_than_zero);
    zero_greater_than_zero = (scratch_greater_than("0.0", "0.0")).into();
    print_value!("zero_greater_than_zero", &zero_greater_than_zero);
    zero_less_than_zero = (scratch_less_than("0.0", "0.0")).into();
    print_value!("zero_less_than_zero", &zero_less_than_zero);
    five_greater_than_four = (scratch_greater_than("5", "4")).into();
    print_value!("five_greater_than_four", &five_greater_than_four);
    five_less_than_four = (scratch_less_than("5", "4")).into();
    print_value!("five_less_than_four", &five_less_than_four);
    join_apple_and_banana = (scratch_join("apple ", "banana")).into();
    print_value!("join_apple_and_banana", &join_apple_and_banana);
    first_letter_of_apple = (scratch_letter(1.0_f64, "apple")).into();
    print_value!("first_letter_of_apple", &first_letter_of_apple);
    apple_contains_a = (scratch_contains("apple", "a")).into();
    print_value!("apple_contains_a", &apple_contains_a);
    digits_contain_34 = (scratch_contains("1234", "34")).into();
    print_value!("digits_contain_34", &digits_contain_34);
    round_1_4 = (scratch_round(1.4_f64)).into();
    print_value!("round_1_4", &round_1_4);
    round_1_5 = (scratch_round(1.5_f64)).into();
    print_value!("round_1_5", &round_1_5);
    absolute_value_of_negative_5 = ((-5.0_f64).abs()).into();
    print_value!("absolute_value_of_negative_5", &absolute_value_of_negative_5);
    floor_of_5_5 = (5.5_f64.floor()).into();
    print_value!("floor_of_5_5", &floor_of_5_5);
    ceiling_of_5_5 = (5.5_f64.ceil()).into();
    print_value!("ceiling_of_5_5", &ceiling_of_5_5);
    subtract_9_minus_4 = (scratch_subtract(9.0_f64, 4.0_f64)).into();
    print_value!("subtract_9_minus_4", &subtract_9_minus_4);
    equal_same_text = (scratch_equals("same", "same")).into();
    print_value!("equal_same_text", &equal_same_text);
    and_false_false = (scratch_and(false, false)).into();
    print_value!("and_false_false", &and_false_false);
    or_false_true = (scratch_or(false, false)).into();
    print_value!("or_false_true", &or_false_true);
    not_false = (scratch_not(false)).into();
    print_value!("not_false", &not_false);
    length_of_scratch = (scratch_length("Scratch")).into();
    print_value!("length_of_scratch", &length_of_scratch);
    square_root_of_9 = (9.0_f64.sqrt()).into();
    print_value!("square_root_of_9", &square_root_of_9);
    sine_of_30_degrees = (scratch_sin(30.0_f64)).into();
    print_value!("sine_of_30_degrees", &sine_of_30_degrees);
    cosine_of_60_degrees = (scratch_cos(60.0_f64)).into();
    print_value!("cosine_of_60_degrees", &cosine_of_60_degrees);
    tangent_of_45_degrees = (scratch_tan(45.0_f64)).into();
    print_value!("tangent_of_45_degrees", &tangent_of_45_degrees);
    arcsine_of_0_5 = (scratch_asin_degrees(0.5_f64)).into();
    print_value!("arcsine_of_0_5", &arcsine_of_0_5);
    arccosine_of_0_5 = (scratch_acos_degrees(0.5_f64)).into();
    print_value!("arccosine_of_0_5", &arccosine_of_0_5);
    arctangent_of_1 = (scratch_atan_degrees(1.0_f64)).into();
    print_value!("arctangent_of_1", &arctangent_of_1);
    natural_log_of_10 = (10.0_f64.ln()).into();
    print_value!("natural_log_of_10", &natural_log_of_10);
    log_base_10_of_10 = (10.0_f64.log10()).into();
    print_value!("log_base_10_of_10", &log_base_10_of_10);
    e_to_the_power_2 = (2.0_f64.exp()).into();
    print_value!("e_to_the_power_2", &e_to_the_power_2);
    _10_to_the_power_3 = (10.0_f64.powf(3.0_f64)).into();
    print_value!("10_to_the_power_3", &_10_to_the_power_3);
    square_loop_index = (1.0_f64).into();
    sum_of_squares = (0.0_f64).into();
    for _scratch_repeat_index in 0..(scratch_round(4.0_f64).max(0.0) as usize) {
        sum_of_squares = (scratch_add(&sum_of_squares, scratch_multiply(&square_loop_index, &square_loop_index))).into();
        square_loop_index = scratch_add(&square_loop_index, 1.0_f64).into();
    }
    print_value!("sum_of_squares", &sum_of_squares);
    repeated_square_root = (256.0_f64).into();
    for _scratch_repeat_index in 0..(scratch_round(3.0_f64).max(0.0) as usize) {
        repeated_square_root = (scratch_to_number(&repeated_square_root).sqrt()).into();
    }
    print_value!("repeated_square_root", &repeated_square_root);
    until_counter = (1.0_f64).into();
    until_sum = (0.0_f64).into();
    while !(scratch_greater_than(&until_counter, 3.0_f64)) {
        until_sum = (scratch_add(&until_sum, &until_counter)).into();
        until_counter = scratch_add(&until_counter, 1.0_f64).into();
    }
    print_value!("until_sum", &until_sum);
    while_counter = (3.0_f64).into();
    while_product = (1.0_f64).into();
    while scratch_greater_than(&while_counter, 0.0_f64) {
        while_product = (scratch_multiply(&while_product, &while_counter)).into();
        while_counter = scratch_add(&while_counter, -1.0_f64).into();
    }
    print_value!("while_product", &while_product);
    warp_math(("5").into(), &mut warp_math_result);
    normal_custom_block_math(("5").into(), &mut normal_math_result);
}
