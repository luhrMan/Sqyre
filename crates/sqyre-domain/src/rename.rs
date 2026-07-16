//! Propagate program / entity renames into macro action references.

use crate::{ActionKind, CoordinateRef, Macro, PROGRAM_DELIMITER};

/// Kind of program-owned entity referenced from macros.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgramEntityKind {
    Point,
    SearchArea,
    Item,
    Collection,
}

impl Macro {
    /// Update actions that reference the renamed entity within `program`.
    /// Returns true when any action was modified.
    pub fn rename_program_entity(
        &mut self,
        kind: ProgramEntityKind,
        program: &str,
        old_name: &str,
        new_name: &str,
    ) -> bool {
        let program = program.trim();
        let old_name = old_name.trim();
        let new_name = new_name.trim();
        if old_name == new_name || old_name.is_empty() || new_name.is_empty() {
            return false;
        }

        let mut changed = false;
        self.root.walk_mut(&mut |a| {
            match kind {
                ProgramEntityKind::Point => {
                    if let ActionKind::Move { point, .. } = &mut a.kind {
                        if point.is_collection() {
                            return;
                        }
                        let next = rename_coordinate_entity(point, program, old_name, new_name);
                        if next != *point {
                            *point = next;
                            changed = true;
                        }
                    }
                }
                ProgramEntityKind::SearchArea => match &mut a.kind {
                    ActionKind::ImageSearch { search_area, .. }
                    | ActionKind::Ocr { search_area, .. }
                    | ActionKind::FindPixel { search_area, .. } => {
                        if search_area.is_collection() {
                            return;
                        }
                        let next =
                            rename_coordinate_entity(search_area, program, old_name, new_name);
                        if next != *search_area {
                            *search_area = next;
                            changed = true;
                        }
                    }
                    _ => {}
                },
                ProgramEntityKind::Collection => match &mut a.kind {
                    ActionKind::Move { point, .. } => {
                        if !point.is_collection() {
                            return;
                        }
                        let next = rename_coordinate_entity(point, program, old_name, new_name);
                        if next != *point {
                            *point = next;
                            changed = true;
                        }
                    }
                    ActionKind::ImageSearch { search_area, .. }
                    | ActionKind::Ocr { search_area, .. }
                    | ActionKind::FindPixel { search_area, .. } => {
                        if !search_area.is_collection() {
                            return;
                        }
                        let next =
                            rename_coordinate_entity(search_area, program, old_name, new_name);
                        if next != *search_area {
                            *search_area = next;
                            changed = true;
                        }
                    }
                    ActionKind::NavigateSelect {
                        program: prog,
                        graph_name,
                        ..
                    } => {
                        if prog == program && graph_name == old_name {
                            *graph_name = new_name.to_string();
                            changed = true;
                        }
                    }
                    _ => {}
                },
                ProgramEntityKind::Item => {
                    if let ActionKind::ImageSearch { targets, .. } = &mut a.kind {
                        for target in targets.iter_mut() {
                            let renamed =
                                rename_item_target_entity(target, program, old_name, new_name);
                            if renamed != *target {
                                *target = renamed;
                                changed = true;
                            }
                        }
                    }
                }
            }
        });
        changed
    }

    /// Update actions that reference entities under the old program name.
    pub fn rename_program(&mut self, old_program: &str, new_program: &str) -> bool {
        let old_program = old_program.trim();
        let new_program = new_program.trim();
        if old_program == new_program || old_program.is_empty() || new_program.is_empty() {
            return false;
        }

        let mut changed = false;
        self.root.walk_mut(&mut |a| match &mut a.kind {
            ActionKind::Move { point, .. } => {
                let next = rename_coordinate_program(point, old_program, new_program);
                if next != *point {
                    *point = next;
                    changed = true;
                }
            }
            ActionKind::ImageSearch {
                search_area,
                targets,
                ..
            } => {
                let next = rename_coordinate_program(search_area, old_program, new_program);
                if next != *search_area {
                    *search_area = next;
                    changed = true;
                }
                for target in targets.iter_mut() {
                    let renamed = rename_item_target_program(target, old_program, new_program);
                    if renamed != *target {
                        *target = renamed;
                        changed = true;
                    }
                }
            }
            ActionKind::Ocr { search_area, .. } | ActionKind::FindPixel { search_area, .. } => {
                let next = rename_coordinate_program(search_area, old_program, new_program);
                if next != *search_area {
                    *search_area = next;
                    changed = true;
                }
            }
            ActionKind::NavigateSelect { program, .. } => {
                if program == old_program {
                    *program = new_program.to_string();
                    changed = true;
                }
            }
            _ => {}
        });
        changed
    }

    /// Update Run Macro actions that call the renamed macro.
    pub fn rename_macro_reference(&mut self, old_name: &str, new_name: &str) -> bool {
        let old_name = old_name.trim();
        let new_name = new_name.trim();
        if old_name == new_name || old_name.is_empty() || new_name.is_empty() {
            return false;
        }
        let mut changed = false;
        self.root.walk_mut(&mut |a| {
            if let ActionKind::RunMacro { macro_name } = &mut a.kind {
                if macro_name == old_name {
                    *macro_name = new_name.to_string();
                    changed = true;
                }
            }
        });
        changed
    }
}

fn rename_coordinate_entity(
    ref_: &CoordinateRef,
    program: &str,
    old_name: &str,
    new_name: &str,
) -> CoordinateRef {
    if ref_.is_empty() {
        return ref_.clone();
    }
    if let Some(prog) = ref_.program() {
        if prog != program || ref_.name() != old_name {
            return ref_.clone();
        }
        return ref_.with_entity_name(prog, new_name);
    }
    if ref_.name() != old_name {
        return ref_.clone();
    }
    ref_.with_entity_name("", new_name)
}

fn rename_coordinate_program(
    ref_: &CoordinateRef,
    old_program: &str,
    new_program: &str,
) -> CoordinateRef {
    if ref_.is_empty() || ref_.program() != Some(old_program) {
        return ref_.clone();
    }
    ref_.with_entity_name(new_program, ref_.name())
}

fn rename_item_target_entity(target: &str, program: &str, old_item: &str, new_item: &str) -> String {
    let Some((prog, rest)) = target.split_once(PROGRAM_DELIMITER) else {
        return target.to_string();
    };
    if prog != program {
        return target.to_string();
    }
    match rest.split_once(PROGRAM_DELIMITER) {
        Some((base, variant)) if base == old_item => {
            format!("{program}{PROGRAM_DELIMITER}{new_item}{PROGRAM_DELIMITER}{variant}")
        }
        None if rest == old_item => {
            format!("{program}{PROGRAM_DELIMITER}{new_item}")
        }
        _ => target.to_string(),
    }
}

fn rename_item_target_program(target: &str, old_program: &str, new_program: &str) -> String {
    let Some((prog, rest)) = target.split_once(PROGRAM_DELIMITER) else {
        return target.to_string();
    };
    if prog != old_program {
        return target.to_string();
    }
    format!("{new_program}{PROGRAM_DELIMITER}{rest}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{root_loop, Action, ActionId, ActionKind, MatchOrder, WaitTilFoundConfig};

    fn move_action(point: &str) -> Action {
        Action {
            id: ActionId::new(),
            kind: ActionKind::Move {
                point: CoordinateRef(point.into()),
                smooth: false,
                smooth_low: 0.0,
                smooth_high: 0.0,
                smooth_delay_ms: 0,
            },
        }
    }

    fn image_search(targets: Vec<&str>, area: &str) -> Action {
        Action {
            id: ActionId::new(),
            kind: ActionKind::ImageSearch {
                name: String::new(),
                targets: targets.into_iter().map(str::to_string).collect(),
                search_area: CoordinateRef(area.into()),
                tolerance: 0.9,
                blur: 0,
                wait: WaitTilFoundConfig::default(),
                coords: Default::default(),
                run_branch_on_no_find: false,
                order: MatchOrder::default(),
                subactions: Vec::new(),
            },
        }
    }

    #[test]
    fn renames_point_and_item_refs() {
        let mut m = Macro::new("t", 0, vec![]);
        m.root = root_loop(vec![
            move_action("Game~Spot"),
            image_search(vec!["Game~Potion"], "Game~Box"),
        ]);
        assert!(m.rename_program_entity(ProgramEntityKind::Point, "Game", "Spot", "Spawn"));
        assert!(m.rename_program_entity(ProgramEntityKind::Item, "Game", "Potion", "Elixir"));
        assert!(m.rename_program_entity(ProgramEntityKind::SearchArea, "Game", "Box", "Area"));

        let mut points = Vec::new();
        m.root.walk(&mut |a| {
            if let ActionKind::Move { point, .. } = &a.kind {
                points.push(point.as_str().to_string());
            }
            if let ActionKind::ImageSearch {
                targets,
                search_area,
                ..
            } = &a.kind
            {
                assert_eq!(targets, &vec!["Game~Elixir".to_string()]);
                assert_eq!(search_area.as_str(), "Game~Area");
            }
        });
        assert_eq!(points, vec!["Game~Spawn".to_string()]);
    }

    #[test]
    fn renames_program_prefix() {
        let mut m = Macro::new("t", 0, vec![]);
        m.root = root_loop(vec![
            move_action("Old~Spot"),
            image_search(vec!["Old~Item~v1"], "Old~Box"),
        ]);
        assert!(m.rename_program("Old", "New"));
        m.root.walk(&mut |a| match &a.kind {
            ActionKind::Move { point, .. } => assert_eq!(point.as_str(), "New~Spot"),
            ActionKind::ImageSearch {
                targets,
                search_area,
                ..
            } => {
                assert_eq!(targets, &vec!["New~Item~v1".to_string()]);
                assert_eq!(search_area.as_str(), "New~Box");
            }
            _ => {}
        });
    }
}
