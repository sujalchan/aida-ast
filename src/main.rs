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

use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
};

mod aida;
mod extractor;
mod parser;

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("--tests") if args.next().is_none() => return compile_tests(),
        Some(argument) => {
            return Err(format!("unknown argument: {argument}\nusage: aida-ast [--tests]").into());
        }
        None => {}
    }

    let sb3_input = "example.sb3";
    let json_output = "project.json";
    let aida_output = "project.aida";

    extractor::extract_project_json(sb3_input, json_output)?;
    println!("Extracted {json_output} from {sb3_input}");

    let json_str = fs::read_to_string(json_output)?;

    let project = parser::parse_project(&json_str)?;
    parser::print_project(&project);

    fs::write(aida_output, aida::generate(&project))?;
    println!("Generated {aida_output}");

    Ok(())
}

fn compile_tests() -> Result<(), Box<dyn Error>> {
    let source_dir = Path::new("tests/src");
    let output_dir = Path::new("tests/output");
    fs::create_dir_all(output_dir)?;

    let mut inputs: Vec<PathBuf> = fs::read_dir(source_dir)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<_, _>>()?;
    inputs.retain(|path| {
        path.is_file()
            && path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("sb3"))
    });
    inputs.sort();

    for input in &inputs {
        let json = extractor::read_project_json(input)?;
        let project = parser::parse_project(&json)?;
        let output_name = input
            .file_stem()
            .ok_or("test project has no filename")?
            .to_string_lossy();
        let output = output_dir.join(format!("{output_name}.rs"));
        fs::write(&output, aida::generate(&project))?;
        println!("Compiled {} -> {}", input.display(), output.display());
    }

    println!("Compiled {} test project(s)", inputs.len());
    Ok(())
}
