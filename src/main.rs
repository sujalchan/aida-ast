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

use std::{error::Error, fs};

mod aida;
mod extractor;
mod parser;

fn main() -> Result<(), Box<dyn Error>> {
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
