use crate::load_image;
use rg3d::{
    asset::core::algebra::Vector2,
    core::{color::Color, pool::Handle},
    gui::{
        brush::Brush,
        define_constructor,
        draw::DrawingContext,
        grid::{Column, GridBuilder, Row},
        image::ImageBuilder,
        message::{MessageDirection, OsEvent, UiMessage},
        text::{TextBuilder, TextMessage},
        tree::{Tree, TreeBuilder},
        widget::{Widget, WidgetBuilder},
        BuildContext, Control, NodeHandleMapping, Thickness, UiNode, UserInterface,
        VerticalAlignment,
    },
};
use std::{
    any::{Any, TypeId},
    ops::{Deref, DerefMut},
    sync::mpsc::Sender,
};

pub mod menu;

#[derive(Debug)]
pub struct LinkItem<S, D> {
    pub tree: Tree,
    text: Handle<UiNode>,
    pub source: Handle<S>,
    pub dest: Handle<D>,
}

impl<S, D> Clone for LinkItem<S, D> {
    fn clone(&self) -> Self {
        Self {
            tree: self.tree.clone(),
            text: self.text,
            source: self.source,
            dest: self.dest,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum LinkItemMessage {
    Name(String),
}

impl LinkItemMessage {
    define_constructor!(LinkItemMessage:Name => fn name(String), layout: false);
}

impl<S, D> Deref for LinkItem<S, D> {
    type Target = Widget;

    fn deref(&self) -> &Self::Target {
        &self.tree
    }
}

impl<S, D> DerefMut for LinkItem<S, D> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.tree
    }
}

impl<S: 'static, D: 'static> Control for LinkItem<S, D> {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        self.tree.query_component(type_id).or_else(|| {
            if type_id == TypeId::of::<Self>() {
                Some(self)
            } else {
                None
            }
        })
    }

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

    fn update(&mut self, _dt: f32, sender: &Sender<UiMessage>) {
        self.tree.update(_dt, sender)
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.tree.handle_routed_message(ui, message);

        if let Some(LinkItemMessage::Name(name)) = message.data::<LinkItemMessage>() {
            ui.send_message(TextMessage::text(
                self.text,
                MessageDirection::ToWidget,
                make_item_name(name, self.source, self.dest),
            ));
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
}

pub struct LinkItemBuilder<S, D> {
    tree_builder: TreeBuilder,
    name: String,
    source: Handle<S>,
    dest: Handle<D>,
}

fn make_item_name<S, D>(name: &str, source: Handle<S>, dest: Handle<D>) -> String {
    format!(
        "{} ({}:{}) - ({}:{})",
        name,
        source.index(),
        source.generation(),
        dest.index(),
        dest.generation()
    )
}

impl<S: 'static, D: 'static> LinkItemBuilder<S, D> {
    pub fn new(tree_builder: TreeBuilder) -> Self {
        Self {
            tree_builder,
            name: Default::default(),
            source: Default::default(),
            dest: Default::default(),
        }
    }

    pub fn with_name<N: AsRef<str>>(mut self, name: N) -> Self {
        self.name = name.as_ref().to_owned();
        self
    }

    pub fn with_source(mut self, source: Handle<S>) -> Self {
        self.source = source;
        self
    }

    pub fn with_dest(mut self, dest: Handle<D>) -> Self {
        self.dest = dest;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let text;
        let grid = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(
                    ImageBuilder::new(
                        WidgetBuilder::new()
                            .on_column(0)
                            .with_width(16.0)
                            .with_height(16.0),
                    )
                    .with_opt_texture(load_image(include_bytes!(
                        "../../../resources/embed/link.png"
                    )))
                    .build(ctx),
                )
                .with_child({
                    text = TextBuilder::new(
                        WidgetBuilder::new()
                            .with_margin(Thickness::left(2.0))
                            .on_column(1)
                            .with_margin(Thickness::uniform(1.0))
                            .with_foreground(Brush::Solid(Color::opaque(34, 177, 76))),
                    )
                    .with_vertical_text_alignment(VerticalAlignment::Center)
                    .with_text(make_item_name(&self.name, self.source, self.dest))
                    .build(ctx);
                    text
                }),
        )
        .add_column(Column::auto())
        .add_column(Column::stretch())
        .add_row(Row::stretch())
        .build(ctx);

        let node = LinkItem {
            tree: self.tree_builder.with_content(grid).build_tree(ctx),
            text,
            source: self.source,
            dest: self.dest,
        };

        ctx.add_node(UiNode::new(node))
    }
}
