//! Guarded numeric locals for bounded, non-suspending direct procedures.
//!
//! The dynamic procedure remains authoritative for noncanonical inputs. All
//! writes are committed immediately; only reads within the call are cached.

use std::collections::{BTreeMap, HashMap, HashSet};
use serde_json::Value;
use crate::parser::{InputValue, ScratchBlock, ScratchTarget, parse_input};
use super::{
    expressions::expression,
    literals::{field_value, line, number_literal},
    project::{ListInfo, VariableInfo},
    scripts::ProcedureSignature,
    statements::variable_field,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind { Number, Boolean }
struct NumericExpr { code: String, kind: Kind }

struct Compiler<'a> {
    target: &'a ScratchTarget,
    variables: &'a HashMap<String, VariableInfo>,
    arguments: HashMap<String, usize>,
    all: BTreeMap<usize, VariableInfo>,
    needed: BTreeMap<usize, VariableInfo>,
    statements: usize,
    has_motion: bool,
}

impl<'a> Compiler<'a> {
    fn read_variable(&mut self, variable: VariableInfo, written: &HashSet<usize>) -> NumericExpr {
        self.all.insert(variable.slot, variable.clone());
        if !written.contains(&variable.slot) { self.needed.insert(variable.slot, variable.clone()); }
        NumericExpr { code: format!("numeric_{}", variable.slot), kind: Kind::Number }
    }

    fn expr(&mut self, input: Option<&Value>, written: &HashSet<usize>, active: &mut HashSet<String>) -> Option<NumericExpr> {
        match input.and_then(parse_input)? {
            InputValue::Number(value) => Some(NumericExpr { code: number_literal(value), kind: Kind::Number }),
            InputValue::String(value) => {
                let text = value.as_str()?;
                let n = text.parse::<i32>().ok()?;
                if n.unsigned_abs() > 1_000_000 || n.to_string() != text { return None; }
                Some(NumericExpr { code: format!("{n}.0_f64"), kind: Kind::Number })
            }
            InputValue::VariableReference(reference) => {
                let variable = self.variables.get(reference.id)?.clone();
                Some(self.read_variable(variable, written))
            }
            InputValue::BlockReference(id) => {
                if !active.insert(id.to_owned()) { return None; }
                let block = self.target.blocks.get(id)?;
                let result = self.block_expr(block, written, active);
                active.remove(id);
                result
            }
            _ => None,
        }
    }

    fn block_expr(&mut self, block: &ScratchBlock, written: &HashSet<usize>, active: &mut HashSet<String>) -> Option<NumericExpr> {
        let opcode = block.opcode.as_str();
        if opcode == "data_variable" {
            return Some(self.read_variable(variable_field(block, "VARIABLE", self.variables)?, written));
        }
        if matches!(opcode, "argument_reporter_string_number" | "argument_reporter_boolean") {
            let name = field_value(block, "VALUE")?.as_str()?;
            return Some(NumericExpr { code: format!("numeric_argument_{}", self.arguments.get(name)?), kind: Kind::Number });
        }
        if opcode == "operator_not" {
            let input = self.expr(block.inputs.get("OPERAND"), written, active)?;
            return Some(NumericExpr { code: format!("!({})", boolean(input)), kind: Kind::Boolean });
        }
        let (left, right) = if matches!(opcode, "operator_add" | "operator_subtract" | "operator_multiply" | "operator_divide") { ("NUM1", "NUM2") } else { ("OPERAND1", "OPERAND2") };
        let left = self.expr(block.inputs.get(left), written, active)?;
        let right = self.expr(block.inputs.get(right), written, active)?;
        if matches!(opcode, "operator_and" | "operator_or") {
            let op = if opcode == "operator_and" { "&&" } else { "||" };
            return Some(NumericExpr { code: format!("({}) {op} ({})", boolean(left), boolean(right)), kind: Kind::Boolean });
        }
        if left.kind != Kind::Number || right.kind != Kind::Number { return None; }
        if let Some(op) = match opcode { "operator_add" => Some("+"), "operator_subtract" => Some("-"), "operator_multiply" => Some("*"), "operator_divide" => Some("/"), _ => None } {
            return Some(NumericExpr { code: format!("(scratch_number_operand({}) {op} scratch_number_operand({}))", left.code, right.code), kind: Kind::Number });
        }
        let (op, fallback) = match opcode { "operator_equals" => ("==", "scratch_equals"), "operator_gt" => (">", "scratch_greater_than"), "operator_lt" => ("<", "scratch_less_than"), _ => return None };
        // Scratch falls back to text comparison for NaN. Canonical guards make
        // the numeric value's text identical to the source value's text.
        Some(NumericExpr { code: format!("{{ let left = {}; let right = {}; if left.is_nan() || right.is_nan() {{ {fallback}(left, right) }} else {{ left {op} right }} }}", left.code, right.code), kind: Kind::Boolean })
    }

    fn chain(&mut self, mut id: Option<&'a str>, written: &mut HashSet<usize>, out: &mut String, indent: usize, seen: &mut HashSet<String>, args: &HashMap<String, String>, lists: &HashMap<String, ListInfo>) -> Option<()> {
        while let Some(current) = id {
            if !seen.insert(current.to_owned()) { return None; }
            let block = self.target.blocks.get(current)?;
            self.statements += 1;
            match block.opcode.as_str() {
                "control_if" | "control_if_else" => {
                    let condition = self.expr(block.inputs.get("CONDITION"), written, &mut HashSet::new())?;
                    line(out, indent, &format!("if {} {{", boolean(condition)));
                    let branch = |name| block.inputs.get(name).and_then(parse_input).and_then(|value| match value { InputValue::BlockReference(id) => Some(id), _ => None });
                    let mut yes = written.clone();
                    self.chain(branch("SUBSTACK"), &mut yes, out, indent + 1, seen, args, lists)?;
                    let mut no = written.clone();
                    if block.opcode == "control_if_else" {
                        line(out, indent, "} else {");
                        self.chain(branch("SUBSTACK2"), &mut no, out, indent + 1, seen, args, lists)?;
                    }
                    line(out, indent, "}");
                    *written = yes.intersection(&no).copied().collect();
                }
                "control_stop" if field_value(block, "STOP_OPTION").and_then(Value::as_str) == Some("this script") => line(out, indent, "return;"),
                "data_setvariableto" => {
                    let variable = variable_field(block, "VARIABLE", self.variables)?;
                    let input = block.inputs.get("VALUE");
                    let value = self.expr(input, written, &mut HashSet::new())?;
                    if value.kind != Kind::Number { return None; }
                    self.all.insert(variable.slot, variable.clone());
                    let direct_copy = match input.and_then(parse_input)? {
                        InputValue::String(_) | InputValue::VariableReference(_) => true,
                        InputValue::BlockReference(id) => self.target.blocks.get(id).is_some_and(|b| matches!(b.opcode.as_str(), "data_variable" | "argument_reporter_string_number" | "argument_reporter_boolean")),
                        _ => false,
                    };
                    line(out, indent, &format!("let numeric_new = {};", value.code));
                    if direct_copy {
                        let source = expression(self.target, input, self.variables, lists, args, &mut HashSet::new());
                        line(out, indent, &format!("let scratch_new_value: ScratchValue = ({source}).to_owned().into();"));
                    } else {
                        line(out, indent, "let scratch_new_value = ScratchValue::Number(numeric_new);");
                    }
                    let setter = if variable.is_stage { "set_shared" } else { "set_local_value" };
                    line(out, indent, &format!("state.{setter}({}, scratch_new_value);", variable.slot));
                    line(out, indent, &format!("numeric_{} = numeric_new;", variable.slot));
                    written.insert(variable.slot);
                }
                "motion_gotoxy" => {
                    let x = self.expr(block.inputs.get("X"), written, &mut HashSet::new())?;
                    let y = self.expr(block.inputs.get("Y"), written, &mut HashSet::new())?;
                    if x.kind != Kind::Number || y.kind != Kind::Number { return None; }
                    line(out, indent, &format!("state.set_xy(scratch_number_operand({}), scratch_number_operand({}));", x.code, y.code));
                    self.has_motion = true;
                }
                "pen_penDown" => { line(out, indent, "state.pen_down();"); self.has_motion = true; }
                "pen_penUp" => line(out, indent, "state.pen_up();"),
                _ => return None,
            }
            id = block.next.as_deref();
        }
        Some(())
    }
}

fn boolean(expr: NumericExpr) -> String {
    match expr.kind { Kind::Boolean => expr.code, Kind::Number => format!("scratch_bool({})", expr.code) }
}

pub(super) fn generate_numeric_direct(target: &ScratchTarget, procedure: &ProcedureSignature, variables: &HashMap<String, VariableInfo>, lists: &HashMap<String, ListInfo>, name: &str, numeric_args_only: bool) -> Option<String> {
    let args = procedure.argument_names.iter().enumerate().map(|(index, name)| (name.clone(), index)).collect();
    let argument_values = procedure.argument_names.iter().enumerate().map(|(index, name)| (name.clone(), if numeric_args_only { format!("ScratchValue::Number(numeric_argument_{index})") } else { format!("argument_{index}") })).collect::<HashMap<_, _>>();
    let mut compiler = Compiler { target, variables, arguments: args, all: BTreeMap::new(), needed: BTreeMap::new(), statements: 0, has_motion: false };
    let definition = target.blocks.get(&procedure.definition_id)?;
    let mut body = String::new();
    compiler.chain(definition.next.as_deref(), &mut HashSet::new(), &mut body, 1, &mut HashSet::new(), &argument_values, lists)?;
    // Avoid creating extra versions of tiny procedures and unrelated code.
    if compiler.statements < 12 || !compiler.has_motion { return None; }
    let params = (0..procedure.argument_ids.len()).map(|i| if numeric_args_only { format!(", numeric_argument_{i}: f64") } else { format!(", argument_{i}: ScratchValue") }).collect::<String>();
    let fallback_args = (0..procedure.argument_ids.len()).map(|i| if numeric_args_only { format!(", ScratchValue::Number(numeric_argument_{i})") } else { format!(", argument_{i}") }).collect::<String>();
    let fallback = format!("{name}_dynamic(state, clock{fallback_args}); return;");
    let mut output = String::new();
    line(&mut output, 0, "#[allow(dead_code, unused_variables, unused_mut, unused_assignments, unreachable_code, unused_parens)]");
    let function = if numeric_args_only { format!("{name}_numbers") } else { name.to_owned() };
    line(&mut output, 0, &format!("fn {function}(state: &mut ScratchState, clock: &dyn Clock{params}) {{"));
    if !numeric_args_only {
        for i in 0..procedure.argument_ids.len() {
            line(&mut output, 1, &format!("let Some(numeric_argument_{i}) = argument_{i}.canonical_number() else {{ {fallback} }};"));
        }
    }
    for (slot, variable) in &compiler.all {
        if compiler.needed.contains_key(slot) {
            line(&mut output, 1, &format!("let Some(mut numeric_{slot}) = {}.canonical_number() else {{ {fallback} }};", variable.read_expression()));
        } else {
            line(&mut output, 1, &format!("let mut numeric_{slot} = 0.0_f64;"));
        }
    }
    output.push_str(&body);
    line(&mut output, 0, "}\n");
    Some(output)
}
