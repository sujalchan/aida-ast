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

use super::*;
use crate::parser::ScratchVariable;
use serde_json::json;

fn block(opcode: &str, inputs: &[(&str, Value)]) -> ScratchBlock {
    ScratchBlock {
        opcode: opcode.to_owned(),
        next: None,
        parent: None,
        inputs: inputs
            .iter()
            .map(|(name, value)| (name.to_string(), value.clone()))
            .collect(),
        fields: Default::default(),
        shadow: false,
        top_level: false,
        x: None,
        y: None,
        mutation: Value::Null,
    }
}

fn hat(next: &str) -> ScratchBlock {
    let mut hat = block("event_whenflagclicked", &[]);
    hat.top_level = true;
    hat.next = Some(next.to_owned());
    hat
}

fn target(blocks: Vec<(&str, ScratchBlock)>) -> ScratchTarget {
    ScratchTarget {
        name: "Sprite1".to_owned(),
        is_stage: false,
        blocks: blocks
            .into_iter()
            .map(|(id, block)| (id.to_owned(), block))
            .collect(),
        variables: Default::default(),
        lists: Default::default(),
    }
}

fn variable(name: &str, value: Value) -> ScratchVariable {
    ScratchVariable {
        name: name.to_owned(),
        value,
        is_cloud: false,
    }
}

fn number(value: &str) -> Value {
    json!([1, [4, value]])
}
fn text(value: &str) -> Value {
    json!([1, [10, value]])
}
fn reference(id: &str) -> Value {
    json!([3, id, [10, "unused shadow"]])
}

fn render_expression(target: &ScratchTarget, id: &str) -> String {
    TargetGenerator::new(target, None)
        .generate_expression(id, &mut HashSet::new())
        .text
}

fn nested_target() -> ScratchTarget {
    let mut set = block("data_setvariableto", &[("VALUE", reference("add"))]);
    set.fields
        .insert("VARIABLE".to_owned(), json!(["test", "test-id"]));
    target(vec![
        ("hat", hat("set")),
        ("set", set),
        (
            "add",
            block(
                "operator_add",
                &[("NUM1", reference("join")), ("NUM2", number("2"))],
            ),
        ),
        (
            "join",
            block(
                "operator_join",
                &[
                    ("STRING1", reference("compare")),
                    ("STRING2", text("banana")),
                ],
            ),
        ),
        (
            "compare",
            block(
                "operator_gt",
                &[("OPERAND1", text("")), ("OPERAND2", number("50"))],
            ),
        ),
    ])
}

#[test]
fn generates_numeric_expression() {
    let target = target(vec![(
        "add",
        block(
            "operator_add",
            &[("NUM1", number("5")), ("NUM2", number("3"))],
        ),
    )]);
    assert_eq!(render_expression(&target, "add"), "5 + 3");
}

#[test]
fn generates_nested_expression() {
    assert_eq!(
        render_expression(&nested_target(), "add"),
        r#"join("" > 50, "banana") + 2"#
    );
}

#[test]
fn generates_green_flag_assignment_and_sorted_variables() {
    let mut stage = target(vec![]);
    stage.name = "Stage".to_owned();
    stage.is_stage = true;
    stage
        .variables
        .insert("second".to_owned(), variable("test 2", json!(0)));
    stage
        .variables
        .insert("test-id".to_owned(), variable("test", json!(7)));
    let project = ScratchProject {
        targets: vec![stage, nested_target()],
    };
    assert_eq!(
        generate(&project),
        concat!(
            "project {\n",
            "    stage \"Stage\" {\n",
            "        var: test = 7\n",
            "        var: test_2 = 0\n",
            "    }\n\n",
            "    sprite \"Sprite1\" {\n",
            "        on green_flag {\n",
            "            set: test = join(\"\" > 50, \"banana\") + 2\n",
            "        }\n",
            "    }\n",
            "}\n",
        )
    );
}

#[test]
fn unknown_opcodes_are_visible_and_do_not_stop_the_script() {
    let mut say = block("looks_say", &[("MESSAGE", reference("reporter"))]);
    say.next = Some("unsupported".to_owned());
    let mut unsupported = block("extension_unknown_statement", &[]);
    unsupported.next = Some("show".to_owned());
    let project = ScratchProject {
        targets: vec![target(vec![
            ("hat", hat("say")),
            ("say", say),
            ("reporter", block("extension_unknown_reporter", &[])),
            ("unsupported", unsupported),
            ("show", block("looks_show", &[])),
        ])],
    };
    let output = generate(&project);
    assert!(output.contains("say unknown(\"extension_unknown_reporter\")\n"));
    assert!(output.contains("unknown \"extension_unknown_statement\"\n            show\n"));
}

#[test]
fn ignores_disconnected_reporters_and_unsupported_hats() {
    let mut reporter = block(
        "operator_add",
        &[("NUM1", number("5")), ("NUM2", number("3"))],
    );
    reporter.top_level = true;
    let mut unsupported_hat = hat("reporter");
    unsupported_hat.opcode = "event_unknown_hat".to_owned();
    let project = ScratchProject {
        targets: vec![target(vec![
            ("reporter", reporter),
            ("key", unsupported_hat),
        ])],
    };
    assert_eq!(
        generate(&project),
        "project {\n    sprite \"Sprite1\" {\n    }\n}\n"
    );
}

#[test]
fn preserves_precedence_and_right_hand_grouping() {
    let target = target(vec![
        (
            "sum",
            block(
                "operator_add",
                &[("NUM1", number("1")), ("NUM2", number("2"))],
            ),
        ),
        (
            "product",
            block(
                "operator_multiply",
                &[("NUM1", reference("sum")), ("NUM2", number("3"))],
            ),
        ),
        (
            "difference",
            block(
                "operator_subtract",
                &[("NUM1", number("2")), ("NUM2", number("3"))],
            ),
        ),
        (
            "root",
            block(
                "operator_subtract",
                &[("NUM1", number("1")), ("NUM2", reference("difference"))],
            ),
        ),
        (
            "compare",
            block(
                "operator_equals",
                &[
                    ("OPERAND1", reference("product")),
                    ("OPERAND2", number("9")),
                ],
            ),
        ),
        (
            "not",
            block("operator_not", &[("OPERAND", reference("compare"))]),
        ),
    ]);
    assert_eq!(render_expression(&target, "product"), "(1 + 2) * 3");
    assert_eq!(render_expression(&target, "root"), "1 - (2 - 3)");
    assert_eq!(render_expression(&target, "not"), "!((1 + 2) * 3 == 9)");
}

#[test]
fn preserves_literal_types_and_escapes_strings() {
    let target = target(vec![]);
    let generator = TargetGenerator::new(&target, None);
    let cases = [
        (number("50"), "50"),
        (text("50"), "\"50\""),
        (text(""), "\"\""),
        (text("a\"b\\c\n"), "\"a\\\"b\\\\c\\n\""),
        (number("+5"), "5"),
        (number("banana"), "unknown(\"invalid numeric literal\")"),
    ];
    for (input, expected) in cases {
        assert_eq!(
            generator
                .input_expression(Some(&input), &mut HashSet::new())
                .text,
            expected
        );
    }
}

#[test]
fn follows_next_links_and_sorts_independent_scripts() {
    let mut show = block("looks_show", &[]);
    show.next = Some("a-hide".to_owned());
    let mut target = target(vec![
        ("z-hat", hat("z-show")),
        ("z-show", show),
        ("a-hide", block("looks_hide", &[])),
        ("a-hat", hat("say")),
        (
            "say",
            block("looks_say", &[("MESSAGE", text("first script"))]),
        ),
    ]);
    target
        .variables
        .insert("z-id".to_owned(), variable("apple", json!(1)));
    target
        .variables
        .insert("a-id".to_owned(), variable("zebra", json!(2)));
    let mut project = ScratchProject {
        targets: vec![target],
    };
    let first = generate(&project);
    assert!(first.find("first script").unwrap() < first.find("            show").unwrap());
    assert!(first.contains("            show\n            hide\n"));
    assert!(first.find("var: apple").unwrap() < first.find("var: zebra").unwrap());

    let target = &mut project.targets[0];
    let mut blocks: Vec<_> = target.blocks.drain().collect();
    blocks.sort_by(|(left, _), (right, _)| right.cmp(left));
    target.blocks = blocks.into_iter().collect();
    let variables: Vec<_> = target.variables.drain().collect();
    target.variables = variables.into_iter().rev().collect();
    assert_eq!(generate(&project), first);
}

#[test]
fn handles_missing_inputs_invalid_references_and_cycles() {
    let mut cycle = block("looks_show", &[]);
    cycle.next = Some("cycle".to_owned());
    let target = target(vec![
        ("hat", hat("cycle")),
        ("cycle", cycle),
        (
            "expression-cycle",
            block(
                "operator_not",
                &[("OPERAND", reference("expression-cycle"))],
            ),
        ),
        ("missing-input", block("operator_not", &[])),
        (
            "invalid-input",
            block("operator_not", &[("OPERAND", json!([1]))]),
        ),
    ]);
    assert_eq!(
        render_expression(&target, "gone"),
        "unknown(\"missing block: gone\")"
    );
    assert!(render_expression(&target, "expression-cycle").contains("cyclic expression reference"));
    assert_eq!(render_expression(&target, "missing-input"), "!false");
    for id in ["invalid-input"] {
        assert_eq!(
            render_expression(&target, id),
            "!unknown(\"missing or invalid input\")"
        );
    }
    let mut project = ScratchProject {
        targets: vec![target],
    };
    let output = generate(&project);
    assert_eq!(output.matches("            show\n").count(), 1);
    assert!(output.contains("unknown \"cyclic statement reference: cycle\""));
    project.targets[0].blocks.get_mut("hat").unwrap().next = Some("gone".to_owned());
    assert!(generate(&project).contains("unknown \"missing block: gone\""));
}

#[test]
fn generates_nested_control_bodies_and_then_continues() {
    let mut repeat = block(
        "control_repeat",
        &[("TIMES", number("2")), ("SUBSTACK", reference("if"))],
    );
    repeat.next = Some("say".to_owned());
    let conditional = block(
        "control_if_else",
        &[
            ("CONDITION", reference("compare")),
            ("SUBSTACK", reference("show")),
            ("SUBSTACK2", reference("hide")),
        ],
    );
    let project = ScratchProject {
        targets: vec![target(vec![
            ("hat", hat("repeat")),
            ("repeat", repeat),
            ("if", conditional),
            ("show", block("looks_show", &[])),
            ("hide", block("looks_hide", &[])),
            (
                "compare",
                block(
                    "operator_gt",
                    &[("OPERAND1", number("1")), ("OPERAND2", number("0"))],
                ),
            ),
            ("say", block("looks_say", &[("MESSAGE", text("done"))])),
        ])],
    };
    assert!(generate(&project).contains(concat!(
        "            repeat 2 {\n",
        "                if 1 > 0 {\n",
        "                    show\n",
        "                } else {\n",
        "                    hide\n",
        "                }\n",
        "            }\n",
        "            say \"done\"\n",
    )));
}

#[test]
fn resolves_variable_ids_and_keeps_cloud_and_string_initializers() {
    let mut stage = target(vec![]);
    stage.name = "Stage".to_owned();
    stage.is_stage = true;
    stage
        .variables
        .insert("global".to_owned(), variable("global name", json!("007")));
    stage.variables.get_mut("global").unwrap().is_cloud = true;
    let mut sprite = target(vec![]);
    sprite
        .variables
        .insert("local".to_owned(), variable("local name", json!(false)));
    let generator = TargetGenerator::new(&sprite, Some(&stage));
    for (id, expected) in [("global", "global name"), ("local", "local name")] {
        let input = json!([2, [12, "old name", id]]);
        assert_eq!(
            generator
                .input_expression(Some(&input), &mut HashSet::new())
                .text,
            identifier(expected)
        );
    }
    let output = generate(&ScratchProject {
        targets: vec![stage, sprite],
    });
    assert!(output.contains("cloud var: global_name = \"007\""));
    assert!(output.contains("var: local_name = false"));
}

#[test]
fn all_listed_blocks_render_their_expected_source() {
    let mut seen = HashSet::new();
    for row in include_str!("block_cases.tsv")
        .lines()
        .filter(|s| !s.starts_with('#'))
    {
        let columns: Vec<_> = row.splitn(5, '\t').collect();
        assert_eq!(columns.len(), 5, "bad fixture: {row}");
        let [opcode, kind, inputs, fields, expected] = columns[..] else {
            unreachable!()
        };
        assert!(seen.insert(opcode), "duplicate fixture: {opcode}");
        let mut node = json!({
            "opcode": opcode,
            "inputs": serde_json::from_str::<Value>(inputs).unwrap(),
            "fields": serde_json::from_str::<Value>(fields).unwrap(),
            "topLevel": kind == "hat"
        });
        let expected = expected.replace("\\n", "\n");
        let (header, body) = if kind == "hat" {
            node["next"] = json!("body");
            (expected, "show".to_owned())
        } else {
            (
                "on green_flag".to_owned(),
                if kind == "reporter" {
                    format!("say {expected}")
                } else {
                    expected
                },
            )
        };
        let mut blocks = json!({
            "node": node,
            "body": {"opcode": "looks_show"},
            "else-body": {"opcode": "looks_hide"}
        });
        if kind != "hat" {
            blocks["hat"] = json!({"opcode":"event_whenflagclicked", "topLevel":true,
                "next": if kind == "reporter" {"say"} else {"node"}});
        }
        if kind == "reporter" {
            blocks["say"] = json!({"opcode":"looks_say", "inputs":{"MESSAGE":[2,"node"]}});
        }
        let json =
            json!({"targets":[{"name":"Sprite1", "isStage":false, "blocks":blocks}]}).to_string();
        let project = crate::parser::parse_project(&json).unwrap();
        let expected = format!(
            "project {{\n    sprite \"Sprite1\" {{\n        {header} {{\n{}\n        }}\n    }}\n}}\n",
            body.lines()
                .map(|s| format!("            {s}"))
                .collect::<Vec<_>>()
                .join("\n")
        );
        assert_eq!(generate(&project), expected, "opcode: {opcode}");
    }
    // The other four concrete opcodes are the procedure fixtures below.
    assert_eq!(seen.len(), 176);
}

#[test]
fn reads_integer_json_values_and_nested_join_inside_arithmetic() {
    let json = json!({"targets":[{
        "name":"Stage", "isStage":true, "variables":{"v":["test 2",12]},
        "blocks":{
            "hat":{"opcode":"event_whenflagclicked","topLevel":true,"next":"set"},
            "set":{"opcode":"data_setvariableto","fields":{"VARIABLE":["stale name","v"]},"inputs":{"VALUE":[3,"add",[4,"0"]]}},
            "add":{"opcode":"operator_add","inputs":{"NUM1":[3,"join",[4,0]],"NUM2":[1,[4,2]]}},
            "join":{"opcode":"operator_join","inputs":{"STRING1":[1,[4,1]],"STRING2":[1,[4,1]]}},
            "loose":{"opcode":"text","topLevel":true,"shadow":true,"fields":{"TEXT":[12,null]}}
        }
    }]}).to_string();
    let project = crate::parser::parse_project(&json).unwrap();
    let output = generate(&project);
    assert!(output.contains("var: test_2 = 12"));
    assert!(output.contains("set: test_2 = join(1, 1) + 2"));
    assert_eq!(render_expression(&project.targets[0], "loose"), "\"12\"");
    assert!(!output.contains("unknown"));
}

#[test]
fn decodes_lists_compact_reporters_and_nested_menu_references() {
    let json = json!({"targets":[{
        "name":"Sprite1", "isStage":false,
        "variables":{"v":["test 2",0]}, "lists":{"l":["to do",[12,"12",true]]},
        "blocks":{
            "loose-var":[12,"old variable name","v",15,20],
            "loose-list":[13,"old list name","l",30,40],
            "hat":{"opcode":"event_whenflagclicked","topLevel":true,"next":"delete"},
            "delete":{"opcode":"data_deleteoflist","fields":{"LIST":["old name","l"]},"inputs":{"INDEX":[1,"index"]},"next":"sound"},
            "index":{"opcode":"data_listindexall","shadow":true,"fields":{"INDEX":["all",null]}},
            "sound":{"opcode":"sound_play","inputs":{"SOUND_MENU":[1,"menu"]}},
            "menu":{"opcode":"sound_sounds_menu","shadow":true,"fields":{"SOUND_MENU":["Meow 2",null]}}
        }
    }]}).to_string();
    let project = crate::parser::parse_project(&json).unwrap();
    let target = &project.targets[0];
    assert_eq!(target.blocks["loose-var"].x, Some(15.0));
    assert_eq!(target.blocks["loose-var"].fields["VARIABLE"][1], "v");
    assert_eq!(render_expression(target, "loose-var"), "test_2");
    assert_eq!(
        render_expression(target, "loose-list"),
        "list_contents(to_do)"
    );
    let output = generate(&project);
    assert!(output.contains("list: to_do = [12, \"12\", true]"));
    assert!(output.contains("delete: to_do at \"all\"\n            play_sound \"Meow 2\""));
    assert!(!output.contains("unknown"));
}

#[test]
fn normalizes_names_without_merging_distinct_ids_or_scopes() {
    let project = crate::parser::parse_project(
        &json!({"targets":[
            {"name":"Stage","isStage":true,"variables":{"g":["test 2",7]}},
            {"name":"Sprite1","isStage":false,
                "variables":{"a":["test 2",1],"b":["test_2",2],"c":["test_2__2",3],"d":["true",4]},
                "lists":{"l":["test 2",[]]}}
        ]})
        .to_string(),
    )
    .unwrap();
    let generator = TargetGenerator::new(&project.targets[1], Some(&project.targets[0]));
    assert_eq!(generator.symbol_name(Some("a"), "stale", false), "test_2");
    assert_eq!(generator.symbol_name(Some("l"), "stale", true), "test_2__3");
    assert_eq!(
        generator.symbol_name(Some("b"), "stale", false),
        "test_2__4"
    );
    assert_eq!(
        generator.symbol_name(Some("c"), "stale", false),
        "test_2__2"
    );
    assert_eq!(
        generator.symbol_name(Some("g"), "stale", false),
        "global.test_2"
    );
    assert_eq!(generator.symbol_name(Some("d"), "stale", false), "_true");
    let output = generate(&project);
    assert!(output.contains("var: test_2 = 1"));
    assert!(output.contains("var: test_2__4 = 2"));
    assert!(output.contains("list: test_2__3 = []"));
}

fn procedure_project() -> ScratchProject {
    // Mutation arrays in SB3 are JSON encoded strings; calls use IDs in their
    // declared order, regardless of input map ordering or parameter names.
    crate::parser::parse_project(&json!({"targets":[{
        "name":"Sprite1", "isStage":false, "blocks":{
            "a-hat":{"opcode":"event_whenflagclicked","topLevel":true,"next":"call"},
            "call":{"opcode":"procedures_call","inputs":{"z":[1,[10,"hello"]],"a":[2,"compare"]},
                "mutation":{"proccode":"mix %s %b","argumentids":"[\"z\",\"a\"]","warp":"true"}},
            "compare":{"opcode":"operator_gt","inputs":{"OPERAND1":[1,[4,3]],"OPERAND2":[1,[4,2]]}},
            "definition":{"opcode":"procedures_definition","topLevel":true,"next":"if","inputs":{"custom_block":[1,"prototype"]}},
            "prototype":{"opcode":"procedures_prototype","shadow":true,"parent":"definition",
                "mutation":{"proccode":"mix %s %b","argumentids":"[\"z\",\"a\"]","argumentnames":"[\"text value\",\"is ready\"]","argumentdefaults":"[\"\",false]","warp":"true"}},
            "if":{"opcode":"control_if","parent":"definition","inputs":{"CONDITION":[2,"boolean-arg"],"SUBSTACK":[2,"say"]}},
            "boolean-arg":{"opcode":"argument_reporter_boolean","parent":"if","fields":{"VALUE":["is ready",null]}},
            "say":{"opcode":"looks_say","parent":"if","inputs":{"MESSAGE":[2,"text-arg"]},"next":"recursive"},
            "text-arg":{"opcode":"argument_reporter_string_number","parent":"say","fields":{"VALUE":["text value",null]}},
            "recursive":{"opcode":"procedures_call","parent":"say","inputs":{"z":[2,"text-arg"]},
                "mutation":{"proccode":"mix %s %b","argumentids":"[\"z\",\"a\"]","warp":true}},
            "z-editor":{"opcode":"procedures_declaration","topLevel":true,
                "mutation":{"proccode":"draft %n","argumentids":["n"],"argumentnames":["how many"],"argumentdefaults":[1],"warp":false}}
        }
    }]}).to_string()).unwrap()
}

#[test]
fn generates_procedures_parameters_ordered_calls_warp_and_editor_declarations() {
    let project = procedure_project();
    assert_eq!(
        generate(&project),
        concat!(
            "project {\n",
            "    sprite \"Sprite1\" {\n",
            "        on green_flag {\n",
            "            call: mix(\"hello\", 3 > 2)\n",
            "        }\n\n",
            "        define: mix(text_value: value = \"\", is_ready: bool = false) warp {\n",
            "            if argument(is_ready) {\n",
            "                say argument(text_value)\n",
            "                call: mix(argument(text_value), false)\n",
            "            }\n",
            "        }\n\n",
            "        declare: draft(how_many: number = 1)\n",
            "    }\n",
            "}\n",
        )
    );
    assert_eq!(
        render_expression(&project.targets[0], "prototype"),
        "procedure(mix)"
    );
    assert_eq!(
        render_expression(&project.targets[0], "z-editor"),
        "procedure(draft)"
    );
}

#[test]
fn malformed_procedures_report_errors_and_keep_following_statements() {
    let mut project = procedure_project();
    let blocks = &mut project.targets[0].blocks;
    blocks.get_mut("call").unwrap().mutation["argumentids"] = json!("invalid JSON");
    blocks.get_mut("call").unwrap().next = Some("after".to_owned());
    blocks.insert("after".to_owned(), block("looks_show", &[]));
    blocks.get_mut("definition").unwrap().inputs.clear();
    let output = generate(&project);
    assert!(output.contains("unknown \"invalid procedure argumentids\"\n            show"));
    assert!(output.contains("unknown \"missing procedure prototype\""));
}

#[test]
fn malformed_compact_reporters_return_parse_errors() {
    for entry in [
        json!([12]),
        json!([13, "items", null]),
        json!([42, "name", "id"]),
    ] {
        let json =
            json!({"targets":[{"name":"Stage","isStage":true,"blocks":{"bad":entry}}]}).to_string();
        assert!(crate::parser::parse_project(&json).is_err());
    }
}

#[test]
fn procedure_labels_keep_escaped_placeholders_and_distinguish_signatures() {
    let mut sprite = target(vec![]);
    for (id, code, args) in [
        ("escaped", r"print \%s", "[]"),
        ("number", "do %n", "[\"arg\"]"),
        ("boolean", "do %b", "[\"arg\"]"),
    ] {
        let mut call = block("procedures_call", &[("arg", number("9"))]);
        call.mutation = json!({"proccode":code,"argumentids":args,"warp":false});
        sprite.blocks.insert(id.to_owned(), call);
    }
    let generator = TargetGenerator::new(&sprite, None);
    assert_eq!(
        generator.procedure_call(&sprite.blocks["escaped"]).unwrap(),
        "call: print__u25_s()"
    );
    assert_eq!(
        generator.procedure_call(&sprite.blocks["boolean"]).unwrap(),
        "call: do(9)"
    );
    assert_eq!(
        generator.procedure_call(&sprite.blocks["number"]).unwrap(),
        "call: do__2(9)"
    );
}

#[test]
fn category_labels_are_not_executable_scripts() {
    let labels = [
        "colour",
        "math",
        "texts",
        "control",
        "data",
        "defaultToolbox",
        "event",
        "extensions",
        "looks",
        "motion",
        "operators",
        "sensing",
        "sound",
    ];
    let mut sprite = target(vec![("body", block("looks_show", &[]))]);
    for label in labels {
        let mut node = hat("body");
        node.opcode = label.to_owned();
        sprite.blocks.insert(label.to_owned(), node);
    }
    assert_eq!(
        generate(&ScratchProject {
            targets: vec![sprite]
        }),
        "project {\n    sprite \"Sprite1\" {\n    }\n}\n"
    );
}

#[test]
fn all_primitive_tags_keep_the_active_value_and_type() {
    let sprite = target(vec![]);
    let generator = TargetGenerator::new(&sprite, None);
    for code in 4..=13 {
        let expected = match code {
            4..=8 => "12",
            9..=11 => "\"12\"",
            12 => "_12",
            13 => "list_contents(_12)",
            _ => unreachable!(),
        };
        for input_kind in 1..=3 {
            let input = json!([input_kind, [code, "12", "id"], [10, "unused shadow"]]);
            assert_eq!(
                generator
                    .input_expression(Some(&input), &mut HashSet::new())
                    .text,
                expected
            );
        }
    }
}

#[test]
fn surplus_procedure_defaults_do_not_discard_definitions_or_bodies() {
    let mut project = procedure_project();
    let baseline = generate(&project);
    let blocks = &mut project.targets[0].blocks;
    blocks.get_mut("prototype").unwrap().mutation["argumentdefaults"] =
        json!("[\"\",false,\"stale\",123]");
    assert_eq!(generate(&project), baseline);

    let mut definition = block(
        "procedures_definition",
        &[("custom_block", reference("zero"))],
    );
    definition.top_level = true;
    definition.next = Some("body".to_owned());
    let mut prototype = block("procedures_prototype", &[]);
    prototype.mutation = json!({"proccode":"No arguments", "argumentids":"[]",
        "argumentnames":"[]", "argumentdefaults":"[\"stale\",\"stale\"]"});
    let output = generate(&ScratchProject {
        targets: vec![target(vec![
            ("definition", definition),
            ("zero", prototype),
            ("body", block("pen_stamp", &[])),
        ])],
    });
    assert!(output.contains("define: No_arguments() {\n            stamp\n"));
    assert!(!output.contains("unknown"));

    project.targets[0]
        .blocks
        .get_mut("prototype")
        .unwrap()
        .mutation["argumentdefaults"] = json!("[]");
    assert!(generate(&project).contains("invalid procedure argument names or defaults"));
}

#[test]
fn pen_commands_keep_nested_inputs_and_special_list_indexes() {
    let mut sprite = target(vec![
        ("hat", hat("color")),
        (
            "color",
            block(
                "pen_setPenColorToColor",
                &[("COLOR", json!([1, [9, "#123456"]]))],
            ),
        ),
        (
            "size",
            block("pen_setPenSizeTo", &[("SIZE", reference("sum"))]),
        ),
        (
            "sum",
            block(
                "operator_add",
                &[("NUM1", number("2")), ("NUM2", number(""))],
            ),
        ),
        ("menu", block("pen_menu_colorParam", &[])),
        (
            "param",
            block(
                "pen_setPenColorParamTo",
                &[("COLOR_PARAM", reference("menu")), ("VALUE", number("50"))],
            ),
        ),
    ]);
    sprite
        .blocks
        .get_mut("menu")
        .unwrap()
        .fields
        .insert("colorParam".to_owned(), json!(["transparency", null]));
    sprite.blocks.get_mut("color").unwrap().next = Some("size".to_owned());
    sprite.blocks.get_mut("size").unwrap().next = Some("param".to_owned());
    let output = generate(&ScratchProject {
        targets: vec![sprite],
    });
    assert!(output.contains(concat!(
        "set_pen_color \"#123456\"\n",
        "            set_pen_size 2 + 0\n",
        "            set_pen_color \"transparency\" to 50\n"
    )));

    for selector in ["last", "all", "random", "any"] {
        let mut delete = block("data_deleteoflist", &[("INDEX", json!([1, [7, selector]]))]);
        delete
            .fields
            .insert("LIST".to_owned(), json!(["items", "list-id"]));
        let project = ScratchProject {
            targets: vec![target(vec![("hat", hat("delete")), ("delete", delete)])],
        };
        assert!(generate(&project).contains(&format!("delete: items at \"{selector}\"")));
    }
    assert_eq!(number_literal(&json!(" \t")).text, "0");
    assert_eq!(
        number_literal(&json!("last")).text,
        "unknown(\"invalid numeric literal\")"
    );
}
