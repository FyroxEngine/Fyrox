use crate::gui::{
    VerticalAlignment,
    Thickness,
    UserInterface,
    HorizontalAlignment,
    node::{UINodeKind, UINode},
    event::UIEventHandler,
};
use rg3d_core::{
    color::Color,
    math::vec2::Vec2,
    pool::Handle,
};

pub struct CommonBuilderFields {
    pub(in crate::gui) name: Option<String>,
    pub(in crate::gui) width: Option<f32>,
    pub(in crate::gui) height: Option<f32>,
    pub(in crate::gui) desired_position: Option<Vec2>,
    pub(in crate::gui) vertical_alignment: Option<VerticalAlignment>,
    pub(in crate::gui) horizontal_alignment: Option<HorizontalAlignment>,
    pub(in crate::gui) max_size: Option<Vec2>,
    pub(in crate::gui) min_size: Option<Vec2>,
    pub(in crate::gui) color: Option<Color>,
    pub(in crate::gui) row: Option<usize>,
    pub(in crate::gui) column: Option<usize>,
    pub(in crate::gui) margin: Option<Thickness>,
    pub(in crate::gui) children: Vec<Handle<UINode>>,
    pub(in crate::gui) event_handler: Option<Box<UIEventHandler>>,
}

impl Default for CommonBuilderFields {
    fn default() -> Self {
        Self::new()
    }
}

impl CommonBuilderFields {
    pub fn new() -> Self {
        Self {
            name: None,
            width: None,
            height: None,
            vertical_alignment: None,
            horizontal_alignment: None,
            max_size: None,
            min_size: None,
            color: None,
            row: None,
            column: None,
            margin: None,
            desired_position: None,
            children: Vec::new(),
            event_handler: None,
        }
    }

    pub fn apply(&mut self, ui: &mut UserInterface, node_handle: Handle<UINode>) {
        let node = ui.nodes.borrow_mut(node_handle);
        if let Some(width) = self.width {
            node.width.set(width);
        }
        if let Some(height) = self.height {
            node.height.set(height);
        }
        if let Some(valign) = self.vertical_alignment {
            node.vertical_alignment = valign;
        }
        if let Some(halign) = self.horizontal_alignment {
            node.horizontal_alignment = halign;
        }
        if let Some(max_size) = self.max_size {
            node.max_size = max_size;
        }
        if let Some(min_size) = self.min_size {
            node.min_size = min_size;
        }
        if let Some(color) = self.color {
            node.color = color;
        }
        if let Some(row) = self.row {
            node.row = row;
        }
        if let Some(column) = self.column {
            node.column = column;
        }
        if let Some(margin) = self.margin {
            node.margin = margin;
        }
        if let Some(desired_position) = self.desired_position {
            node.desired_local_position.set(desired_position);
        }
        if let Some(name) = self.name.take() {
            node.name = name;
        }
        node.event_handler = self.event_handler.take();
        for child_handle in self.children.iter() {
            ui.link_nodes(*child_handle, node_handle);
        }
    }
}

#[macro_use]
macro_rules! impl_default_builder_methods {
    () => (
        pub fn with_width(mut self, width: f32) -> Self {
            self.common.width = Some(width);
            self
        }

        pub fn with_height(mut self, height: f32) -> Self {
            self.common.height = Some(height);
            self
        }

        pub fn with_vertical_alignment(mut self, valign: $crate::gui::VerticalAlignment) -> Self {
            self.common.vertical_alignment = Some(valign);
            self
        }

        pub fn with_horizontal_alignment(mut self, halign: $crate::gui::HorizontalAlignment) -> Self {
            self.common.horizontal_alignment = Some(halign);
            self
        }

        pub fn with_max_size(mut self, max_size: rg3d_core::math::vec2::Vec2) -> Self {
            self.common.max_size = Some(max_size);
            self
        }

        pub fn with_min_size(mut self, min_size: rg3d_core::math::vec2::Vec2) -> Self {
            self.common.min_size = Some(min_size);
            self
        }

        pub fn with_color(mut self, color: rg3d_core::color::Color) -> Self {
            self.common.color = Some(color);
            self
        }

        pub fn on_row(mut self, row: usize) -> Self {
            self.common.row = Some(row);
            self
        }

        pub fn on_column(mut self, column: usize) -> Self {
            self.common.column = Some(column);
            self
        }

        pub fn with_margin(mut self, margin: $crate::gui::Thickness) -> Self {
            self.common.margin = Some(margin);
            self
        }

        pub fn with_desired_position(mut self, desired_position: rg3d_core::math::vec2::Vec2) -> Self {
            self.common.desired_position = Some(desired_position);
            self
        }

        pub fn with_child(mut self, handle: rg3d_core::pool::Handle<$crate::gui::UINode>) -> Self {
            if handle.is_some() {
                self.common.children.push(handle);
            }
            self
        }

        pub fn with_name(mut self, name: &str) -> Self {
            self.common.name = Some(String::from(name));
            self
        }

        pub fn with_event_handler(mut self, handler: Box<$crate::gui::event::UIEventHandler>) -> Self {
            self.common.event_handler = Some(handler);
            self
        }
    )
}

pub struct GenericNodeBuilder {
    kind: UINodeKind,
    common: CommonBuilderFields,
}

impl GenericNodeBuilder {
    pub fn new(kind: UINodeKind, common: CommonBuilderFields) -> Self {
        Self {
            kind,
            common,
        }
    }

    impl_default_builder_methods!();

    pub fn build(mut self, ui: &mut UserInterface) -> Handle<UINode> {
        let handle = ui.add_node(UINode::new(self.kind));
        self.common.apply(ui, handle);
        handle
    }
}