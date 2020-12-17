use crate::{asset::AssetItem, load_image, world_outliner::SceneItem};
use rg3d::{
    core::{algebra::Vector2, math::Rect, pool::Handle},
    engine::resource_manager::ResourceManager,
    gui::{
        border::BorderBuilder,
        button::ButtonBuilder,
        decorator::DecoratorBuilder,
        draw::DrawingContext,
        grid::{Column, GridBuilder, Row},
        image::ImageBuilder,
        message::{ButtonMessage, MessageData, MessageDirection, OsEvent, UiMessageData},
        widget::WidgetBuilder,
        Control, NodeHandleMapping, Thickness,
    },
};
use std::ops::{Deref, DerefMut};

#[derive(Debug, Clone, PartialEq)]
pub enum AssetItemMessage {
    Select(bool),
}

#[derive(Debug, Clone, PartialEq)]
pub enum SceneItemMessage {
    NodeVisibility(bool),
    Name(String),
    /// Odd or even.
    Order(bool),
}

#[derive(Debug, Clone, PartialEq)]
pub enum EditorUiMessage {
    AssetItem(AssetItemMessage),
    SceneItem(SceneItemMessage),
    EmitterItem(DeletableItemMessage),
}

impl MessageData for EditorUiMessage {}

impl SceneItemMessage {
    pub fn node_visibility(destination: Handle<UiNode>, visibility: bool) -> UiMessage {
        UiMessage::user(
            destination,
            MessageDirection::ToWidget,
            EditorUiMessage::SceneItem(SceneItemMessage::NodeVisibility(visibility)),
        )
    }

    pub fn name(destination: Handle<UiNode>, name: String) -> UiMessage {
        UiMessage::user(
            destination,
            MessageDirection::ToWidget,
            EditorUiMessage::SceneItem(SceneItemMessage::Name(name)),
        )
    }
}

impl AssetItemMessage {
    pub fn select(destination: Handle<UiNode>, select: bool) -> UiMessage {
        UiMessage::user(
            destination,
            MessageDirection::ToWidget,
            EditorUiMessage::AssetItem(AssetItemMessage::Select(select)),
        )
    }
}

pub type CustomWidget = rg3d::gui::widget::Widget<EditorUiMessage, EditorUiNode>;
pub type UiNode = rg3d::gui::node::UINode<EditorUiMessage, EditorUiNode>;
pub type Ui = rg3d::gui::UserInterface<EditorUiMessage, EditorUiNode>;
pub type UiMessage = rg3d::gui::message::UiMessage<EditorUiMessage, EditorUiNode>;
pub type BuildContext<'a> = rg3d::gui::BuildContext<'a, EditorUiMessage, EditorUiNode>;
pub type UiWidgetBuilder = rg3d::gui::widget::WidgetBuilder<EditorUiMessage, EditorUiNode>;

#[derive(Debug, Clone, PartialEq)]
pub enum DeletableItemMessage {
    Delete,
}

/// An item that has content and a button to request deletion.
#[derive(Debug, Clone)]
pub struct DeletableItem<D: Clone> {
    widget: CustomWidget,
    pub delete: Handle<UiNode>,
    pub data: Option<D>,
}

impl<D: Clone> Deref for DeletableItem<D> {
    type Target = CustomWidget;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<D: Clone> DerefMut for DeletableItem<D> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl<D: Clone + 'static> Control<EditorUiMessage, EditorUiNode> for DeletableItem<D> {
    fn resolve(&mut self, node_map: &NodeHandleMapping<EditorUiMessage, EditorUiNode>) {
        node_map.resolve(&mut self.delete);
    }

    fn handle_routed_message(&mut self, ui: &mut Ui, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let UiMessageData::Button(msg) = message.data() {
            if let ButtonMessage::Click = msg {
                if message.destination() == self.delete {
                    ui.send_message(UiMessage::user(
                        self.handle(),
                        MessageDirection::ToWidget,
                        EditorUiMessage::EmitterItem(DeletableItemMessage::Delete),
                    ));
                }
            }
        }
    }
}

pub struct DeletableItemBuilder<D> {
    widget_builder: UiWidgetBuilder,
    content: Handle<UiNode>,
    data: Option<D>,
}

impl<D: Clone + 'static> DeletableItemBuilder<D> {
    pub fn new(widget_builder: UiWidgetBuilder) -> Self {
        Self {
            widget_builder,
            content: Handle::NONE,
            data: None,
        }
    }

    pub fn with_data(mut self, data: D) -> Self {
        self.data = Some(data);
        self
    }

    pub fn with_content(mut self, content: Handle<UiNode>) -> Self {
        self.content = content;
        self
    }

    pub fn build(
        self,
        ctx: &mut BuildContext,
        resource_manager: ResourceManager,
    ) -> DeletableItem<D> {
        let delete;
        DeletableItem {
            widget: self
                .widget_builder
                .with_child(
                    DecoratorBuilder::new(BorderBuilder::new(
                        WidgetBuilder::new().with_child(
                            GridBuilder::new(
                                WidgetBuilder::new().with_child(self.content).with_child({
                                    delete = ButtonBuilder::new(WidgetBuilder::new().on_column(1))
                                        .with_content(
                                            ButtonBuilder::new(
                                                WidgetBuilder::new()
                                                    .with_width(16.0)
                                                    .with_height(16.0)
                                                    .with_margin(Thickness::uniform(1.0)),
                                            )
                                            .with_content(
                                                ImageBuilder::new(WidgetBuilder::new())
                                                    .with_opt_texture(load_image(
                                                        "resources/cross.png",
                                                        resource_manager,
                                                    ))
                                                    .build(ctx),
                                            )
                                            .build(ctx),
                                        )
                                        .build(ctx);
                                    delete
                                }),
                            )
                            .add_column(Column::stretch())
                            .add_column(Column::strict(16.0))
                            .add_row(Row::stretch())
                            .build(ctx),
                        ),
                    ))
                    .build(ctx),
                )
                .build(),
            delete,
            data: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum EditorUiNode {
    AssetItem(AssetItem),
    SceneItem(SceneItem),
    EmitterItem(DeletableItem<usize>),
}

macro_rules! static_dispatch {
    ($self:ident, $func:ident, $($args:expr),*) => {
        match $self {
            EditorUiNode::AssetItem(v) => v.$func($($args),*),
            EditorUiNode::SceneItem(v) => v.$func($($args),*),
            EditorUiNode::EmitterItem(v) => v.$func($($args),*),
        }
    }
}

impl Deref for EditorUiNode {
    type Target = CustomWidget;

    fn deref(&self) -> &Self::Target {
        static_dispatch!(self, deref,)
    }
}

impl DerefMut for EditorUiNode {
    fn deref_mut(&mut self) -> &mut Self::Target {
        static_dispatch!(self, deref_mut,)
    }
}

impl Control<EditorUiMessage, EditorUiNode> for EditorUiNode {
    fn resolve(&mut self, node_map: &NodeHandleMapping<EditorUiMessage, EditorUiNode>) {
        static_dispatch!(self, resolve, node_map);
    }

    fn measure_override(&self, ui: &Ui, available_size: Vector2<f32>) -> Vector2<f32> {
        static_dispatch!(self, measure_override, ui, available_size)
    }

    fn arrange_override(&self, ui: &Ui, final_size: Vector2<f32>) -> Vector2<f32> {
        static_dispatch!(self, arrange_override, ui, final_size)
    }

    fn arrange(&self, ui: &Ui, final_rect: &Rect<f32>) {
        static_dispatch!(self, arrange, ui, final_rect)
    }

    fn measure(&self, ui: &Ui, available_size: Vector2<f32>) {
        static_dispatch!(self, measure, ui, available_size)
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        static_dispatch!(self, draw, drawing_context)
    }

    fn update(&mut self, dt: f32) {
        static_dispatch!(self, update, dt)
    }

    fn handle_routed_message(&mut self, ui: &mut Ui, message: &mut UiMessage) {
        static_dispatch!(self, handle_routed_message, ui, message)
    }

    fn preview_message(&self, ui: &Ui, message: &mut UiMessage) {
        static_dispatch!(self, preview_message, ui, message)
    }

    fn handle_os_event(&mut self, self_handle: Handle<UiNode>, ui: &mut Ui, event: &OsEvent) {
        static_dispatch!(self, handle_os_event, self_handle, ui, event)
    }

    fn remove_ref(&mut self, handle: Handle<UiNode>) {
        static_dispatch!(self, remove_ref, handle)
    }
}
