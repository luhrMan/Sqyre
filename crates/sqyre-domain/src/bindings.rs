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
    Graph,
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
            (&self.output_graph, BindingRole::Graph),
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
            Self::SetVariable { variable_name, .. } if !variable_name.is_empty() => {
                vec![VariableBinding {
                    name: variable_name.clone(),
                    role: BindingRole::Value,
                }]
            }
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
            Self::NavigateSelect(data) => data.outputs.variable_bindings(),
            _ => Vec::new(),
        }
    }
}
