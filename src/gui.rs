use crate::{
    asset::AssetItem,
    world_outliner::SceneItem
};
use std::ops::{
    Deref,
    DerefMut
};

use rg3d::{
    core::{
        math::Rect,
        math::vec2::Vec2,
        pool::Handle,
    },
    gui::{
        Control,
        message::{
            UiMessageData,
            OsEvent,
        },
        draw::DrawingContext,
        NodeHandleMapping,
    },
};

#[derive(Debug)]
pub enum AssetItemMessage {
    Select(bool)
}

#[derive(Debug)]
pub enum SceneItemMessage {
    NodeVisibility(bool)
}

#[derive(Debug)]
pub enum EditorUiMessage {
    AssetItem(AssetItemMessage),
    SceneItem(SceneItemMessage)
}

impl SceneItemMessage {
    pub fn node_visibility(destination: Handle<UiNode>, visibility: bool) -> UiMessage {
        UiMessage {
            handled: false,
            data: UiMessageData::User(EditorUiMessage::SceneItem(SceneItemMessage::NodeVisibility(visibility))),
            destination
        }
    }
}

impl AssetItemMessage {
    pub fn select(destination: Handle<UiNode>, select: bool) -> UiMessage {
        UiMessage {
            handled: false,
            data: UiMessageData::User(EditorUiMessage::AssetItem(AssetItemMessage::Select(select))),
            destination
        }
    }
}

pub type CustomWidget = rg3d::gui::widget::Widget<EditorUiMessage, EditorUiNode>;
pub type UiNode = rg3d::gui::node::UINode<EditorUiMessage, EditorUiNode>;
pub type Ui = rg3d::gui::UserInterface<EditorUiMessage, EditorUiNode>;
pub type UiMessage = rg3d::gui::message::UiMessage<EditorUiMessage, EditorUiNode>;
pub type BuildContext<'a> = rg3d::gui::BuildContext<'a, EditorUiMessage, EditorUiNode>;
pub type UiWidgetBuilder = rg3d::gui::widget::WidgetBuilder<EditorUiMessage, EditorUiNode>;

#[derive(Debug)]
pub enum EditorUiNode {
    AssetItem(AssetItem),
    SceneItem(SceneItem)
}

macro_rules! static_dispatch {
    ($self:ident, $func:ident, $($args:expr),*) => {
        match $self {
            EditorUiNode::AssetItem(v) => v.$func($($args),*),
            EditorUiNode::SceneItem(v) => v.$func($($args),*),
        }
    }
}

macro_rules! static_dispatch_deref {
    ($self:ident) => {
        match $self {
            EditorUiNode::AssetItem(v) => v,
            EditorUiNode::SceneItem(v) => v
        }
    }
}

impl Deref for EditorUiNode {
    type Target = CustomWidget;

    fn deref(&self) -> &Self::Target {
        static_dispatch_deref!(self)
    }
}

impl DerefMut for EditorUiNode {
    fn deref_mut(&mut self) -> &mut Self::Target {
        static_dispatch_deref!(self)
    }
}

impl Control<EditorUiMessage, EditorUiNode> for EditorUiNode {
    fn raw_copy(&self) -> UiNode {
        static_dispatch!(self, raw_copy,)
    }

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

