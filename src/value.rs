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

use serde_json::Value;

#[derive(Debug, PartialEq)]
pub enum InferredValue {
    I64(i64),
    F64(f64),
    String(String),
    Bool(bool),
}

impl InferredValue {
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::I64(_) => "i64",
            Self::F64(_) => "f64",
            Self::String(_) => "string",
            Self::Bool(_) => "bool",
        }
    }
}

/// Infer a Rust scalar from a Scratch value. Numeric and boolean strings are
/// recognized; other strings keep their original contents.
pub fn infer_value(value: &Value) -> Option<InferredValue> {
    match value {
        Value::Bool(value) => Some(InferredValue::Bool(*value)),
        Value::Number(value) => value
            .as_i64()
            .map(InferredValue::I64)
            .or_else(|| value.as_f64().map(InferredValue::F64)),
        Value::String(value) => {
            let trimmed = value.trim();
            if trimmed.eq_ignore_ascii_case("true") {
                Some(InferredValue::Bool(true))
            } else if trimmed.eq_ignore_ascii_case("false") {
                Some(InferredValue::Bool(false))
            } else if let Ok(number) = trimmed.parse::<i64>() {
                Some(InferredValue::I64(number))
            } else if let Ok(number) = trimmed.parse::<f64>() {
                if number.is_finite() {
                    Some(InferredValue::F64(number))
                } else {
                    Some(InferredValue::String(value.clone()))
                }
            } else {
                Some(InferredValue::String(value.clone()))
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{InferredValue, infer_value};
    use serde_json::json;

    #[test]
    fn infers_scalar_values() {
        assert_eq!(infer_value(&json!(42)), Some(InferredValue::I64(42)));
        assert_eq!(infer_value(&json!("42")), Some(InferredValue::I64(42)));
        assert_eq!(infer_value(&json!(2.5)), Some(InferredValue::F64(2.5)));
        assert_eq!(infer_value(&json!("2.5")), Some(InferredValue::F64(2.5)));
        assert_eq!(infer_value(&json!(true)), Some(InferredValue::Bool(true)));
        assert_eq!(
            infer_value(&json!("FALSE")),
            Some(InferredValue::Bool(false))
        );
        assert_eq!(
            infer_value(&json!("hello")),
            Some(InferredValue::String("hello".to_owned()))
        );
    }

    #[test]
    fn preserves_values_that_are_not_finite_numbers() {
        assert_eq!(
            infer_value(&json!("NaN")),
            Some(InferredValue::String("NaN".to_owned()))
        );
        assert_eq!(infer_value(&json!(null)), None);
    }
}
