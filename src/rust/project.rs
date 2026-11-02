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

//! builds the generated Rust module and initializes its Scratch variables.
//!
//! allocates stable Rust identifiers for project variables, writes saved
//! variable values, then delegates each target's scripts to the script module.

use std::collections::{HashMap, HashSet};

use crate::parser::ScratchProject;

use super::{
    literals::{line, literal},
    scripts::{generate_procedures, generate_scripts},
};

/// generate the module header, saved variable declarations, and target scripts.
pub fn generate(project: &ScratchProject) -> String {
    let variables = allocate_variables(project);
    let mut output = String::from(
        "#[allow(unused_imports)]\nuse crate::runtime::{\n    scratch_acos_degrees, scratch_add, scratch_and, scratch_asin_degrees, scratch_atan_degrees,\n    scratch_bool, scratch_contains, scratch_cos, scratch_divide, scratch_equals,\n    scratch_greater_than, scratch_join, scratch_length, scratch_less_than, scratch_letter,\n    scratch_mod, scratch_multiply, scratch_not, scratch_or, scratch_random, scratch_round,\n    scratch_sin, scratch_subtract, scratch_tan, scratch_to_number, ScratchValue,\n};\n\n#[allow(unused_macros)]\nmacro_rules! print_value {\n    ($name:expr, $value:expr) => {\n        println!(\"{}: {:?}\", $name, $value);\n    };\n}\n\n",
    );

    for target in &project.targets {
        generate_procedures(target, &variables, &mut output);
    }

    output.push_str("#[allow(unused_mut, unused_variables, unused_assignments)]\npub fn run() {\n");

    for target in &project.targets {
        let mut declarations: Vec<_> = target.variables.iter().collect();
        declarations.sort_by(|(left_id, left), (right_id, right)| {
            left.name.cmp(&right.name).then(left_id.cmp(right_id))
        });
        for (id, variable) in declarations {
            if let Some(name) = variables.get(id) {
                line(
                    &mut output,
                    1,
                    &format!(
                        "let mut {name}: ScratchValue = ({}).into();",
                        literal(&variable.value)
                    ),
                );
            }
        }
    }

    for target in &project.targets {
        generate_scripts(target, &variables, &mut output);
    }

    output.push_str("}\n");
    output
}

// assign each Scratch variable ID a unique, stable Rust identifier.
fn allocate_variables(project: &ScratchProject) -> HashMap<String, String> {
    let mut entries: Vec<_> = project
        .targets
        .iter()
        .flat_map(|target| {
            target
                .variables
                .iter()
                .map(|(id, variable)| (id.as_str(), variable.name.as_str()))
        })
        .collect();
    entries.sort_unstable();
    let mut used = HashSet::new();
    let mut output = HashMap::new();
    for (id, name) in entries {
        let base = rust_identifier(name);
        let mut candidate = base.clone();
        let mut suffix = 2;
        while !used.insert(candidate.clone()) {
            candidate = format!("{base}_{suffix}");
            suffix += 1;
        }
        output.insert(id.to_owned(), candidate);
    }
    output
}

// convert a Scratch variable name into a valid Rust identifier.
pub(super) fn rust_identifier(name: &str) -> String {
    let mut output = String::new();
    for (index, character) in name.chars().enumerate() {
        if character.is_ascii_alphanumeric() || character == '_' {
            if index == 0 && character.is_ascii_digit() {
                output.push('_');
            }
            output.push(character);
        } else if !output.ends_with('_') {
            output.push('_');
        }
    }
    let output = output.trim_matches('_');
    let output = if output.is_empty() {
        "variable"
    } else {
        output
    };
    let output = if output.as_bytes()[0].is_ascii_digit() {
        format!("_{output}")
    } else {
        output.to_owned()
    };
    if is_keyword(&output) {
        format!("_{output}")
    } else {
        output
    }
}

// identify Rust keywords that need a prefix before they can be used as variables.
fn is_keyword(name: &str) -> bool {
    matches!(
        name,
        "as" | "break"
            | "const"
            | "continue"
            | "crate"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "Self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
            | "async"
            | "await"
            | "dyn"
    )
}
