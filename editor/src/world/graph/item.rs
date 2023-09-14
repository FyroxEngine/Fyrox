use crate::{load_image, message::MessageSender, utils::make_node_name, Message};
use fyrox::scene::node::Node;
use fyrox::{
    core::{algebra::Vector2, pool::Handle},
    gui::{
        brush::Brush,
        define_constructor,
        draw::{DrawingContext, SharedTexture},
        grid::{Column, GridBuilder, Row},
        image::ImageBuilder,
        message::{MessageDirection, OsEvent, UiMessage},
        text::{TextBuilder, TextMessage},
        tree::{Tree, TreeBuilder},
        utils::make_simple_tooltip,
        widget::{Widget, WidgetBuilder, WidgetMessage},
        BuildContext, Control, NodeHandleMapping, Thickness, UiNode, UserInterface,
        VerticalAlignment,
    },
};
use std::{
    any::{Any, TypeId},
    fmt::{Debug, Formatter},
    ops::{Deref, DerefMut},
    sync::mpsc::Sender,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SceneItemMessage {
    Name(String),
    Validate(Result<(), String>),
}

impl SceneItemMessage {
    define_constructor!(Self:Name => fn name(String), layout: false);
    define_constructor!(Self:Validate => fn validate(Result<(), String>), layout: false);
}

pub struct SceneItem {
    pub tree: Tree,
    text_name: Handle<UiNode>,
    name_value: String,
    grid: Handle<UiNode>,
    pub entity_handle: Handle<Node>,
    // Can be unassigned if there's no warning.
    pub warning_icon: Handle<UiNode>,
    sender: MessageSender,
}

impl SceneItem {
    pub fn name(&self) -> &str {
        &self.name_value
    }
}

impl Clone for SceneItem {
    fn clone(&self) -> Self {
        Self {
            tree: self.tree.clone(),
            text_name: self.text_name,
            name_value: self.name_value.clone(),
            grid: self.grid,
            entity_handle: self.entity_handle,
            warning_icon: self.warning_icon,
            sender: self.sender.clone(),
        }
    }
}

impl Debug for SceneItem {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "SceneItem")
    }
}

impl Deref for SceneItem {
    type Target = Widget;

    fn deref(&self) -> &Self::Target {
        &self.tree
    }
}

impl DerefMut for SceneItem {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.tree
    }
}

impl Control for SceneItem {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        self.tree.query_component(type_id).or_else(|| {
            if type_id == TypeId::of::<Self>() {
                Some(self)
            } else {
                None
            }
        })
    }

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

    fn update(&mut self, dt: f32, sender: &Sender<UiMessage>) {
        self.tree.update(dt, sender);
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.tree.handle_routed_message(ui, message);

        if let Some(SceneItemMessage::Name(name)) = message.data() {
            if message.destination() == self.handle() {
                self.name_value = make_node_name(name, self.entity_handle.into());

                ui.send_message(TextMessage::text(
                    self.text_name,
                    MessageDirection::ToWidget,
                    self.name_value.clone(),
                ));
            }
        } else if let Some(SceneItemMessage::Validate(result)) = message.data() {
            if message.destination() == self.handle() {
                match result {
                    Ok(_) => {
                        ui.send_message(WidgetMessage::remove(
                            self.warning_icon,
                            MessageDirection::ToWidget,
                        ));
                        self.warning_icon = Handle::NONE;
                    }
                    Err(msg) => {
                        self.warning_icon = ImageBuilder::new(
                            WidgetBuilder::new()
                                .with_width(20.0)
                                .with_height(20.0)
                                .with_tooltip(make_simple_tooltip(&mut ui.build_ctx(), msg))
                                .with_margin(Thickness::uniform(1.0))
                                .on_row(0)
                                .on_column(2),
                        )
                        .with_opt_texture(load_image(include_bytes!(
                            "../../../resources/embed/warning.png"
                        )))
                        .build(&mut ui.build_ctx());

                        ui.send_message(WidgetMessage::link(
                            self.warning_icon,
                            MessageDirection::ToWidget,
                            self.grid,
                        ));
                    }
                }
            }
        } else if let Some(WidgetMessage::DoubleClick { .. }) = message.data() {
            let flag = 0b0010;
            if message.flags & flag != flag {
                self.sender.send(Message::FocusObject(self.entity_handle));
                message.set_handled(true);
                message.flags |= flag;
            }
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
}

pub struct SceneItemBuilder {
    tree_builder: TreeBuilder,
    entity_handle: Handle<Node>,
    name: String,
    icon: Option<SharedTexture>,
    text_brush: Option<Brush>,
}

impl SceneItemBuilder {
    pub fn new(tree_builder: TreeBuilder) -> Self {
        Self {
            tree_builder,
            entity_handle: Default::default(),
            name: Default::default(),
            icon: None,
            text_brush: None,
        }
    }

    pub fn with_entity_handle(mut self, entity_handle: Handle<Node>) -> Self {
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

    pub fn build(self, ctx: &mut BuildContext, sender: MessageSender) -> Handle<UiNode> {
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
                                    .unwrap_or(Brush::Solid(fyrox::gui::COLOR_FOREGROUND)),
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
        .add_column(Column::auto())
        .build(ctx);

        let tree = self.tree_builder.with_content(content).build_tree(ctx);

        let item = SceneItem {
            tree,
            entity_handle: self.entity_handle,
            name_value: self.name,
            text_name,
            grid: content,
            warning_icon: Default::default(),
            sender,
        };

        ctx.add_node(UiNode::new(item))
    }
}
