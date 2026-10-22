//! Produced variable bindings (semantic output/value roles), independent of
//! how they are rendered — see `sqyre-ui-model` for pill/tree chrome.

use crate::{Action, ActionKind, CoordinateOutputs, NavOutputs};

/// Produced variable binding for tree/output chips.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingRole {
    OutputX,
    OutputY,
    Value,
    Output,
    Ref,
    Atlas,
    Row,
    Col,
    Collection,
    Length,
}

impl BindingRole {
    pub fn pill_label(self) -> &'static str {
        match self {
            Self::OutputX => "X",
            Self::OutputY => "Y",
            Self::Length => "Length",
            Self::Value => "Variable",
            _ => "Output",
        }
    }

    pub fn validate_label(self, name: &str) -> String {
        let name = name.trim();
        match self {
            Self::Value => format!("variable {name:?}"),
            Self::Output => format!("output variable {name:?}"),
            Self::OutputX => format!("output X variable {name:?}"),
            Self::OutputY => format!("output Y variable {name:?}"),
            _ => format!("variable {name:?}"),
        }
    }
}

/// Produced variable binding for tree/output chips.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableBinding {
    pub name: String,
    pub role: BindingRole,
}

impl VariableBinding {
    pub fn pill_label(&self) -> &'static str {
        self.role.pill_label()
    }
}

impl CoordinateOutputs {
    pub fn variable_bindings(&self) -> Vec<VariableBinding> {
        let mut out = Vec::new();
        if !self.output_x_variable.is_empty() {
            out.push(VariableBinding {
                name: self.output_x_variable.clone(),
                role: BindingRole::OutputX,
            });
        }
        if !self.output_y_variable.is_empty() {
            out.push(VariableBinding {
                name: self.output_y_variable.clone(),
                role: BindingRole::OutputY,
            });
        }
        out
    }
}

impl NavOutputs {
    pub fn variable_bindings(&self) -> Vec<VariableBinding> {
        let mut out = Vec::new();
        for (name, role) in [
            (&self.output_ref, BindingRole::Ref),
            (&self.output_atlas, BindingRole::Atlas),
            (&self.output_row, BindingRole::Row),
            (&self.output_col, BindingRole::Col),
            (&self.output_collection, BindingRole::Collection),
        ] {
            if !name.is_empty() {
                out.push(VariableBinding {
                    name: name.clone(),
                    role,
                });
            }
        }
        out
    }
}

impl Action {
    pub fn variable_bindings(&self) -> Vec<VariableBinding> {
        self.kind.variable_bindings()
    }
}

impl ActionKind {
    pub fn variable_bindings(&self) -> Vec<VariableBinding> {
        match self {
            Self::SetVariable { assignments } => assignments
                .iter()
                .filter(|a| !a.variable_name.is_empty())
                .map(|a| VariableBinding {
                    name: a.variable_name.clone(),
                    role: BindingRole::Value,
                })
                .collect(),
            Self::ImageSearch { detection, .. } | Self::FindPixel { detection, .. } => {
                detection.coords.variable_bindings()
            }
            Self::Ocr {
                detection,
                output_variable,
                ..
            } => {
                let mut out = detection.coords.variable_bindings();
                if !output_variable.is_empty() {
                    out.push(VariableBinding {
                        name: output_variable.clone(),
                        role: BindingRole::Output,
                    });
                }
                out
            }
            Self::ForEachRow { sources, .. } => sources
                .iter()
                .filter(|s| !s.output_var.is_empty())
                .map(|s| VariableBinding {
                    name: s.output_var.clone(),
                    role: BindingRole::Output,
                })
                .collect(),
            Self::ForEachCell { .. } => Vec::new(),
            Self::NavigateSelect(data) => data.outputs.variable_bindings(),
            _ => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        DetectionBranch, ListColumn, NavOutputs, NavigateSelectData, ScalarValue,
        VariableAssignment,
    };

    #[test]
    fn pill_labels_cover_roles() {
        assert_eq!(BindingRole::OutputX.pill_label(), "X");
        assert_eq!(BindingRole::OutputY.pill_label(), "Y");
        assert_eq!(BindingRole::Length.pill_label(), "Length");
        assert_eq!(BindingRole::Value.pill_label(), "Variable");
        assert_eq!(BindingRole::Output.pill_label(), "Output");
        assert_eq!(BindingRole::Ref.pill_label(), "Output");
        let binding = VariableBinding {
            name: "n".into(),
            role: BindingRole::Col,
        };
        assert_eq!(binding.pill_label(), "Output");
    }

    #[test]
    fn validate_labels_distinguish_roles() {
        assert_eq!(
            BindingRole::Value.validate_label("score"),
            "variable \"score\""
        );
        assert_eq!(
            BindingRole::Output.validate_label("text"),
            "output variable \"text\""
        );
        assert_eq!(
            BindingRole::OutputX.validate_label("x"),
            "output X variable \"x\""
        );
        assert_eq!(BindingRole::Row.validate_label("r"), "variable \"r\"");
    }

    #[test]
    fn coordinate_outputs_binding_roles() {
        assert!(CoordinateOutputs::default().variable_bindings().is_empty());
        let out = CoordinateOutputs {
            output_x_variable: "px".into(),
            output_y_variable: "py".into(),
        }
        .variable_bindings();
        assert_eq!(
            out,
            vec![
                VariableBinding {
                    name: "px".into(),
                    role: BindingRole::OutputX
                },
                VariableBinding {
                    name: "py".into(),
                    role: BindingRole::OutputY
                },
            ]
        );
    }

    #[test]
    fn nav_outputs_bind_all_fields() {
        assert!(NavOutputs::default().variable_bindings().is_empty());
        let out = NavOutputs {
            output_ref: "ref".into(),
            output_atlas: "a".into(),
            output_row: "r".into(),
            output_col: "c".into(),
            output_collection: "col".into(),
        }
        .variable_bindings();
        let roles: Vec<BindingRole> = out.iter().map(|b| b.role).collect();
        assert_eq!(
            roles,
            vec![
                BindingRole::Ref,
                BindingRole::Atlas,
                BindingRole::Row,
                BindingRole::Col,
                BindingRole::Collection
            ]
        );
    }

    #[test]
    fn kind_variable_bindings_per_action() {
        // SetVariable skips blank names.
        let set = ActionKind::SetVariable {
            assignments: vec![
                VariableAssignment {
                    variable_name: "score".into(),
                    value: ScalarValue::Int(1),
                },
                VariableAssignment::default(),
            ],
        };
        let bindings = set.variable_bindings();
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].role, BindingRole::Value);

        // Detection kinds delegate to coordinate outputs.
        let mut detection = DetectionBranch::default();
        detection.coords.output_x_variable = "x".into();
        detection.coords.output_y_variable = "y".into();
        let search = ActionKind::ImageSearch {
            name: String::new(),
            targets: Vec::new(),
            target_tags: Vec::new(),
            search_area: crate::CoordinateRef::default(),
            tolerance: 0.95,
            blur: 0,
            match_method: crate::MatchMethod::CcoeffNormed,
            sort_by: Default::default(),
            sort_then: Default::default(),
            tag_priority: Vec::new(),
            detection,
        };
        assert_eq!(
            search.variable_bindings(),
            vec![
                VariableBinding {
                    name: "x".into(),
                    role: BindingRole::OutputX
                },
                VariableBinding {
                    name: "y".into(),
                    role: BindingRole::OutputY
                },
            ]
        );

        // ForEachRow binds populated output vars only.
        let fe = ActionKind::ForEachRow {
            name: String::new(),
            sources: vec![
                ListColumn {
                    source: "t".into(),
                    output_var: "cell".into(),
                    ..Default::default()
                },
                ListColumn::default(),
            ],
            start_row: ScalarValue::Int(1),
            end_row: ScalarValue::Int(1),
            subactions: Vec::new(),
        };
        assert_eq!(
            fe.variable_bindings(),
            vec![VariableBinding {
                name: "cell".into(),
                role: BindingRole::Output
            }]
        );

        // NavigateSelect delegates to NavOutputs.
        let nav = ActionKind::NavigateSelect(Box::new(NavigateSelectData {
            outputs: NavOutputs {
                output_row: "r".into(),
                ..Default::default()
            },
            ..Default::default()
        }));
        assert_eq!(
            nav.variable_bindings(),
            vec![VariableBinding {
                name: "r".into(),
                role: BindingRole::Row
            }]
        );

        // Leaf kinds bind nothing.
        assert!(ActionKind::Wait {
            time: ScalarValue::Int(0)
        }
        .variable_bindings()
        .is_empty());
    }
}
