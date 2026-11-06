//! lower Scratch reporters while keeping dynamic and proven-numeric paths distinct.

use std::collections::{HashMap, HashSet};

use serde_json::Value;

use crate::parser::{InputValue, ScratchBlock, ScratchTarget, parse_input};

use super::{
    literals::{field_value, number_literal, quote, text_literal},
    project::{ListInfo, VariableInfo},
    statements::variable_field,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LocalValueKind {
    Number,
    Text,
    Boolean,
}

pub(super) struct LocalExpression {
    pub(super) code: String,
    pub(super) kind: LocalValueKind,
}

mod local;
mod reporters;
#[cfg(test)]
mod tests;

use reporters::identical_literal_inputs;

pub(super) use local::{
    collect_numeric_local_slots, expression_argument, local_expression, local_to_bool,
    local_to_number,
};
pub(super) use reporters::{
    boolean_input, constant_numeric_input, constant_random_bounds, expression,
};
