use rg3d::{
    core::{algebra::Vector2, pool::Handle},
    gui::{
        brush::Brush,
        core::color::Color,
        draw::{DrawingContext, SharedTexture},
        grid::{Column, GridBuilder, Row},
        image::ImageBuilder,
        message::{
            DecoratorMessage, MessageDirection, OsEvent, TextMessage, UiMessage, UiMessageData,
        },
        text::TextBuilder,
        tree::{Tree, TreeBuilder},
        widget::Widget,
        widget::WidgetBuilder,
        BuildContext, Control, NodeHandleMapping, Thickness, UiNode, UserInterface,
        VerticalAlignment,
    },
};
use std::{
    fmt::{Debug, Formatter},
    ops::{Deref, DerefMut},
};

#[derive(Debug, Clone, PartialEq)]
pub enum SceneItemMessage {
    Name(String),
    /// Odd or even.
    Order(bool),
}

impl SceneItemMessage {
    pub fn name(destination: Handle<UiNode>, name: String) -> UiMessage {
        UiMessage::user(
            destination,
            MessageDirection::ToWidget,
            Box::new(SceneItemMessage::Name(name)),
        )
    }
}

pub struct SceneItem<T> {
    pub tree: Tree,
    text_name: Handle<UiNode>,
    pub entity_handle: Handle<T>,
}

impl<T> Clone for SceneItem<T> {
    fn clone(&self) -> Self {
        Self {
            tree: self.tree.clone(),
            text_name: self.text_name,
            entity_handle: self.entity_handle,
        }
    }
}

impl<T> Debug for SceneItem<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "SceneItem")
    }
}

impl<T> Deref for SceneItem<T> {
    type Target = Widget;

    fn deref(&self) -> &Self::Target {
        &self.tree
    }
}

impl<T> DerefMut for SceneItem<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.tree
    }
}

impl<T: 'static> Control for SceneItem<T> {
    fn resolve(&mut self, node_map: &NodeHandleMapping) {
        self.tree.resolve(node_map);
        node_map.resolve(&mut self.text_name);
    }

    fn measure_override(&self, ui: &UserInterface, available_size: Vector2<f32>) -> Vector2<f32> {
        self.tree.measure_override(ui, available_size)
    }

    fn arrange_override(&self, ui: &UserInterface, final_size: Vector2<f32>) -> Vector2<f32> {
        self.tree.arrange_override(ui, final_size)
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        self.tree.draw(drawing_context);
    }

    fn update(&mut self, dt: f32) {
        self.tree.update(dt);
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.tree.handle_routed_message(ui, message);

        match message.data() {
            UiMessageData::User(msg) => {
                if let Some(msg) = msg.cast::<SceneItemMessage>() {
                    match msg {
                        &SceneItemMessage::Order(order) => {
                            if message.destination() == self.handle() {
                                ui.send_message(DecoratorMessage::normal_brush(
                                    self.tree.back(),
                                    MessageDirection::ToWidget,
                                    Brush::Solid(if order {
                                        Color::opaque(50, 50, 50)
                                    } else {
                                        Color::opaque(60, 60, 60)
                                    }),
                                ));
                            }
                        }
                        SceneItemMessage::Name(name) => {
                            if message.destination() == self.handle() {
                                let name = format!(
                                    "{} ({}:{})",
                                    name,
                                    self.entity_handle.index(),
                                    self.entity_handle.generation()
                                );

                                ui.send_message(TextMessage::text(
                                    self.text_name,
                                    MessageDirection::ToWidget,
                                    name,
                                ));
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn preview_message(&self, ui: &UserInterface, message: &mut UiMessage) {
        self.tree.preview_message(ui, message);
    }

    fn handle_os_event(
        &mut self,
        self_handle: Handle<UiNode>,
        ui: &mut UserInterface,
        event: &OsEvent,
    ) {
        self.tree.handle_os_event(self_handle, ui, event);
    }

    fn remove_ref(&mut self, handle: Handle<UiNode>) {
        self.tree.remove_ref(handle);
    }
}

pub struct SceneItemBuilder<T> {
    tree_builder: TreeBuilder,
    entity_handle: Handle<T>,
    name: String,
    icon: Option<SharedTexture>,
    text_brush: Option<Brush>,
}

impl<T: 'static> SceneItemBuilder<T> {
    pub fn new(tree_builder: TreeBuilder) -> Self {
        Self {
            tree_builder,
            entity_handle: Default::default(),
            name: Default::default(),
            icon: None,
            text_brush: None,
        }
    }

    pub fn with_entity_handle(mut self, entity_handle: Handle<T>) -> Self {
        self.entity_handle = entity_handle;
        self
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.name = name;
        self
    }

    pub fn with_icon(mut self, icon: Option<SharedTexture>) -> Self {
        self.icon = icon;
        self
    }

    pub fn with_text_brush(mut self, brush: Brush) -> Self {
        self.text_brush = Some(brush);
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let text_name;
        let content = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(
                    ImageBuilder::new(
                        WidgetBuilder::new()
                            .with_width(16.0)
                            .with_height(16.0)
                            .on_column(0)
                            .with_margin(Thickness::uniform(1.0)),
                    )
                    .with_opt_texture(self.icon)
                    .build(ctx),
                )
                .with_child({
                    text_name = TextBuilder::new(
                        WidgetBuilder::new()
                            .with_foreground(
                                self.text_brush
                                    .unwrap_or(Brush::Solid(rg3d::gui::COLOR_FOREGROUND)),
                            )
                            .with_margin(Thickness::uniform(1.0))
                            .on_column(1)
                            .with_vertical_alignment(VerticalAlignment::Center),
                    )
                    .with_text(format!(
                        "{} ({}:{})",
                        self.name,
                        self.entity_handle.index(),
                        self.entity_handle.generation()
                    ))
                    .build(ctx);
                    text_name
                }),
        )
        .add_row(Row::stretch())
        .add_column(Column::auto())
        .add_column(Column::stretch())
        .build(ctx);

        let tree = self.tree_builder.with_content(content).build_tree(ctx);

        let item = SceneItem {
            tree,
            entity_handle: self.entity_handle,
            text_name,
        };

        ctx.add_node(UiNode::new(item))
    }
}
