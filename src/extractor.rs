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

pub fn read_project_json<P: AsRef<Path>>(sb3_path: P) -> Result<String, Box<dyn Error>> {
    let file = File::open(sb3_path)?;
    let mut archive = ZipArchive::new(file)?;
    let mut project_json = archive.by_name("project.json")?;
    let mut contents = String::new();
    io::Read::read_to_string(&mut project_json, &mut contents)?;
    Ok(contents)
}
