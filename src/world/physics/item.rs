use rg3d::gui::VerticalAlignment;
use rg3d::{
    asset::core::algebra::Vector2,
    core::pool::Handle,
    gui::{
        draw::DrawingContext,
        message::{MessageDirection, OsEvent, TextMessage, UiMessage, UiMessageData},
        text::TextBuilder,
        tree::{Tree, TreeBuilder},
        widget::{Widget, WidgetBuilder},
        BuildContext, Control, NodeHandleMapping, UiNode, UserInterface,
    },
};
use std::ops::{Deref, DerefMut};

#[derive(Debug)]
pub struct PhysicsItem<T> {
    pub tree: Tree,
    text: Handle<UiNode>,
    pub physics_entity: Handle<T>,
}

impl<T> Clone for PhysicsItem<T> {
    fn clone(&self) -> Self {
        Self {
            tree: self.tree.clone(),
            text: self.text,
            physics_entity: self.physics_entity,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PhysicsItemMessage {
    Name(String),
}

impl<T> Deref for PhysicsItem<T> {
    type Target = Widget;

    fn deref(&self) -> &Self::Target {
        &self.tree
    }
}

impl<T> DerefMut for PhysicsItem<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.tree
    }
}

impl<T: 'static> Control for PhysicsItem<T> {
    fn resolve(&mut self, _node_map: &NodeHandleMapping) {
        self.tree.resolve(_node_map)
    }

    fn measure_override(&self, ui: &UserInterface, available_size: Vector2<f32>) -> Vector2<f32> {
        self.tree.measure_override(ui, available_size)
    }

    fn arrange_override(&self, ui: &UserInterface, final_size: Vector2<f32>) -> Vector2<f32> {
        self.tree.arrange_override(ui, final_size)
    }

    fn draw(&self, _drawing_context: &mut DrawingContext) {
        self.tree.draw(_drawing_context)
    }

    fn update(&mut self, _dt: f32) {
        self.tree.update(_dt)
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.tree.handle_routed_message(ui, message);

        if let UiMessageData::User(msg) = message.data() {
            if let Some(PhysicsItemMessage::Name(name)) = msg.cast::<PhysicsItemMessage>() {
                ui.send_message(TextMessage::text(
                    self.text,
                    MessageDirection::ToWidget,
                    make_item_name(name, self.physics_entity),
                ));
            }
        }
    }

    fn preview_message(&self, _ui: &UserInterface, _message: &mut UiMessage) {
        self.tree.preview_message(_ui, _message)
    }

    fn handle_os_event(
        &mut self,
        _self_handle: Handle<UiNode>,
        _ui: &mut UserInterface,
        _event: &OsEvent,
    ) {
        self.tree.handle_os_event(_self_handle, _ui, _event)
    }

    fn remove_ref(&mut self, _handle: Handle<UiNode>) {
        self.tree.remove_ref(_handle)
    }
}

pub struct PhysicsItemBuilder<T> {
    tree_builder: TreeBuilder,
    name: String,
    physics_entity: Handle<T>,
}

fn make_item_name<T>(name: &str, handle: Handle<T>) -> String {
    format!("{} ({}:{})", name, handle.index(), handle.generation())
}

impl<T: 'static> PhysicsItemBuilder<T> {
    pub fn new(tree_builder: TreeBuilder) -> Self {
        Self {
            tree_builder,
            name: Default::default(),
            physics_entity: Default::default(),
        }
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.name = name;
        self
    }

    pub fn with_physics_entity(mut self, entity: Handle<T>) -> Self {
        self.physics_entity = entity;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let text = TextBuilder::new(WidgetBuilder::new())
            .with_vertical_text_alignment(VerticalAlignment::Center)
            .with_text(make_item_name(&self.name, self.physics_entity))
            .build(ctx);

        let node = PhysicsItem {
            tree: self.tree_builder.with_content(text).build_tree(ctx),
            text,
            physics_entity: self.physics_entity,
        };

        ctx.add_node(UiNode::new(node))
    }
}
