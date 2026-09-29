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
