use super::*;

pub(super) fn rust_identifiers(source: &str) -> HashSet<String> {
    let characters = source.chars().collect::<Vec<_>>();
    let mut identifiers = HashSet::new();
    let mut index = 0;
    while index < characters.len() {
        if characters[index] == '"' {
            index += 1;
            while index < characters.len() {
                if characters[index] == '\\' {
                    index += 2;
                } else if characters[index] == '"' {
                    index += 1;
                    break;
                } else {
                    index += 1;
                }
            }
            continue;
        }
        if characters[index] == '/' && characters.get(index + 1) == Some(&'/') {
            index += 2;
            while index < characters.len() && characters[index] != '\n' {
                index += 1;
            }
            continue;
        }
        if characters[index] == '/' && characters.get(index + 1) == Some(&'*') {
            index += 2;
            while index + 1 < characters.len()
                && !(characters[index] == '*' && characters[index + 1] == '/')
            {
                index += 1;
            }
            index = (index + 2).min(characters.len());
            continue;
        }
        if characters[index] == '_' || characters[index].is_ascii_alphabetic() {
            let start = index;
            index += 1;
            while index < characters.len()
                && (characters[index] == '_' || characters[index].is_ascii_alphanumeric())
            {
                index += 1;
            }
            identifiers.insert(characters[start..index].iter().collect());
            continue;
        }
        index += 1;
    }
    identifiers
}

// assign each Scratch variable ID a stable shared-state slot.
pub(super) fn allocate_variables(project: &ScratchProject) -> HashMap<String, VariableInfo> {
    let numeric_variables = numeric_variables(project);
    let non_nan_variables = non_nan_variables(project);
    let mut entries = BTreeMap::new();
    for target in &project.targets {
        for (id, variable) in &target.variables {
            let info = entries.entry(id.clone()).or_insert_with(|| VariableInfo {
                slot: 0,
                name: variable.name.clone(),
                target: target.name.clone(),
                is_stage: target.is_stage,
                target_index: project
                    .targets
                    .iter()
                    .position(|candidate| {
                        candidate.name == target.name && candidate.is_stage == target.is_stage
                    })
                    .unwrap_or(0),
                numeric: numeric_variables.contains(id),
                non_nan: non_nan_variables.contains(id),
                monitor_visible: project
                    .monitors
                    .iter()
                    .any(|monitor| monitor.visible && monitor.id == *id),
                index_read_override: None,
            });
            if target.is_stage {
                info.name = variable.name.clone();
                info.target = target.name.clone();
                info.is_stage = true;
                info.target_index = project
                    .targets
                    .iter()
                    .position(|candidate| candidate.is_stage)
                    .unwrap_or(0);
            }
        }
    }
    let mut output = HashMap::new();
    for (slot, (id, mut info)) in entries.into_iter().enumerate() {
        info.slot = slot;
        output.insert(id, info);
    }
    output
}

// prove numeric variables non-nan when every write preserves that property.
pub(super) fn non_nan_variables(project: &ScratchProject) -> HashSet<String> {
    let mut non_nan = project
        .targets
        .iter()
        .flat_map(|target| &target.variables)
        .filter(|(_, variable)| variable.value.as_f64().is_some_and(|value| !value.is_nan()))
        .map(|(id, _)| id.clone())
        .collect::<HashSet<_>>();

    loop {
        let previous = non_nan.clone();
        for target in &project.targets {
            for block in target.blocks.values() {
                if !matches!(
                    block.opcode.as_str(),
                    "data_setvariableto" | "data_changevariableby"
                ) {
                    continue;
                }
                let Some(id) = block
                    .fields
                    .get("VARIABLE")
                    .and_then(|field| field.as_array())
                    .and_then(|field| field.get(1))
                    .and_then(|id| id.as_str())
                else {
                    continue;
                };
                let preserves_non_nan = if block.opcode == "data_setvariableto" {
                    input_is_non_nan_number(
                        target,
                        block.inputs.get("VALUE"),
                        &previous,
                        &mut HashSet::new(),
                    )
                } else {
                    constant_numeric_input(target, block.inputs.get("VALUE"), &mut HashSet::new())
                        .is_some_and(f64::is_finite)
                };
                if !preserves_non_nan {
                    non_nan.remove(id);
                }
            }
        }
        if non_nan == previous {
            return non_nan;
        }
    }
}

pub(super) fn input_is_non_nan_number(
    target: &ScratchTarget,
    input: Option<&serde_json::Value>,
    non_nan: &HashSet<String>,
    active: &mut HashSet<String>,
) -> bool {
    match input.and_then(parse_input) {
        Some(InputValue::Number(value)) => value
            .as_f64()
            .or_else(|| value.as_str().and_then(|text| text.parse().ok()))
            .is_some_and(|value: f64| !value.is_nan()),
        Some(InputValue::VariableReference(reference)) => non_nan.contains(reference.id),
        Some(InputValue::BlockReference(id)) => {
            if !active.insert(id.to_owned()) {
                return false;
            }
            let result = target.blocks.get(id).is_some_and(|block| {
                if block.opcode == "data_variable" {
                    return block
                        .fields
                        .get("VARIABLE")
                        .and_then(|field| field.as_array())
                        .and_then(|field| field.get(1))
                        .and_then(|id| id.as_str())
                        .is_some_and(|id| non_nan.contains(id));
                }
                matches!(
                    block.opcode.as_str(),
                    "math_number"
                        | "math_integer"
                        | "math_whole_number"
                        | "math_positive_number"
                        | "math_angle"
                ) && block
                    .fields
                    .get("NUM")
                    .and_then(|field| field.as_array())
                    .and_then(|field| field.first())
                    .and_then(|value| {
                        value
                            .as_f64()
                            .or_else(|| value.as_str().and_then(|text| text.parse().ok()))
                    })
                    .is_some_and(|value: f64| !value.is_nan())
            });
            active.remove(id);
            result
        }
        _ => false,
    }
}

// prove variables numeric only when their saved value and every assignment are numeric.
pub(super) fn numeric_variables(project: &ScratchProject) -> HashSet<String> {
    let mut numeric = project
        .targets
        .iter()
        .flat_map(|target| &target.variables)
        .filter(|(_, variable)| variable.value.is_number() && !variable.is_cloud)
        .map(|(id, _)| id.clone())
        .collect::<HashSet<_>>();

    loop {
        let previous = numeric.clone();
        for target in &project.targets {
            for block in target.blocks.values() {
                if block.opcode != "data_setvariableto" {
                    continue;
                }
                let Some(id) = block
                    .fields
                    .get("VARIABLE")
                    .and_then(|field| field.as_array())
                    .and_then(|field| field.get(1))
                    .and_then(|id| id.as_str())
                else {
                    continue;
                };
                if !input_is_numeric(
                    target,
                    block.inputs.get("VALUE"),
                    &previous,
                    &mut HashSet::new(),
                ) {
                    numeric.remove(id);
                }
            }
        }
        if numeric == previous {
            return numeric;
        }
    }
}

pub(super) fn input_is_numeric(
    target: &ScratchTarget,
    input: Option<&serde_json::Value>,
    numeric: &HashSet<String>,
    active: &mut HashSet<String>,
) -> bool {
    match input.and_then(parse_input) {
        Some(InputValue::Number(_)) => true,
        Some(InputValue::VariableReference(reference)) => numeric.contains(reference.id),
        Some(InputValue::BlockReference(id)) => {
            if !active.insert(id.to_owned()) {
                return false;
            }
            let result = target
                .blocks
                .get(id)
                .is_some_and(|block| match block.opcode.as_str() {
                    "operator_add" | "operator_subtract" | "operator_multiply"
                    | "operator_divide" | "operator_mod" | "operator_round" | "operator_random"
                    | "operator_mathop" | "data_lengthoflist" | "data_itemnumoflist" => true,
                    "data_variable" => block
                        .fields
                        .get("VARIABLE")
                        .and_then(|field| field.as_array())
                        .and_then(|field| field.get(1))
                        .and_then(|id| id.as_str())
                        .is_some_and(|id| numeric.contains(id)),
                    "math_number"
                    | "math_integer"
                    | "math_whole_number"
                    | "math_positive_number"
                    | "math_angle" => true,
                    _ => false,
                });
            active.remove(id);
            result
        }
        _ => false,
    }
}

// assign each scratch list id a stable shared-state slot.
pub(super) fn allocate_lists(project: &ScratchProject) -> HashMap<String, ListInfo> {
    let written_lists = project
        .targets
        .iter()
        .flat_map(|target| target.blocks.values())
        .filter(|block| {
            matches!(
                block.opcode.as_str(),
                "data_addtolist"
                    | "data_deleteoflist"
                    | "data_deletealloflist"
                    | "data_insertatlist"
                    | "data_replaceitemoflist"
            )
        })
        .filter_map(|block| block.fields.get("LIST")?.as_array()?.get(1)?.as_str())
        .collect::<HashSet<_>>();
    let mut entries = BTreeMap::new();
    for target in &project.targets {
        for (id, list) in &target.lists {
            let info = entries.entry(id.clone()).or_insert_with(|| ListInfo {
                slot: 0,
                name: list.name.clone(),
                target: target.name.clone(),
                is_stage: target.is_stage,
                target_index: project
                    .targets
                    .iter()
                    .position(|candidate| {
                        candidate.name == target.name && candidate.is_stage == target.is_stage
                    })
                    .unwrap_or(0),
                cache_numeric_reads: list.items.len() >= 1024
                    && !written_lists.contains(id.as_str()),
            });
            if target.is_stage {
                info.name = list.name.clone();
                info.target = target.name.clone();
                info.is_stage = true;
                info.target_index = project
                    .targets
                    .iter()
                    .position(|candidate| candidate.is_stage)
                    .unwrap_or(0);
            }
        }
    }
    let mut output = HashMap::new();
    for (slot, (id, mut info)) in entries.into_iter().enumerate() {
        info.slot = slot;
        output.insert(id, info);
    }
    output
}

// convert a Scratch variable name into a valid Rust identifier.
pub(in crate::rust) fn rust_identifier(name: &str) -> String {
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
pub(super) fn is_keyword(name: &str) -> bool {
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
