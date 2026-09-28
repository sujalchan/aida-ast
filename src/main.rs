use std::{error::Error, fs};

mod extractor;
mod parser;

fn main() -> Result<(), Box<dyn Error>> {
    let sb3_input = "example.sb3";
    let json_output = "project.json";

    extractor::extract_project_json(sb3_input, json_output)?;
    println!("Extracted {json_output} from {sb3_input}");

    let json_str = fs::read_to_string(json_output)?;

    parser::print_targets(&json_str)?;

    Ok(())
}
