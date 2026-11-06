use super::*;

pub(super) fn reporter_monitor_values(
    project: &ScratchProject,
    variables: &HashMap<String, VariableInfo>,
    lists: &HashMap<String, ListInfo>,
) -> Vec<(String, String, Option<usize>)> {
    project
        .monitors
        .iter()
        .filter_map(|monitor| {
            let monitor_target = project.targets.iter().enumerate().find(|(_, target)| {
                monitor
                    .sprite_name
                    .as_deref()
                    .map_or(target.is_stage, |sprite| target.name == sprite)
            });
            let mut target_context = None;
            let expression = match monitor.opcode.as_str() {
                "sensing_current" => {
                    let method = match monitor
                        .params
                        .get("CURRENTMENU")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_ascii_uppercase()
                        .as_str()
                    {
                        "YEAR" => "current_year",
                        "MONTH" => "current_month",
                        "DATE" => "current_date",
                        "DAYOFWEEK" => "current_day_of_week",
                        "HOUR" => "current_hour",
                        "MINUTE" => "current_minute",
                        "SECOND" => "current_second",
                        _ => "",
                    };
                    if method.is_empty() {
                        "0.0".to_owned()
                    } else {
                        format!("self.state.{method}()")
                    }
                }
                "sensing_dayssince2000" => "self.state.days_since_2000()".to_owned(),
                "sensing_online" => "false".to_owned(),
                "sensing_username" => "self.state.username()".to_owned(),
                "sensing_mousex" => "self.state.mouse_x()".to_owned(),
                "sensing_mousey" => "self.state.mouse_y()".to_owned(),
                "sensing_mousedown" => "self.state.mouse_down()".to_owned(),
                "sensing_loudness" => "self.state.loudness()".to_owned(),
                "sound_volume" => format!(
                    "self.state.sound_volume_for_target({})",
                    monitor_target.map_or(0, |(index, _)| index)
                ),
                "sensing_keypressed" => {
                    // a nonstandard saved monitor may point back to its full reporter block.
                    let active_block = monitor_target.and_then(|(index, target)| {
                        target
                            .blocks
                            .get(&monitor.id)
                            .map(|block| (index, target, block))
                    });
                    if let Some((index, target, block)) = active_block {
                        target_context = Some(index);
                        format!(
                            "state.key_pressed({})",
                            expression(
                                target,
                                block.inputs.get("KEY_OPTION"),
                                variables,
                                lists,
                                &HashMap::new(),
                                &mut HashSet::new(),
                            )
                        )
                    } else {
                        let key = monitor
                            .params
                            .get("KEY_OPTION")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or_default();
                        format!("state.key_pressed({})", quote(key))
                    }
                }
                // these reporters have their own snapshot paths.
                "data_variable"
                | "data_listcontents"
                | "sensing_timer"
                | "motion_xposition"
                | "motion_yposition"
                | "motion_direction"
                | "looks_size"
                | "looks_backdropnumbername"
                | "looks_costumenumbername" => return None,
                _ => return None,
            };
            let target_name = monitor_target.map(|(_, target)| target.name.as_str());
            let key = target_name.map_or_else(
                || format!("__reporter/{}/{}", monitor.opcode, monitor.id),
                |target| format!("__reporter/{target}/{}/{}", monitor.opcode, monitor.id),
            );
            Some((key, expression, target_context))
        })
        .collect()
}
