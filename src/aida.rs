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

use std::collections::{HashMap, HashSet};

use serde_json::Value;

use crate::parser::{InputValue, ScratchBlock, ScratchProject, ScratchTarget, parse_input};

mod blocks;
mod procedures;
use blocks::{Rule, rule};

const OR: u8 = 1;
const AND: u8 = 2;
const COMPARISON: u8 = 3;
const SUM: u8 = 4;
const PRODUCT: u8 = 5;
const UNARY: u8 = 6;
const ATOM: u8 = 7;

struct Expression {
    text: String,
    precedence: u8,
}

impl Expression {
    fn atom(text: String) -> Self {
        Self {
            text,
            precedence: ATOM,
        }
    }

    fn unknown(reason: &str) -> Self {
        Self::atom(format!("unknown({})", quote(reason)))
    }

    fn in_context(self, minimum_precedence: u8) -> String {
        if self.precedence < minimum_precedence {
            format!("({})", self.text)
        } else {
            self.text
        }
    }

    fn binary(left: Self, operator: &str, right: Self, precedence: u8) -> Self {
        // Preserve grouping on the right, including a - (b - c) and a / (b * c).
        // Comparisons must not turn nested comparisons into a chained comparison.
        let left_minimum = precedence + u8::from(precedence == COMPARISON);
        Self {
            text: format!(
                "{} {operator} {}",
                left.in_context(left_minimum),
                right.in_context(precedence + 1)
            ),
            precedence,
        }
    }
}

/// Render the Scratch graph as source text; unsupported or broken nodes get placeholders.
pub fn generate(project: &ScratchProject) -> String {
    let stage = project.targets.iter().find(|target| target.is_stage);
    let mut output = String::from("project {\n");
    for (index, target) in project.targets.iter().enumerate() {
        if index > 0 {
            output.push('\n');
        }
        TargetGenerator::new(target, stage).generate_target(&mut output);
    }
    output.push_str("}\n");
    output
}

struct TargetGenerator<'a> {
    target: &'a ScratchTarget,
    stage: Option<&'a ScratchTarget>,
    names: HashMap<String, String>,
    global_names: HashMap<String, String>,
    procedure_names: HashMap<String, String>,
}

impl<'a> TargetGenerator<'a> {
    fn new(target: &'a ScratchTarget, stage: Option<&'a ScratchTarget>) -> Self {
        let declarations = |target: &ScratchTarget| {
            allocate_names(
                target
                    .variables
                    .iter()
                    .map(|(id, v)| (id.as_str(), v.name.as_str()))
                    .chain(
                        target
                            .lists
                            .iter()
                            .map(|(id, l)| (id.as_str(), l.name.as_str())),
                    ),
            )
        };
        let mut codes: Vec<_> = target
            .blocks
            .values()
            .filter_map(|block| block.mutation.get("proccode").and_then(Value::as_str))
            .collect();
        codes.sort_unstable();
        codes.dedup();
        let labels: Vec<_> = codes
            .iter()
            .map(|code| (code.to_string(), procedures::parts(code).0))
            .collect();
        Self {
            target,
            stage,
            names: declarations(target),
            global_names: stage.map(declarations).unwrap_or_default(),
            procedure_names: allocate_names(
                labels
                    .iter()
                    .map(|(code, label)| (code.as_str(), label.trim())),
            ),
        }
    }

    fn generate_target(&self, output: &mut String) {
        let kind = if self.target.is_stage {
            "stage"
        } else {
            "sprite"
        };
        line(
            output,
            1,
            &format!("{kind} {} {{", quote(&self.target.name)),
        );

        let mut variables: Vec<_> = self.target.variables.iter().collect();
        variables.sort_by(|(left_id, left), (right_id, right)| {
            left.name.cmp(&right.name).then(left_id.cmp(right_id))
        });
        for (id, variable) in variables {
            let kind = if variable.is_cloud {
                "cloud var"
            } else {
                "var"
            };
            line(
                output,
                2,
                &format!(
                    "{kind}: {} = {}",
                    self.symbol_name(Some(id), &variable.name, false),
                    scalar(&variable.value)
                ),
            );
        }

        let mut lists: Vec<_> = self.target.lists.iter().collect();
        lists.sort_by(|(a_id, a), (b_id, b)| a.name.cmp(&b.name).then(a_id.cmp(b_id)));
        for (id, list) in lists {
            line(
                output,
                2,
                &format!(
                    "list: {} = [{}]",
                    self.symbol_name(Some(id), &list.name, true),
                    list.items.iter().map(scalar).collect::<Vec<_>>().join(", ")
                ),
            );
        }

        let mut scripts: Vec<_> = self
            .target
            .blocks
            .iter()
            .filter(|(_, block)| {
                block.top_level
                    && !block.shadow
                    && (matches!(rule(&block.opcode), Some(Rule::Hat(_)))
                        || matches!(
                            block.opcode.as_str(),
                            "procedures_definition" | "procedures_declaration"
                        ))
            })
            .collect();
        scripts.sort_by(|(left_id, _), (right_id, _)| left_id.cmp(right_id));
        for (index, (id, hat)) in scripts.into_iter().enumerate() {
            if index > 0 || !self.target.variables.is_empty() || !self.target.lists.is_empty() {
                output.push('\n');
            }
            let header = match rule(&hat.opcode) {
                Some(Rule::Hat(template)) => format!(
                    "on {}",
                    self.render_template(hat, template, &mut HashSet::new())
                ),
                _ => match self.procedure_header(hat) {
                    Ok(header) => header,
                    Err(reason) => {
                        unknown_statement(output, 2, &reason);
                        continue;
                    }
                },
            };
            if hat.opcode == "procedures_declaration" {
                line(output, 2, &header);
                continue;
            }
            line(output, 2, &format!("{header} {{"));
            self.generate_statements(
                hat.next.as_deref(),
                3,
                output,
                &mut HashSet::from([id.clone()]),
            );
            line(output, 2, "}");
        }
        line(output, 1, "}");
    }

    fn generate_statements(
        &self,
        first: Option<&str>,
        indent: usize,
        output: &mut String,
        active: &mut HashSet<String>,
    ) {
        let mut next = first;
        let mut chain = Vec::new();
        while let Some(id) = next {
            if !active.insert(id.to_owned()) {
                unknown_statement(output, indent, &format!("cyclic statement reference: {id}"));
                break;
            }
            chain.push(id);
            let Some(block) = self.target.blocks.get(id) else {
                unknown_statement(output, indent, &format!("missing block: {id}"));
                break;
            };
            self.generate_statement(block, indent, output, active);
            next = block.next.as_deref();
        }
        for id in chain {
            active.remove(id);
        }
    }

    fn generate_statement(
        &self,
        block: &ScratchBlock,
        indent: usize,
        output: &mut String,
        active: &mut HashSet<String>,
    ) {
        if let Some(Rule::Statement(template)) = rule(&block.opcode) {
            line(
                output,
                indent,
                &self.render_template(block, template, &mut HashSet::new()),
            );
            return;
        }
        let expression = |name| {
            self.input_expression(block.inputs.get(name), &mut HashSet::new())
                .text
        };
        let statement = match block.opcode.as_str() {
            "control_repeat"
            | "control_if"
            | "control_if_else"
            | "control_forever"
            | "control_repeat_until"
            | "control_while"
            | "control_for_each"
            | "control_all_at_once" => {
                let condition = || {
                    self.boolean_input(block, "CONDITION", &mut HashSet::new())
                        .text
                };
                let header = match block.opcode.as_str() {
                    "control_repeat" => format!("repeat {}", expression("TIMES")),
                    "control_forever" => "forever".to_owned(),
                    "control_repeat_until" => format!("repeat_until {}", condition()),
                    "control_while" => format!("while {}", condition()),
                    "control_for_each" => format!(
                        "for_each: {} from 1 through {}",
                        self.symbol_field(block, "VARIABLE", false),
                        expression("VALUE")
                    ),
                    "control_all_at_once" => "all_at_once".to_owned(),
                    _ => format!("if {}", condition()),
                };
                line(output, indent, &format!("{header} {{"));
                self.generate_substack(block, "SUBSTACK", indent + 1, output, active);
                if block.opcode == "control_if_else" {
                    line(output, indent, "} else {");
                    self.generate_substack(block, "SUBSTACK2", indent + 1, output, active);
                }
                line(output, indent, "}");
                return;
            }
            "procedures_call" => match self.procedure_call(block) {
                Ok(call) => call,
                Err(reason) => {
                    unknown_statement(output, indent, &reason);
                    return;
                }
            },
            _ => {
                unknown_statement(output, indent, &block.opcode);
                return;
            }
        };
        line(output, indent, &statement);
    }

    fn generate_substack(
        &self,
        block: &ScratchBlock,
        name: &str,
        indent: usize,
        output: &mut String,
        active: &mut HashSet<String>,
    ) {
        let Some(input) = block.inputs.get(name) else {
            return;
        };
        match parse_input(input) {
            Some(InputValue::BlockReference(id)) => {
                self.generate_statements(Some(id), indent, output, active)
            }
            Some(InputValue::Empty) => {}
            _ => unknown_statement(output, indent, &format!("invalid substack: {name}")),
        }
    }

    fn input_expression(&self, input: Option<&Value>, active: &mut HashSet<String>) -> Expression {
        match input.and_then(parse_input) {
            Some(InputValue::Number(value)) => number_literal(value),
            Some(InputValue::String(value)) => text_literal(value),
            Some(InputValue::BlockReference(id)) => self.generate_expression(id, active),
            Some(InputValue::VariableReference(reference)) => {
                Expression::atom(self.symbol_name(Some(reference.id), reference.name, false))
            }
            Some(InputValue::ListReference(reference)) => Expression::atom(format!(
                "list_contents({})",
                self.symbol_name(Some(reference.id), reference.name, true)
            )),
            Some(InputValue::Empty) => Expression::unknown("empty input"),
            None => Expression::unknown("missing or invalid input"),
        }
    }

    fn generate_expression(&self, id: &str, active: &mut HashSet<String>) -> Expression {
        if !active.insert(id.to_owned()) {
            return Expression::unknown(&format!("cyclic expression reference: {id}"));
        }
        let expression = match self.target.blocks.get(id) {
            Some(block) => self.block_expression(block, active),
            None => Expression::unknown(&format!("missing block: {id}")),
        };
        active.remove(id);
        expression
    }

    fn block_expression(&self, block: &ScratchBlock, active: &mut HashSet<String>) -> Expression {
        match rule(&block.opcode) {
            Some(Rule::Reporter(template)) => {
                return Expression::atom(self.render_template(block, template, active));
            }
            Some(Rule::TextField(name)) => {
                return field_value(block, name)
                    .map(text_literal)
                    .unwrap_or_else(|| Expression::unknown(&format!("missing {name} field")));
            }
            Some(Rule::NumberField(name)) => {
                return field_value(block, name)
                    .map(number_literal)
                    .unwrap_or_else(|| Expression::unknown(&format!("missing {name} field")));
            }
            _ => {}
        }
        let binary = match block.opcode.as_str() {
            "operator_add" => Some(("+", "NUM1", "NUM2", SUM)),
            "operator_subtract" => Some(("-", "NUM1", "NUM2", SUM)),
            "operator_multiply" => Some(("*", "NUM1", "NUM2", PRODUCT)),
            "operator_divide" => Some(("/", "NUM1", "NUM2", PRODUCT)),
            "operator_gt" => Some((">", "OPERAND1", "OPERAND2", COMPARISON)),
            "operator_lt" => Some(("<", "OPERAND1", "OPERAND2", COMPARISON)),
            "operator_equals" => Some(("==", "OPERAND1", "OPERAND2", COMPARISON)),
            "operator_and" => Some(("&&", "OPERAND1", "OPERAND2", AND)),
            "operator_or" => Some(("||", "OPERAND1", "OPERAND2", OR)),
            _ => None,
        };
        if let Some((operator, left, right, precedence)) = binary {
            let boolean = matches!(block.opcode.as_str(), "operator_and" | "operator_or");
            let left = if boolean {
                self.boolean_input(block, left, active)
            } else {
                self.input_expression(block.inputs.get(left), active)
            };
            let right = if boolean {
                self.boolean_input(block, right, active)
            } else {
                self.input_expression(block.inputs.get(right), active)
            };
            return Expression::binary(left, operator, right, precedence);
        }
        match block.opcode.as_str() {
            "operator_not" => Expression {
                text: format!(
                    "!{}",
                    self.boolean_input(block, "OPERAND", active)
                        .in_context(UNARY)
                ),
                precedence: UNARY,
            },
            "argument_reporter_boolean"
            | "argument_reporter_string_number"
            | "argument_editor_boolean"
            | "argument_editor_string_number" => {
                Expression::atom(format!("argument({})", self.argument_name(block)))
            }
            "procedures_prototype" | "procedures_declaration" => match self.procedure(block) {
                Ok(procedure) => Expression::atom(format!("procedure({})", procedure.name)),
                Err(reason) => Expression::unknown(&reason),
            },
            _ => Expression::unknown(&block.opcode),
        }
    }

    fn boolean_input(
        &self,
        block: &ScratchBlock,
        name: &str,
        active: &mut HashSet<String>,
    ) -> Expression {
        let input = block.inputs.get(name);
        if input.is_none() || matches!(input.and_then(parse_input), Some(InputValue::Empty)) {
            Expression::atom("false".to_owned())
        } else {
            self.input_expression(input, active)
        }
    }

    fn render_template(
        &self,
        block: &ScratchBlock,
        template: &str,
        active: &mut HashSet<String>,
    ) -> String {
        let mut output = String::new();
        let mut rest = template;
        while let Some((prefix, tail)) = rest.split_once('{') {
            output.push_str(prefix);
            let Some((key, suffix)) = tail.split_once('}') else {
                break;
            };
            let value = if let Some(field) = key.strip_prefix('@') {
                field_literal(block, field)
            } else if let Some(field) = key.strip_prefix('$') {
                self.symbol_field(block, field, false)
            } else if let Some(field) = key.strip_prefix('#') {
                self.symbol_field(block, field, true)
            } else if let Some(input) = key.strip_prefix('?') {
                self.boolean_input(block, input, active).text
            } else if key == "INDEX" {
                self.list_index(block.inputs.get(key), active).text
            } else {
                self.input_expression(block.inputs.get(key), active).text
            };
            output.push_str(&value);
            rest = suffix;
        }
        output.push_str(rest);
        output
    }

    fn list_index(&self, input: Option<&Value>, active: &mut HashSet<String>) -> Expression {
        // Scratch saves its special list selectors in numeric shadow slots.
        if let Some(InputValue::Number(value)) = input.and_then(parse_input) {
            if matches!(value.as_str(), Some("last" | "all" | "random" | "any")) {
                return text_literal(value);
            }
        }
        self.input_expression(input, active)
    }

    fn symbol_field(&self, block: &ScratchBlock, field: &str, list: bool) -> String {
        let Some(values) = block.fields.get(field).and_then(Value::as_array) else {
            return Expression::unknown(&format!("missing {field} field")).text;
        };
        let Some(name) = values.first().and_then(Value::as_str) else {
            return Expression::unknown(&format!("invalid {field} field")).text;
        };
        self.symbol_name(values.get(1).and_then(Value::as_str), name, list)
    }

    fn symbol_name(&self, id: Option<&str>, fallback: &str, list: bool) -> String {
        let contains = |target: &ScratchTarget, id: &str| {
            if list {
                target.lists.contains_key(id)
            } else {
                target.variables.contains_key(id)
            }
        };
        if let Some(id) = id {
            if contains(self.target, id) {
                if let Some(name) = self.names.get(id) {
                    return name.clone();
                }
            } else if self.stage.is_some_and(|stage| contains(stage, id)) {
                if let Some(name) = self.global_names.get(id) {
                    return if self.names.values().any(|local| local == name) {
                        format!("global.{name}")
                    } else {
                        name.clone()
                    };
                }
            }
        }
        identifier(fallback)
    }
}

fn field_value<'a>(block: &'a ScratchBlock, name: &str) -> Option<&'a Value> {
    block.fields.get(name)?.as_array()?.first()
}

fn field_literal(block: &ScratchBlock, name: &str) -> String {
    field_value(block, name)
        .map(scalar)
        .unwrap_or_else(|| Expression::unknown(&format!("missing {name} field")).text)
}

fn text_literal(value: &Value) -> Expression {
    match value {
        Value::String(text) => Expression::atom(quote(text)),
        Value::Number(_) | Value::Bool(_) => Expression::atom(quote(&value.to_string())),
        _ => Expression::unknown("invalid text literal"),
    }
}

fn identifier(name: &str) -> String {
    let mut result = String::new();
    for c in name.chars() {
        if c.is_alphanumeric() || c == '_' {
            result.push(c);
        } else if c.is_whitespace() {
            result.push('_');
        } else {
            result.push_str(&format!("_u{:x}_", c as u32));
        }
    }
    if result.is_empty() {
        result.push('_');
    }
    if result.starts_with(char::is_numeric) {
        result.insert(0, '_');
    }
    // Keep literal values and the global scope qualifier unambiguous.
    if matches!(result.as_str(), "true" | "false" | "global") {
        result.insert(0, '_');
    }
    result
}

fn allocate_names<'a>(
    entries: impl Iterator<Item = (&'a str, &'a str)>,
) -> HashMap<String, String> {
    let mut entries: Vec<_> = entries.collect();
    entries.sort_by(|(a_id, a), (b_id, b)| a.cmp(b).then(a_id.cmp(b_id)));
    let reserved: HashSet<_> = entries.iter().map(|(_, name)| identifier(name)).collect();
    let mut used = HashSet::new();
    entries
        .into_iter()
        .map(|(id, name)| {
            let base = identifier(name);
            let mut candidate = base.clone();
            let mut suffix = 2;
            while used.contains(&candidate) || (candidate != base && reserved.contains(&candidate))
            {
                candidate = format!("{base}__{suffix}");
                suffix += 1;
            }
            used.insert(candidate.clone());
            (id.to_owned(), candidate)
        })
        .collect()
}

fn number_literal(value: &Value) -> Expression {
    if let Value::Number(number) = value {
        return Expression::atom(number.to_string());
    }
    if let Some(text) = value.as_str() {
        let text = text.trim();
        // A blank numeric socket is zero in Scratch, not a missing input.
        if text.is_empty() {
            return Expression::atom("0".to_owned());
        }
        if let Ok(number) = serde_json::from_str::<serde_json::Number>(text) {
            return Expression::atom(number.to_string());
        }
        // Normalize spellings such as +5, 05 and .5 to valid numeric literals.
        if let Ok(number) = text.parse::<f64>() {
            if number.is_finite() {
                return Expression::atom(number.to_string());
            }
        }
    }
    Expression::unknown("invalid numeric literal")
}

fn scalar(value: &Value) -> String {
    match value {
        Value::String(_) | Value::Number(_) | Value::Bool(_) => value.to_string(),
        _ => Expression::unknown("unsupported variable value").text,
    }
}

fn quote(text: &str) -> String {
    Value::String(text.to_owned()).to_string()
}

fn line(output: &mut String, indent: usize, text: &str) {
    output.push_str(&"    ".repeat(indent));
    output.push_str(text);
    output.push('\n');
}

fn unknown_statement(output: &mut String, indent: usize, reason: &str) {
    line(output, indent, &format!("unknown {}", quote(reason)));
}

#[cfg(test)]
mod tests;
