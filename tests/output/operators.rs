project {
    stage "Stage" {
        var: _10_to_the_power_3 = 0
        var: absolute_value_of_negative_5 = 5
        var: add_1_plus_1 = 2
        var: and_false_false = 0
        var: apple_contains_a = true
        var: arccosine_of_0_5 = 0
        var: arcsine_of_0_5 = 0
        var: arctangent_of_1 = 0
        var: banana_greater_than_zero = true
        var: banana_less_than_zero = false
        var: ceiling_of_5_5 = 6
        var: cosine_of_60_degrees = 0
        var: digits_contain_34 = true
        var: divide_1_by_5 = 0.2
        var: e_to_the_power_2 = 0
        var: equal_same_text = 0
        var: first_letter_of_apple = "a"
        var: five_greater_than_four = true
        var: five_less_than_four = false
        var: floor_of_5_5 = 5
        var: join_apple_and_banana = "apple banana"
        var: length_of_scratch = 0
        var: log_base_10_of_10 = 0
        var: mod_4_by_5 = 4
        var: multiply_1_by_5 = 5
        var: natural_log_of_10 = 0
        var: not_false = 0
        var: or_false_true = 0
        var: random_10_to_negative_10 = -0.053779793743032656
        var: random_1_to_10 = 10
        var: round_1_4 = 1
        var: round_1_5 = 2
        var: sine_of_30_degrees = 0
        var: square_root_of_9 = 0
        var: subtract_9_minus_4 = 0
        var: tangent_of_45_degrees = 0
        var: zero_greater_than_zero = false
        var: zero_less_than_zero = false
    }

    sprite "Sprite1" {
        on green_flag {
            set: add_1_plus_1 = 1 + 1
            set: divide_1_by_5 = 1 / 5
            set: multiply_1_by_5 = 1 * 5
            set: mod_4_by_5 = mod(4, 5)
            set: random_1_to_10 = random(1, 10)
            set: random_10_to_negative_10 = random(10, -10.0)
            set: banana_greater_than_zero = "banana" > 0.0
            set: banana_less_than_zero = "banana" < 0.0
            set: zero_greater_than_zero = 0.0 > 0.0
            set: zero_less_than_zero = 0.0 < 0.0
            set: five_greater_than_four = 5 > 4
            set: five_less_than_four = 5 < 4
            set: join_apple_and_banana = join("apple ", "banana")
            set: first_letter_of_apple = letter(1, "apple")
            set: apple_contains_a = contains("apple", "a")
            set: digits_contain_34 = contains(1234, 34)
            set: round_1_4 = round(1.4)
            set: round_1_5 = round(1.5)
            set: absolute_value_of_negative_5 = (-5 as f64).abs()
            set: floor_of_5_5 = (5.5 as f64).floor()
            set: ceiling_of_5_5 = (5.5 as f64).ceil()
            set: subtract_9_minus_4 = 9 - 4
            set: equal_same_text = "same" == "same"
            set: and_false_false = false && false
            set: or_false_true = false || false
            set: not_false = !false
            set: length_of_scratch = length("Scratch")
            set: square_root_of_9 = (9 as f64).sqrt()
            set: sine_of_30_degrees = (30 as f64).to_radians().sin()
            set: cosine_of_60_degrees = (60 as f64).to_radians().cos()
            set: tangent_of_45_degrees = (45 as f64).to_radians().tan()
            set: arcsine_of_0_5 = (0.5 as f64).asin().to_degrees()
            set: arccosine_of_0_5 = (0.5 as f64).acos().to_degrees()
            set: arctangent_of_1 = (1 as f64).atan().to_degrees()
            set: natural_log_of_10 = (10 as f64).ln()
            set: log_base_10_of_10 = (10 as f64).log10()
            set: e_to_the_power_2 = (2 as f64).exp()
            set: _10_to_the_power_3 = 10.0_f64.powf((3 as f64))
        }
    }
}
