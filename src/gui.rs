use crate::{asset::AssetItem, world_outliner::SceneItem};
use std::ops::{Deref, DerefMut};

use rg3d::gui::message::{MessageData, MessageDirection};
use rg3d::{
    core::{math::vec2::Vec2, math::Rect, pool::Handle},
    gui::{draw::DrawingContext, message::OsEvent, Control, NodeHandleMapping},
};

#[derive(Debug, Clone, PartialEq)]
pub enum AssetItemMessage {
    Select(bool),
}

#[derive(Debug, Clone, PartialEq)]
pub enum SceneItemMessage {
    NodeVisibility(bool),
}

#[derive(Debug, Clone, PartialEq)]
pub enum EditorUiMessage {
    AssetItem(AssetItemMessage),
    SceneItem(SceneItemMessage),
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

#[derive(Debug, Clone)]
pub enum EditorUiNode {
    AssetItem(AssetItem),
    SceneItem(SceneItem),
}

macro_rules! static_dispatch {
    ($self:ident, $func:ident, $($args:expr),*) => {
        match $self {
            EditorUiNode::AssetItem(v) => v.$func($($args),*),
            EditorUiNode::SceneItem(v) => v.$func($($args),*),
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

    fn measure_override(&self, ui: &Ui, available_size: Vec2) -> Vec2 {
        static_dispatch!(self, measure_override, ui, available_size)
    }

    fn arrange_override(&self, ui: &Ui, final_size: Vec2) -> Vec2 {
        static_dispatch!(self, arrange_override, ui, final_size)
    }

    fn arrange(&self, ui: &Ui, final_rect: &Rect<f32>) {
        static_dispatch!(self, arrange, ui, final_rect)
    }

    fn measure(&self, ui: &Ui, available_size: Vec2) {
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

    fn preview_message(&mut self, ui: &mut Ui, message: &mut UiMessage) {
        static_dispatch!(self, preview_message, ui, message)
    }

    fn handle_os_event(&mut self, self_handle: Handle<UiNode>, ui: &mut Ui, event: &OsEvent) {
        static_dispatch!(self, handle_os_event, self_handle, ui, event)
    }

    fn remove_ref(&mut self, handle: Handle<UiNode>) {
        static_dispatch!(self, remove_ref, handle)
    }
}
