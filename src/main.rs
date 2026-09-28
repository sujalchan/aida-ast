use std::error::Error;

mod extractor;

fn main() -> Result<(), Box<dyn Error>> {
    let sb3_input = "example.sb3";
    let json_output = "project.json";

    extractor::extract_project_json(sb3_input, json_output)?;

    println!("Successfully extracted {json_output} from {sb3_input}");
    Ok(())
}
