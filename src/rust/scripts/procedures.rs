use serde_json::Value;

use crate::parser::{InputValue, ScratchTarget, parse_input};

#[derive(Clone)]
pub(in crate::rust) struct ProcedureSignature {
    pub(in crate::rust) frame_id: usize,
    pub(in crate::rust) definition_id: String,
    pub(in crate::rust) proccode: String,
    pub(in crate::rust) argument_ids: Vec<String>,
    pub(in crate::rust) argument_names: Vec<String>,
    pub(in crate::rust) warp: bool,
}

// collect valid custom block definitions in stable frame order.
pub(super) fn procedure_signatures(target: &ScratchTarget) -> Vec<ProcedureSignature> {
    let mut definitions: Vec<_> = target
        .blocks
        .iter()
        .filter(|(_, block)| block.opcode == "procedures_definition")
        .collect();
    definitions.sort_by(|(left_id, _), (right_id, _)| left_id.cmp(right_id));

    let mut procedures = Vec::new();
    for (definition_id, definition) in definitions {
        let Some(InputValue::BlockReference(prototype_id)) =
            definition.inputs.get("custom_block").and_then(parse_input)
        else {
            continue;
        };
        let Some(prototype) = target.blocks.get(prototype_id) else {
            continue;
        };
        let Some(mutation) = prototype.mutation.as_object() else {
            continue;
        };
        let Some(proccode) = mutation.get("proccode").and_then(Value::as_str) else {
            continue;
        };

        let argument_names = parse_string_array(mutation.get("argumentnames"));
        let argument_ids = parse_string_array(mutation.get("argumentids"));
        procedures.push(ProcedureSignature {
            frame_id: procedures.len() + 1,
            definition_id: definition_id.to_owned(),
            proccode: proccode.to_owned(),
            argument_ids,
            argument_names,
            warp: mutation
                .get("warp")
                .and_then(|value| {
                    value
                        .as_bool()
                        .or_else(|| value.as_str().and_then(|value| value.parse().ok()))
                })
                .unwrap_or(false),
        });
    }
    procedures
}

fn parse_string_array(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_str)
        .and_then(|encoded| serde_json::from_str::<Vec<String>>(encoded).ok())
        .unwrap_or_default()
}
