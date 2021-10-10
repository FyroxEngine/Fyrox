use crate::{
    gui::SceneItemMessage,
    load_image,
    scene::commands::{graph::SetVisibleCommand, SceneCommand},
    Message,
};
use rg3d::{
    core::{algebra::Vector2, pool::Handle},
    engine::resource_manager::ResourceManager,
    gui::{
        brush::Brush,
        button::ButtonBuilder,
        core::color::Color,
        draw::{DrawingContext, SharedTexture},
        grid::{Column, GridBuilder, Row},
        image::ImageBuilder,
        message::{
            ButtonMessage, DecoratorMessage, MessageDirection, OsEvent, TextMessage, UiMessage,
            UiMessageData,
        },
        text::TextBuilder,
        tree::{Tree, TreeBuilder},
        widget::Widget,
        widget::WidgetBuilder,
        BuildContext, Control, HorizontalAlignment, NodeHandleMapping, Thickness, UiNode,
        UserInterface, VerticalAlignment,
    },
    scene::node::Node,
};
use std::{
    fmt::{Debug, Formatter},
    ops::{Deref, DerefMut},
    sync::mpsc::Sender,
};

#[derive(Clone)]
pub struct GraphNodeItem {
    pub tree: Tree,
    text_name: Handle<UiNode>,
    pub node: Handle<Node>,
    visibility_toggle: Handle<UiNode>,
    sender: Sender<Message>,
    visibility: bool,
    resource_manager: ResourceManager,
}

impl Debug for GraphNodeItem {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "SceneItem")
    }
}

impl Deref for GraphNodeItem {
    type Target = Widget;

    fn deref(&self) -> &Self::Target {
        &self.tree
    }
}

impl DerefMut for GraphNodeItem {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.tree
    }
}

impl Control for GraphNodeItem {
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
            UiMessageData::Button(msg) => {
                if message.destination() == self.visibility_toggle {
                    if let ButtonMessage::Click = msg {
                        let command =
                            SceneCommand::new(SetVisibleCommand::new(self.node, !self.visibility));
                        self.sender.send(Message::DoSceneCommand(command)).unwrap();
                    }
                }
            }
            UiMessageData::User(msg) => {
                if let Some(msg) = msg.cast::<SceneItemMessage>() {
                    match msg {
                        &SceneItemMessage::NodeVisibility(visibility) => {
                            if self.visibility != visibility
                                && message.destination() == self.handle()
                            {
                                self.visibility = visibility;
                                let image = if visibility {
                                    load_image(include_bytes!(
                                        "../../../resources/embed/visible.png"
                                    ))
                                } else {
                                    load_image(include_bytes!(
                                        "../../../resources/embed/invisible.png"
                                    ))
                                };
                                let image = ImageBuilder::new(WidgetBuilder::new())
                                    .with_opt_texture(image)
                                    .build(&mut ui.build_ctx());
                                ui.send_message(ButtonMessage::content(
                                    self.visibility_toggle,
                                    MessageDirection::ToWidget,
                                    image,
                                ));
                            }
                        }
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
                                    self.node.index(),
                                    self.node.generation()
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

#[derive(Default)]
pub struct SceneItemBuilder {
    node: Handle<Node>,
    name: String,
    visibility: bool,
    icon: Option<SharedTexture>,
    context_menu: Handle<UiNode>,
}

impl SceneItemBuilder {
    pub fn new() -> Self {
        Self {
            node: Default::default(),
            name: Default::default(),
            visibility: true,
            icon: None,
            context_menu: Default::default(),
        }
    }

    pub fn with_node(mut self, node: Handle<Node>) -> Self {
        self.node = node;
        self
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.name = name;
        self
    }

    pub fn with_visibility(mut self, visibility: bool) -> Self {
        self.visibility = visibility;
        self
    }

    pub fn with_icon(mut self, icon: Option<SharedTexture>) -> Self {
        self.icon = icon;
        self
    }

    pub fn with_context_menu(mut self, menu: Handle<UiNode>) -> Self {
        self.context_menu = menu;
        self
    }

    pub fn build(
        self,
        ctx: &mut BuildContext,
        sender: Sender<Message>,
        resource_manager: ResourceManager,
        node: &Node,
    ) -> Handle<UiNode> {
        let visible_texture = load_image(include_bytes!("../../../resources/embed/visible.png"));

        let text_name;
        let visibility_toggle;
        let tree = TreeBuilder::new(
            WidgetBuilder::new()
                .with_context_menu(self.context_menu)
                .with_margin(Thickness {
                    left: 1.0,
                    top: 1.0,
                    right: 0.0,
                    bottom: 0.0,
                }),
        )
        .with_content(
            GridBuilder::new(
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
                                .with_foreground(if node.resource().is_some() {
                                    Brush::Solid(Color::opaque(160, 160, 200))
                                } else {
                                    Brush::Solid(rg3d::gui::COLOR_FOREGROUND)
                                })
                                .with_margin(Thickness::uniform(1.0))
                                .on_column(1)
                                .with_vertical_alignment(VerticalAlignment::Center),
                        )
                        .with_text(format!(
                            "{} ({}:{})",
                            self.name,
                            self.node.index(),
                            self.node.generation()
                        ))
                        .build(ctx);
                        text_name
                    })
                    .with_child({
                        visibility_toggle = ButtonBuilder::new(
                            WidgetBuilder::new()
                                .with_margin(Thickness::uniform(1.0))
                                .with_width(22.0)
                                .with_height(16.0)
                                .with_horizontal_alignment(HorizontalAlignment::Right)
                                .on_column(2),
                        )
                        .with_content(
                            ImageBuilder::new(
                                WidgetBuilder::new().with_margin(Thickness::uniform(1.0)),
                            )
                            .with_opt_texture(visible_texture)
                            .build(ctx),
                        )
                        .build(ctx);
                        visibility_toggle
                    }),
            )
            .add_row(Row::stretch())
            .add_column(Column::auto())
            .add_column(Column::auto())
            .add_column(Column::stretch())
            .build(ctx),
        )
        .build_tree(ctx);

        let item = GraphNodeItem {
            tree,
            node: self.node,
            visibility_toggle,
            sender,
            visibility: self.visibility,
            resource_manager,
            text_name,
        };

        ctx.add_node(UiNode::new(item))
    }
}
