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

use serde_json::Value;
use std::{error::Error, fs::File, io, path::Path};
use zip::ZipArchive;

pub fn read_project_json<P: AsRef<Path>>(sb3_path: P) -> Result<String, Box<dyn Error>> {
    let file = File::open(sb3_path)?;
    let mut archive = ZipArchive::new(file)?;
    let mut project_json = archive.by_name("project.json")?;
    let mut contents = String::new();
    io::Read::read_to_string(&mut project_json, &mut contents)?;
    drop(project_json);

    let mut project: Value = serde_json::from_str(&contents)?;
    if let Some(targets) = project.get_mut("targets").and_then(Value::as_array_mut) {
        for costume in targets
            .iter_mut()
            .filter_map(|target| target.get_mut("costumes").and_then(Value::as_array_mut))
            .flatten()
        {
            let Some(path) = costume_archive_path(costume) else {
                continue;
            };
            let Ok(mut asset) = archive.by_name(&path) else {
                continue;
            };
            let mut bytes = Vec::new();
            if io::Read::read_to_end(&mut asset, &mut bytes).is_err() {
                continue;
            }
            let data_format = costume
                .get("dataFormat")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let bitmap_resolution = costume
                .get("bitmapResolution")
                .and_then(Value::as_f64)
                .unwrap_or(1.0);
            if let Some(size) = logical_costume_size(&bytes, data_format, bitmap_resolution) {
                *costume
                    .as_object_mut()
                    .expect("costumes in project.json are objects")
                    .entry("aidaLogicalSize")
                    .or_insert_with(|| serde_json::json!(size)) = serde_json::json!(size);
            }
        }
    }
    Ok(serde_json::to_string(&project)?)
}

fn costume_archive_path(costume: &Value) -> Option<String> {
    costume
        .get("md5ext")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .or_else(|| {
            let asset_id = costume.get("assetId")?.as_str()?;
            let data_format = costume.get("dataFormat")?.as_str()?;
            (!asset_id.is_empty() && !data_format.is_empty())
                .then(|| format!("{asset_id}.{data_format}"))
        })
}

fn logical_costume_size(
    bytes: &[u8],
    data_format: &str,
    bitmap_resolution: f64,
) -> Option<[f64; 2]> {
    if data_format.eq_ignore_ascii_case("svg")
        || bytes.starts_with(b"<svg")
        || bytes.starts_with(b"<?xml")
    {
        svg_view_box_size(bytes)
    } else {
        let (width, height) = bitmap_dimensions(bytes)?;
        let resolution = if bitmap_resolution == 0.0 {
            2.0
        } else {
            bitmap_resolution
        };
        if !resolution.is_finite() || resolution < 0.0 {
            return None;
        }
        Some([width / resolution, height / resolution])
    }
}

fn svg_view_box_size(bytes: &[u8]) -> Option<[f64; 2]> {
    let text = std::str::from_utf8(bytes).ok()?;
    let document = roxmltree::Document::parse(text).ok()?;
    let root = document.root_element();
    if root.tag_name().name() != "svg" {
        return None;
    }
    let view_box = root
        .attribute("viewBox")
        .or_else(|| root.attribute("viewbox"));
    let size = if let Some(view_box) = view_box {
        let values = view_box
            .split(|character: char| character.is_ascii_whitespace() || character == ',')
            .filter(|value| !value.is_empty())
            .map(str::parse::<f64>)
            .collect::<Result<Vec<_>, _>>()
            .ok()?;
        [*values.get(2)?, *values.get(3)?]
    } else {
        // SVGDOM's empty viewBox has zero size. Scratch's SVG loader normalizes
        // ordinary assets before SVGSkin reads it; width/height are the useful
        // fallback for valid SVG files which omit an explicit viewBox.
        [
            svg_length(root.attribute("width")?)?,
            svg_length(root.attribute("height")?)?,
        ]
    };
    (size[0].is_finite() && size[1].is_finite() && size[0] >= 0.0 && size[1] >= 0.0).then_some(size)
}

fn svg_length(value: &str) -> Option<f64> {
    let value = value.trim();
    let numeric_end = value
        .find(|character: char| {
            !(character.is_ascii_digit() || matches!(character, '+' | '-' | '.' | 'e' | 'E'))
        })
        .unwrap_or(value.len());
    let number = value.get(..numeric_end)?.parse::<f64>().ok()?;
    (number.is_finite() && number >= 0.0).then_some(number)
}

fn bitmap_dimensions(bytes: &[u8]) -> Option<(f64, f64)> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") && bytes.len() >= 24 {
        let width = u32::from_be_bytes(bytes.get(16..20)?.try_into().ok()?);
        let height = u32::from_be_bytes(bytes.get(20..24)?.try_into().ok()?);
        return Some((width as f64, height as f64));
    }
    jpeg_dimensions(bytes)
}

fn jpeg_dimensions(bytes: &[u8]) -> Option<(f64, f64)> {
    if !bytes.starts_with(&[0xff, 0xd8]) {
        return None;
    }
    let mut cursor = 2;
    while cursor < bytes.len() {
        while bytes.get(cursor) == Some(&0xff) {
            cursor += 1;
        }
        let marker = *bytes.get(cursor)?;
        cursor += 1;
        if marker == 0xd9 || marker == 0xda {
            return None;
        }
        if marker == 0x01 || (0xd0..=0xd7).contains(&marker) {
            continue;
        }
        let segment_length =
            u16::from_be_bytes(bytes.get(cursor..cursor + 2)?.try_into().ok()?) as usize;
        if segment_length < 2 {
            return None;
        }
        if matches!(marker, 0xc0..=0xc3 | 0xc5..=0xc7 | 0xc9..=0xcb | 0xcd..=0xcf) {
            let height = u16::from_be_bytes(bytes.get(cursor + 3..cursor + 5)?.try_into().ok()?);
            let width = u16::from_be_bytes(bytes.get(cursor + 5..cursor + 7)?.try_into().ok()?);
            return Some((width as f64, height as f64));
        }
        cursor = cursor.checked_add(segment_length)?;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::logical_costume_size;

    #[test]
    fn svg_dimensions_follow_view_box_instead_of_intrinsic_or_painted_size() {
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" width="600" height="300" viewBox="5, 7, 61.5, 117.75"><rect x="20" y="20" width="3" height="4"/></svg>"#;
        assert_eq!(logical_costume_size(svg, "svg", 1.0), Some([61.5, 117.75]));
    }

    #[test]
    fn bitmap_dimensions_are_divided_by_costume_resolution() {
        let mut png = vec![0; 24];
        png[..8].copy_from_slice(b"\x89PNG\r\n\x1a\n");
        png[16..20].copy_from_slice(&200_u32.to_be_bytes());
        png[20..24].copy_from_slice(&100_u32.to_be_bytes());
        assert_eq!(logical_costume_size(&png, "png", 2.0), Some([100.0, 50.0]));
        assert_eq!(logical_costume_size(&png, "png", 1.0), Some([200.0, 100.0]));
    }
}
