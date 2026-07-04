use std::collections::HashMap;

use egui::Color32;
use egui_snarl::{
    ui::{PinInfo, SnarlViewer},
    InPinId, NodeId, Snarl,
};

use crate::nodes::{ExecutionState, PrismNode, Value};
use crate::plugin::NodeRegistration;

/// The viewer knows what nodes are available from the active vault's plugins
pub struct PrismViewer {
    /// Nodes grouped by category, populated from the vault's installed plugins
    pub available_nodes: Vec<(String, Vec<NodeRegistration>)>,
}

impl PrismViewer {
    pub fn new(available_nodes: Vec<(String, Vec<NodeRegistration>)>) -> Self {
        Self { available_nodes }
    }
}

// Colors for the UI
const PIN_IN_COLOR: Color32 = Color32::from_rgb(90, 180, 255);
const PIN_OUT_COLOR: Color32 = Color32::from_rgb(255, 160, 60);
const COLOR_SUCCESS: Color32 = Color32::from_rgb(80, 230, 160);
const COLOR_DARK: Color32 = Color32::from_rgb(255, 70, 90);
const COLOR_IDLE: Color32 = Color32::from_rgb(120, 120, 140);
const COLOR_CATEGORY: Color32 = Color32::from_rgb(180, 140, 255);

impl SnarlViewer<PrismNode> for PrismViewer {
    fn title(&mut self, node: &PrismNode) -> String {
        node.label.clone()
    }

    fn inputs(&mut self, node: &PrismNode) -> usize {
        node.num_inputs
    }

    fn outputs(&mut self, node: &PrismNode) -> usize {
        node.num_outputs
    }

    fn show_input(
        &mut self,
        pin: &egui_snarl::InPin,
        ui: &mut egui::Ui,
        snarl: &mut Snarl<PrismNode>,
    ) -> impl egui_snarl::ui::SnarlPin + 'static {
        let label = snarl[pin.id.node].input_label(pin.id.input).to_owned();
        ui.label(egui::RichText::new(&label).small().color(PIN_IN_COLOR));
        PinInfo::circle().with_fill(PIN_IN_COLOR)
    }

    fn show_output(
        &mut self,
        pin: &egui_snarl::OutPin,
        ui: &mut egui::Ui,
        snarl: &mut Snarl<PrismNode>,
    ) -> impl egui_snarl::ui::SnarlPin + 'static {
        let label = snarl[pin.id.node].output_label(pin.id.output).to_owned();
        ui.label(egui::RichText::new(&label).small().color(PIN_OUT_COLOR));
        PinInfo::circle().with_fill(PIN_OUT_COLOR)
    }

    fn has_body(&mut self, _node: &PrismNode) -> bool {
        true
    }

    fn show_body(
        &mut self,
        node: NodeId,
        _inputs: &[egui_snarl::InPin],
        _outputs: &[egui_snarl::OutPin],
        ui: &mut egui::Ui,
        snarl: &mut Snarl<PrismNode>,
    ) {
        let n = &mut snarl[node];

        // Node-specific editor UI
        match n.type_id.as_str() {
            "text_source" => {
                ui.add(
                    egui::TextEdit::singleline(&mut n.text_buffer)
                        .desired_width(160.0)
                        .hint_text("enter text..."),
                );
            }
            "number_source" => {
                ui.add(
                    egui::DragValue::new(&mut n.number_buffer)
                        .speed(0.1)
                        .prefix("val: "),
                );
            }
            _ => {
                // Show category tag
                ui.label(
                    egui::RichText::new(&n.category)
                        .small()
                        .color(COLOR_CATEGORY),
                );
            }
        }

        // Execution state indicator
        ui.add_space(2.0);
        match &n.state {
            ExecutionState::Idle => {
                ui.label(egui::RichText::new("idle").small().color(COLOR_IDLE));
            }
            ExecutionState::Success => {
                let output_text = format!("{}", n.last_output);
                let truncated = if output_text.len() > 30 {
                    format!("{}...", &output_text[..27])
                } else {
                    output_text
                };
                ui.label(egui::RichText::new(truncated).small().strong().color(COLOR_SUCCESS));
            }
            ExecutionState::Dark(msg) => {
                ui.label(egui::RichText::new(format!("DARK: {msg}")).small().color(COLOR_DARK));
            }
        }
    }

    fn has_graph_menu(&mut self, _pos: egui::Pos2, _snarl: &mut Snarl<PrismNode>) -> bool {
        true
    }

    fn show_graph_menu(
        &mut self,
        pos: egui::Pos2,
        ui: &mut egui::Ui,
        snarl: &mut Snarl<PrismNode>,
    ) {
        ui.label(egui::RichText::new("Add Atom").strong().color(COLOR_CATEGORY));
        ui.separator();

        if self.available_nodes.is_empty() {
            ui.colored_label(COLOR_IDLE, "No plugins installed");
            return;
        }

        for (category, nodes) in &self.available_nodes {
            ui.label(egui::RichText::new(category).small().strong().color(Color32::from_rgb(200, 200, 220)));
            for reg in nodes {
                let btn = ui.button(&reg.display_name);
                if btn.on_hover_text(&reg.description).clicked() {
                    snarl.insert_node(pos, PrismNode::from_registration(reg));
                    ui.close();
                }
            }
            ui.add_space(4.0);
        }
    }

    fn has_node_menu(&mut self, _node: &PrismNode) -> bool {
        true
    }

    fn show_node_menu(
        &mut self,
        node: NodeId,
        _inputs: &[egui_snarl::InPin],
        _outputs: &[egui_snarl::OutPin],
        ui: &mut egui::Ui,
        snarl: &mut Snarl<PrismNode>,
    ) {
        let type_id = snarl[node].type_id.clone();
        ui.label(egui::RichText::new(format!("type: {type_id}")).small().color(COLOR_IDLE));
        ui.separator();

        if ui.button("Reset").clicked() {
            snarl[node].state = ExecutionState::Idle;
            snarl[node].last_output = Value::Empty;
            ui.close();
        }
        if ui
            .button(egui::RichText::new("Remove").color(COLOR_DARK))
            .clicked()
        {
            snarl.remove_node(node);
            ui.close();
        }
    }
}

/// Topological sort + execute all nodes in dependency order
pub fn execute_graph(snarl: &mut Snarl<PrismNode>) {
    let node_ids: Vec<NodeId> = snarl.node_ids().map(|(id, _)| id).collect();

    for &id in &node_ids {
        snarl[id].state = ExecutionState::Idle;
        snarl[id].last_output = Value::Empty;
    }

    let mut in_degree: HashMap<NodeId, usize> = HashMap::new();
    let mut dependents: HashMap<NodeId, Vec<(NodeId, usize)>> = HashMap::new();

    for &id in &node_ids {
        in_degree.entry(id).or_insert(0);
    }

    for &id in &node_ids {
        let n_inputs = snarl[id].num_inputs;
        for input_idx in 0..n_inputs {
            let in_pin_id = InPinId { node: id, input: input_idx };
            let in_pin = snarl.in_pin(in_pin_id);
            for remote in &in_pin.remotes {
                dependents.entry(remote.node).or_default().push((id, input_idx));
                *in_degree.entry(id).or_insert(0) += 1;
            }
        }
    }

    // Kahn's algorithm
    let mut queue: Vec<NodeId> = node_ids
        .iter()
        .filter(|id| in_degree.get(id) == Some(&0))
        .copied()
        .collect();

    let mut order: Vec<NodeId> = Vec::new();

    while let Some(current) = queue.pop() {
        order.push(current);
        if let Some(deps) = dependents.get(&current) {
            for &(dep_node, _) in deps {
                if let Some(deg) = in_degree.get_mut(&dep_node) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push(dep_node);
                    }
                }
            }
        }
    }

    if order.len() != node_ids.len() {
        for &id in &node_ids {
            if !order.contains(&id) {
                snarl[id].state = ExecutionState::Dark("cycle detected".into());
            }
        }
        return;
    }

    let mut outputs: HashMap<NodeId, Value> = HashMap::new();

    for &id in &order {
        let n_inputs = snarl[id].num_inputs;
        let mut input_values: Vec<Value> = Vec::with_capacity(n_inputs);

        for input_idx in 0..n_inputs {
            let in_pin_id = InPinId { node: id, input: input_idx };
            let in_pin = snarl.in_pin(in_pin_id);
            let val = in_pin
                .remotes
                .first()
                .and_then(|remote| outputs.get(&remote.node))
                .cloned()
                .unwrap_or(Value::Empty);
            input_values.push(val);
        }

        let result = snarl[id].execute(&input_values);
        outputs.insert(id, result);
    }
}
