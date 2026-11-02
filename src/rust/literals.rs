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

//! formats Scratch values and small pieces of generated Rust source.
//!
//! keeps string escaping and numeric literal formatting consistent across
//! variable declarations, expressions, and generated statement lines.

use serde_json::Value;

use crate::parser::ScratchBlock;

// read the value portion of a Scratch block field.
pub(super) fn field_value<'a>(block: &'a ScratchBlock, field: &str) -> Option<&'a Value> {
    block.fields.get(field)?.as_array()?.first()
}

// convert a saved Scratch variable value to its Rust source representation.
pub(super) fn literal(value: &Value) -> String {
    match value {
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value
            .as_f64()
            .map(number_literal_f64)
            .unwrap_or_else(|| "0.0_f64".to_owned()),
        Value::String(value) => quote(value),
        _ => "0.0_f64".to_owned(),
    }
}

// convert a numeric value or numeric string into a Rust f64 literal.
pub(super) fn number_literal(value: &Value) -> String {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|text| text.parse::<f64>().ok()))
        .map(number_literal_f64)
        .unwrap_or_else(|| "0.0_f64".to_owned())
}

// format an f64 so parsing the generated literal recreates the same value.
fn number_literal_f64(value: f64) -> String {
    format!("{value:?}_f64")
}

// preserve a text value as an escaped Rust string literal.
pub(super) fn text_literal(value: &Value) -> String {
    value.as_str().map(quote).unwrap_or_else(|| quote(""))
}

// escape and quote a string using Rust-compatible debug formatting.
pub(super) fn quote(value: &str) -> String {
    format!("{value:?}")
}

// append one indented source line to the generated module.
pub(super) fn line(output: &mut String, indent: usize, text: &str) {
    output.push_str(&"    ".repeat(indent));
    output.push_str(text);
    output.push('\n');
}
