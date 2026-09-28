use serde::Deserialize;
use serde_json;
use std::error::Error;

#[derive(Debug, Deserialize)]
pub struct ScratchProject {
    pub targets: Vec<ScratchTarget>,
}

#[derive(Debug, Deserialize)]
pub struct ScratchTarget {
    pub name: String,
    #[serde(rename = "isStage")]
    pub is_stage: bool,
}

// Print the targets found
pub fn print_targets(json_str: &str) -> Result<(), Box<dyn Error>> {
    let project: ScratchProject = serde_json::from_str(json_str)?;

    let total_targets = project.targets.len();

    // Get sprite count by filtering out the Stage
    let sprite_count = project.targets.iter().filter(|t| !t.is_stage).count();
    let stage_count = total_targets - sprite_count;

    println!("Total targets: {total_targets} ({stage_count} Stage, {sprite_count} Sprites)");

    for target in &project.targets {
        if target.is_stage {
            println!(" - Stage: {}", target.name);
        } else {
            println!(" - Sprite: {}", target.name);
        }
    }
    Ok(())
}
