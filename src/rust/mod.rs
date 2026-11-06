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

mod direct_numeric;
mod expressions;
mod literals;
mod project;
mod scripts;
mod statements;

#[cfg(not(test))]
pub use project::generate_with_list_data;
#[cfg(test)]
pub use project::{generate, generate_with_list_data, generate_with_options};

#[cfg(test)]
mod tests;
