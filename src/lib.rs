pub mod extractor;
pub mod parser;

use std::{
    error::Error,
    fs,
    io::Read,
    path::{Component, Path, PathBuf},
};

use parser::{ScratchProject, parse_project};
use zip::ZipArchive;

pub struct LoadedProject {
    pub project: ScratchProject,
    pub assets_dir: PathBuf,
}

pub fn load_sb3(
    path: impl AsRef<Path>,
    assets_root: impl AsRef<Path>,
) -> Result<LoadedProject, Box<dyn Error>> {
    let path = path.as_ref();
    let name = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("project");
    let project_name: String = name
        .chars()
        .map(|value| {
            if value.is_ascii_alphanumeric() || "-_".contains(value) {
                value
            } else {
                '_'
            }
        })
        .collect();
    let assets_dir = assets_root.as_ref().join(if project_name.is_empty() {
        "project"
    } else {
        &project_name
    });
    fs::create_dir_all(&assets_dir)?;
    let file = fs::File::open(path)?;
    let mut archive = ZipArchive::new(file)?;
    let mut json = String::new();
    archive.by_name("project.json")?.read_to_string(&mut json)?;
    let project = parse_project(&json)?;
    for target in &project.targets {
        for asset in target.costumes.iter().chain(&target.sounds) {
            let filename = Path::new(&asset.md5ext);
            if filename.components().count() != 1
                || filename
                    .components()
                    .any(|part| !matches!(part, Component::Normal(_)))
            {
                return Err(format!("invalid SB3 asset path: {}", asset.md5ext).into());
            }
            let destination = assets_dir.join(filename);
            if !destination.exists() {
                let mut source = archive.by_name(&asset.md5ext)?;
                let mut bytes = Vec::new();
                source.read_to_end(&mut bytes)?;
                fs::write(destination, bytes)?;
            }
        }
    }
    Ok(LoadedProject {
        project,
        assets_dir,
    })
}
