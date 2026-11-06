use serde_json::json;

use super::{generate, generate_with_list_data, generate_with_options};
use crate::parser::{ScratchBlock, parse_project};

mod control_flow;
mod data;
mod events;
mod expressions;
mod generation;
mod procedures;
mod rendering;
