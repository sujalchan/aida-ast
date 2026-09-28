use std::{error::Error, fs::File, io, path::Path};
use zip::ZipArchive;

// Extracts project.json given a .sb3 path and output path
pub fn extract_project_json<P: AsRef<Path>>(
    sb3_path: P,
    output_path: P,
) -> Result<(), Box<dyn Error>> {
    let file = File::open(sb3_path)?;
    let mut archive = ZipArchive::new(file)?;

    let mut extracted_json = archive.by_name("project.json")?;
    let mut output_file = File::create(output_path)?;

    io::copy(&mut extracted_json, &mut output_file)?;

    Ok(())
}
