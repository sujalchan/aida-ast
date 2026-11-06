use super::expression;
use crate::parser::ScratchTarget;
use serde_json::json;
use std::collections::{HashMap, HashSet};

#[test]
fn comparison_only_normalizes_canonical_integer_text() {
    let mut target: ScratchTarget = serde_json::from_value(json!({
            "name": "Stage", "isStage": true, "blocks": {
                "sensor": {"opcode": "sensing_loudness"},
                "compare": {"opcode": "operator_gt", "inputs": {"OPERAND1": [2, "sensor"], "OPERAND2": [1, [10, "240"]]}}
            }
        })).unwrap();
    let root = json!([2, "compare"]);
    let generate = |target: &ScratchTarget| {
        expression(
            target,
            Some(&root),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &mut HashSet::new(),
        )
    };
    let generated = generate(&target);
    assert!(generated.contains("let left = state.loudness()"));
    assert!(generated.contains("let right = 240.0_f64"));
    assert!(generated.contains("scratch_greater_than(left, right)"));
    for value in ["0240", "-0", " 240", "240.0", "1000001"] {
        target
            .blocks
            .get_mut("compare")
            .unwrap()
            .inputs
            .insert("OPERAND2".into(), json!([1, [10, value]]));
        assert!(generate(&target).contains(&format!("{value:?}")));
    }
}

#[test]
fn sensing_reporter_inside_nested_operator_uses_the_shared_input_evaluator() {
    let target: ScratchTarget = serde_json::from_value(json!({
        "name": "Stage",
        "isStage": true,
        "blocks": {
            "loudness": { "opcode": "sensing_loudness" },
            "join": {
                "opcode": "operator_join",
                "inputs": {
                    "STRING1": [3, "loudness", [10, "fallback"]],
                    "STRING2": [1, [10, " dB"]]
                }
            }
        }
    }))
    .unwrap();
    let root = json!([3, "join"]);
    let generated = expression(
        &target,
        Some(&root),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &mut HashSet::new(),
    );

    assert!(generated.contains("state.loudness()"));
    assert!(generated.starts_with("scratch_join("));
    assert!(generated.contains("dB"));
    assert!(!generated.contains("fallback"));
}

#[test]
fn project_sensing_reporters_lower_inside_join_and_comparison_inputs() {
    let mut target: ScratchTarget = serde_json::from_value(json!({
        "name": "Stage",
        "isStage": true,
        "blocks": {
            "current": {
                "opcode": "sensing_current",
                "fields": { "CURRENTMENU": ["DAYOFWEEK", null] }
            },
            "days": { "opcode": "sensing_dayssince2000" },
            "online": { "opcode": "sensing_online" },
            "username": { "opcode": "sensing_username" },
            "mousex": { "opcode": "sensing_mousex" },
            "mousey": { "opcode": "sensing_mousey" },
            "mousedown": { "opcode": "sensing_mousedown" },
            "timer": { "opcode": "sensing_timer" },
            "loudness": { "opcode": "sensing_loudness" },
            "consumer": { "opcode": "operator_join" }
        }
    }))
    .unwrap();
    let cases = [
        ("current", "state.current_day_of_week()"),
        ("days", "state.days_since_2000()"),
        ("online", "false"),
        ("username", "state.username()"),
        ("mousex", "state.mouse_x()"),
        ("mousey", "state.mouse_y()"),
        ("mousedown", "state.mouse_down()"),
        ("timer", "state.project_timer_seconds(clock.now_millis())"),
        ("loudness", "state.loudness()"),
    ];

    for (sensor, expected) in cases {
        let consumer = target.blocks.get_mut("consumer").unwrap();
        consumer.opcode = "operator_join".to_owned();
        consumer.inputs = HashMap::from([
            ("STRING1".to_owned(), json!([3, sensor])),
            ("STRING2".to_owned(), json!([1, [10, ""]])),
        ]);
        let join = expression(
            &target,
            Some(&json!([3, "consumer"])),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &mut HashSet::new(),
        );
        assert!(join.starts_with("scratch_join("));
        assert!(join.contains(expected), "{sensor} missing in {join}");

        let consumer = target.blocks.get_mut("consumer").unwrap();
        consumer.opcode = "operator_equals".to_owned();
        consumer.inputs = HashMap::from([
            ("OPERAND1".to_owned(), json!([3, sensor])),
            ("OPERAND2".to_owned(), json!([1, [10, ""]])),
        ]);
        let comparison = expression(
            &target,
            Some(&json!([3, "consumer"])),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &mut HashSet::new(),
        );
        assert!(comparison.starts_with("scratch_equals("));
        assert!(
            comparison.contains(expected),
            "{sensor} missing in {comparison}"
        );
    }
}

// preserve every selector choice and scratch's default before a list consumer interprets it.
#[test]
fn list_index_dropdowns_keep_scratch_values_and_default_to_one() {
    let cases = [
        ("data_listindexall", "1", "1"),
        ("data_listindexall", "last", "last"),
        ("data_listindexall", "all", "all"),
        ("data_listindexrandom", "1", "1"),
        ("data_listindexrandom", "last", "last"),
        ("data_listindexrandom", "random", "random"),
    ];

    for (opcode, selected, expected) in cases {
        let target: ScratchTarget = serde_json::from_value(json!({
            "name": "Stage",
            "isStage": true,
            "blocks": {
                "selector": {
                    "opcode": opcode,
                    "fields": { "INDEX": [selected, null] }
                }
            }
        }))
        .unwrap();
        let generated = expression(
            &target,
            Some(&json!([3, "selector"])),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &mut HashSet::new(),
        );

        assert_eq!(generated, format!("{expected:?}"), "{opcode}: {selected}");
    }

    for opcode in ["data_listindexall", "data_listindexrandom"] {
        let target: ScratchTarget = serde_json::from_value(json!({
            "name": "Stage",
            "isStage": true,
            "blocks": { "selector": { "opcode": opcode } }
        }))
        .unwrap();
        let generated = expression(
            &target,
            Some(&json!([3, "selector"])),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &mut HashSet::new(),
        );

        assert_eq!(generated, "\"1\"");
    }
}
