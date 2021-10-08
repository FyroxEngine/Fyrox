use rg3d::{
    core::pool::Handle,
    gui::{
        border::BorderBuilder,
        decorator::DecoratorBuilder,
        message::{MessageDirection, TextMessage, UiMessage, UiMessageData},
        text::TextBuilder,
        widget::{Widget, WidgetBuilder},
        BuildContext, Control, NodeHandleMapping, UiNode, UserInterface,
    },
};
use std::ops::{Deref, DerefMut};

#[derive(Debug)]
pub struct PhysicsItem<T> {
    widget: Widget,
    text: Handle<UiNode>,
    pub physics_entity: Handle<T>,
}

impl<T> Clone for PhysicsItem<T> {
    fn clone(&self) -> Self {
        Self {
            widget: self.widget.clone(),
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
        &self.widget
    }
}

impl<T> DerefMut for PhysicsItem<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl<T: 'static> Control for PhysicsItem<T> {
    fn resolve(&mut self, node_map: &NodeHandleMapping) {
        node_map.resolve(&mut self.text)
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
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
}

pub struct PhysicsItemBuilder<T> {
    widget_builder: WidgetBuilder,
    name: String,
    physics_entity: Handle<T>,
}

fn make_item_name<T>(name: &str, handle: Handle<T>) -> String {
    format!("{} ({}:{})", name, handle.index(), handle.generation())
}

impl<T: 'static> PhysicsItemBuilder<T> {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
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
        let text;
        let decorator =
            DecoratorBuilder::new(BorderBuilder::new(WidgetBuilder::new().with_child({
                text = TextBuilder::new(WidgetBuilder::new())
                    .with_text(make_item_name(&self.name, self.physics_entity))
                    .build(ctx);
                text
            })))
            .build(ctx);

        let node = PhysicsItem {
            widget: self.widget_builder.with_child(decorator).build(),
            text,
            physics_entity: self.physics_entity,
        };

        ctx.add_node(UiNode::new(node))
    }
}
