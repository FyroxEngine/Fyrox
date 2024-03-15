use crate::fyrox::core::color::Color;
use crate::fyrox::graph::BaseSceneGraph;
use crate::fyrox::gui::draw::{CommandTexture, Draw};
use crate::fyrox::{
    asset::untyped::UntypedResource,
    core::{
        algebra::Vector2, pool::ErasedHandle, pool::Handle, reflect::prelude::*,
        type_traits::prelude::*, uuid_provider, visitor::prelude::*,
    },
    gui::{
        brush::Brush,
        define_constructor,
        draw::DrawingContext,
        grid::{Column, GridBuilder, Row},
        image::ImageBuilder,
        message::{MessageDirection, OsEvent, UiMessage},
        text::{TextBuilder, TextMessage},
        tree::{Tree, TreeBuilder},
        utils::make_simple_tooltip,
        widget::{Widget, WidgetBuilder, WidgetMessage},
        BuildContext, Control, Thickness, UiNode, UserInterface, VerticalAlignment,
    },
};
use crate::{load_image, message::MessageSender, utils::make_node_name, Message};
use std::{
    fmt::{Debug, Formatter},
    ops::{Deref, DerefMut},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SceneItemMessage {
    Name(String),
    Validate(Result<(), String>),
}

impl SceneItemMessage {
    define_constructor!(SceneItemMessage:Name => fn name(String), layout: false);
    define_constructor!(SceneItemMessage:Validate => fn validate(Result<(), String>), layout: false);
}

#[derive(Copy, Clone)]
pub enum DropAnchor {
    Side {
        visual_offset: f32,
        index_offset: isize,
    },
    OnTop,
}

#[derive(Visit, Reflect, ComponentProvider)]
pub struct SceneItem {
    #[component(include)]
    pub tree: Tree,
    text_name: Handle<UiNode>,
    name_value: String,
    grid: Handle<UiNode>,
    pub entity_handle: ErasedHandle,
    // Can be unassigned if there's no warning.
    pub warning_icon: Handle<UiNode>,
    #[reflect(hidden)]
    #[visit(skip)]
    sender: MessageSender,
    #[reflect(hidden)]
    #[visit(skip)]
    pub drop_anchor: DropAnchor,
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
            drop_anchor: self.drop_anchor,
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

uuid_provider!(SceneItem = "16f35257-a250-413b-ab51-b1ad086a3a9c");

impl Control for SceneItem {
    fn measure_override(&self, ui: &UserInterface, available_size: Vector2<f32>) -> Vector2<f32> {
        self.tree.measure_override(ui, available_size)
    }

    fn arrange_override(&self, ui: &UserInterface, final_size: Vector2<f32>) -> Vector2<f32> {
        self.tree.arrange_override(ui, final_size)
    }

    fn post_draw(&self, drawing_context: &mut DrawingContext) {
        self.tree.draw(drawing_context);

        let width = self.screen_bounds().w();
        match self.drop_anchor {
            DropAnchor::Side { visual_offset, .. } => {
                drawing_context.push_line(
                    Vector2::new(0.0, visual_offset),
                    Vector2::new(width, visual_offset),
                    2.0,
                );
            }
            DropAnchor::OnTop => {}
        }
        drawing_context.commit(
            self.clip_bounds().inflate(0.0, 2.0),
            Brush::Solid(Color::CORN_FLOWER_BLUE),
            CommandTexture::None,
            None,
        );
    }

    fn update(&mut self, dt: f32, ui: &mut UserInterface) {
        self.tree.update(dt, ui);
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.tree.handle_routed_message(ui, message);

        if let Some(SceneItemMessage::Name(name)) = message.data() {
            if message.destination() == self.handle() {
                self.name_value = make_node_name(name, self.entity_handle);

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
                            "../../../resources/warning.png"
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
                self.sender
                    .send(Message::FocusObject(self.entity_handle.into()));
                message.set_handled(true);
                message.flags |= flag;
            }
        } else if let Some(msg) = message.data::<WidgetMessage>() {
            match msg {
                WidgetMessage::DragOver(_) => {
                    if let Some(background) = ui.try_get(self.tree.background) {
                        let cursor_pos = ui.cursor_position();
                        let bounds = background.screen_bounds();
                        let deflated_bounds = bounds.deflate(0.0, 5.0);
                        if bounds.contains(cursor_pos) {
                            if cursor_pos.y < deflated_bounds.y() {
                                self.drop_anchor = DropAnchor::Side {
                                    visual_offset: 0.0,
                                    index_offset: 0,
                                };
                            } else if deflated_bounds.contains(cursor_pos) {
                                self.drop_anchor = DropAnchor::OnTop;
                            } else if cursor_pos.y > deflated_bounds.y() + deflated_bounds.h() {
                                self.drop_anchor = DropAnchor::Side {
                                    visual_offset: bounds.h() - 1.0,
                                    index_offset: 0,
                                };
                            }
                        } else {
                            self.drop_anchor = DropAnchor::OnTop;
                        }
                    }
                }
                WidgetMessage::MouseLeave => {
                    self.drop_anchor = DropAnchor::OnTop;
                }
                _ => (),
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
    entity_handle: ErasedHandle,
    name: String,
    icon: Option<UntypedResource>,
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

    pub fn with_entity_handle(mut self, entity_handle: ErasedHandle) -> Self {
        self.entity_handle = entity_handle;
        self
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.name = name;
        self
    }

    pub fn with_icon(mut self, icon: Option<UntypedResource>) -> Self {
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
                            .with_margin(Thickness::left_right(1.0))
                            .with_visibility(self.icon.is_some()),
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
                            .with_margin(Thickness::left(1.0))
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
            drop_anchor: DropAnchor::OnTop,
        };

        ctx.add_node(UiNode::new(item))
    }
}
