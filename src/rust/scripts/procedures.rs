use std::collections::HashSet;

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
    let mut used_frame_ids = HashSet::new();
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
            frame_id: stable_frame_id(definition_id, &mut used_frame_ids),
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

fn stable_frame_id(definition_id: &str, used: &mut HashSet<usize>) -> usize {
    for salt in 0_u32.. {
        let mut hash = 0x811c_9dc5_u32;
        for byte in definition_id.bytes().chain(salt.to_le_bytes().into_iter()) {
            hash ^= u32::from(byte);
            hash = hash.wrapping_mul(0x0100_0193);
        }
        let frame_id = hash.max(1) as usize;
        if used.insert(frame_id) {
            return frame_id;
        }
    }
    unreachable!("procedure frame ID collision space exhausted")
}

fn parse_string_array(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_str)
        .and_then(|encoded| serde_json::from_str::<Vec<String>>(encoded).ok())
        .unwrap_or_default()
}
