use serde::{Deserialize, Serialize};

use crate::plugin::NodeRegistration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Value {
    Text(String),
    Number(f64),
    Empty,
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Text(s) => write!(f, "{s}"),
            Value::Number(n) => write!(f, "{n:.2}"),
            Value::Empty => write!(f, "—"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionState {
    Idle,
    Success,
    Dark(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrismNode {
    /// Which plugin type_id created this node
    pub type_id: String,
    pub label: String,
    pub category: String,
    pub num_inputs: usize,
    pub num_outputs: usize,
    pub input_labels: Vec<String>,
    pub output_labels: Vec<String>,
    pub last_output: Value,
    pub state: ExecutionState,
    // User-editable buffers
    pub text_buffer: String,
    pub number_buffer: f64,
}

impl PrismNode {
    /// Create a node instance from a plugin's NodeRegistration
    pub fn from_registration(reg: &NodeRegistration) -> Self {
        Self {
            type_id: reg.type_id.clone(),
            label: reg.display_name.clone(),
            category: reg.category.clone(),
            num_inputs: reg.num_inputs,
            num_outputs: reg.num_outputs,
            input_labels: reg.input_labels.clone(),
            output_labels: reg.output_labels.clone(),
            last_output: Value::Empty,
            state: ExecutionState::Idle,
            text_buffer: String::new(),
            number_buffer: 0.0,
        }
    }

    pub fn from_registration_with_text(reg: &NodeRegistration, text: &str) -> Self {
        let mut node = Self::from_registration(reg);
        node.text_buffer = text.to_string();
        node
    }

    pub fn from_registration_with_number(reg: &NodeRegistration, num: f64) -> Self {
        let mut node = Self::from_registration(reg);
        node.number_buffer = num;
        node
    }

    pub fn input_label(&self, idx: usize) -> &str {
        self.input_labels.get(idx).map(|s| s.as_str()).unwrap_or("in")
    }

    pub fn output_label(&self, idx: usize) -> &str {
        self.output_labels.get(idx).map(|s| s.as_str()).unwrap_or("out")
    }

    /// Returns true if this node has user-editable content in its body
    pub fn has_editor(&self) -> bool {
        matches!(
            self.type_id.as_str(),
            "text_source" | "number_source"
        )
    }

    /// Execute this node given its inputs, return output
    pub fn execute(&mut self, inputs: &[Value]) -> Value {
        let result = match self.type_id.as_str() {
            "text_source" => Value::Text(self.text_buffer.clone()),
            "number_source" => Value::Number(self.number_buffer),
            "uppercase" => match inputs.first().unwrap_or(&Value::Empty) {
                Value::Text(s) => Value::Text(s.to_uppercase()),
                other => other.clone(),
            },
            "lowercase" => match inputs.first().unwrap_or(&Value::Empty) {
                Value::Text(s) => Value::Text(s.to_lowercase()),
                other => other.clone(),
            },
            "reverse" => match inputs.first().unwrap_or(&Value::Empty) {
                Value::Text(s) => Value::Text(s.chars().rev().collect()),
                other => other.clone(),
            },
            "length" => match inputs.first().unwrap_or(&Value::Empty) {
                Value::Text(s) => Value::Number(s.len() as f64),
                _ => Value::Number(0.0),
            },
            "double" => match inputs.first().unwrap_or(&Value::Empty) {
                Value::Number(n) => Value::Number(n * 2.0),
                other => other.clone(),
            },
            "negate" => match inputs.first().unwrap_or(&Value::Empty) {
                Value::Number(n) => Value::Number(-n),
                other => other.clone(),
            },
            "add" => {
                let a = match inputs.first().unwrap_or(&Value::Empty) {
                    Value::Number(n) => *n,
                    _ => 0.0,
                };
                let b = match inputs.get(1).unwrap_or(&Value::Empty) {
                    Value::Number(n) => *n,
                    _ => 0.0,
                };
                Value::Number(a + b)
            }
            "merge" => {
                let a = inputs.first().unwrap_or(&Value::Empty);
                let b = inputs.get(1).unwrap_or(&Value::Empty);
                Value::Text(format!("{a} | {b}"))
            }
            "sink" => inputs.first().unwrap_or(&Value::Empty).clone(),
            _ => Value::Empty,
        };

        self.last_output = result.clone();
        self.state = ExecutionState::Success;
        result
    }
}
