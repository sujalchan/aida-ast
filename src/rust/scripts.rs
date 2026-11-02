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

//! finds supported Scratch event scripts and sends their bodies to statement lowering.
//!
//! sorting hat block IDs keeps script output stable when the project is generated
//! more than once.

use std::collections::HashMap;

use crate::parser::ScratchTarget;

use super::statements::generate_statements;

// lower each supported top-level event hat in stable block-ID order.
pub(super) fn generate_scripts(
    target: &ScratchTarget,
    variables: &HashMap<String, String>,
    output: &mut String,
) {
    let mut hats: Vec<_> = target
        .blocks
        .iter()
        .filter(|(_, block)| {
            block.top_level
                && matches!(
                    block.opcode.as_str(),
                    "event_whenflagclicked"
                        | "event_whenthisspriteclicked"
                        | "event_whenstageclicked"
                        | "event_whenkeypressed"
                        | "event_whenbroadcastreceived"
                )
        })
        .collect();
    hats.sort_by(|(left_id, _), (right_id, _)| left_id.cmp(right_id));
    for (_, hat) in hats {
        generate_statements(target, hat.next.as_deref(), variables, output);
    }
}
