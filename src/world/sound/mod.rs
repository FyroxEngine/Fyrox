use crate::utils;
use rg3d::{
    asset::core::algebra::Vector2,
    core::{algebra::Vector3, pool::Handle},
    gui::{
        draw::DrawingContext,
        message::{MessageDirection, OsEvent, TextMessage, UiMessage, UiMessageData},
        text::TextBuilder,
        tree::{Tree, TreeBuilder},
        widget::Widget,
        widget::WidgetBuilder,
        BuildContext, Control, NodeHandleMapping, UiNode, UserInterface,
    },
    sound::{context::SoundContext, source::SoundSource},
};
use std::ops::{Deref, DerefMut};

#[derive(Debug, Clone)]
pub struct SoundSelection {
    pub sources: Vec<Handle<SoundSource>>,
}

impl SoundSelection {
    pub fn sources(&self) -> &[Handle<SoundSource>] {
        &self.sources
    }

    pub fn is_single_selection(&self) -> bool {
        self.sources.len() == 1
    }

    pub fn first(&self) -> Option<Handle<SoundSource>> {
        self.sources.first().cloned()
    }

    pub fn center(&self, sound_context: &SoundContext) -> Option<Vector3<f32>> {
        let state = sound_context.state();
        let mut count = 0;
        let position_sum = self
            .sources
            .iter()
            .filter_map(|&handle| match state.source(handle) {
                SoundSource::Generic(_) => None,
                SoundSource::Spatial(spatial) => Some(spatial.position()),
            })
            .fold(Vector3::default(), |acc, source_position| {
                count += 1;
                acc + source_position
            });
        if count > 0 {
            Some(position_sum.scale(1.0 / count as f32))
        } else {
            None
        }
    }
}

impl PartialEq for SoundSelection {
    fn eq(&self, other: &Self) -> bool {
        utils::is_slice_equal_permutation(self.sources(), other.sources())
    }
}

impl Eq for SoundSelection {}

#[derive(Clone, Debug)]
pub struct SoundItem {
    pub tree: Tree,
    text: Handle<UiNode>,
    pub sound_source: Handle<SoundSource>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SoundItemMessage {
    Name(String),
}

impl Deref for SoundItem {
    type Target = Widget;

    fn deref(&self) -> &Self::Target {
        &self.tree
    }
}

impl DerefMut for SoundItem {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.tree
    }
}

impl Control for SoundItem {
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
            if let Some(SoundItemMessage::Name(name)) = msg.cast::<SoundItemMessage>() {
                ui.send_message(TextMessage::text(
                    self.text,
                    MessageDirection::ToWidget,
                    make_item_name(name, self.sound_source),
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

pub struct SoundItemBuilder {
    tree_builder: TreeBuilder,
    name: String,
    sound_source: Handle<SoundSource>,
}

fn make_item_name(name: &str, handle: Handle<SoundSource>) -> String {
    format!("{} ({}:{})", name, handle.index(), handle.generation())
}

impl SoundItemBuilder {
    pub fn new(tree_builder: TreeBuilder) -> Self {
        Self {
            tree_builder,
            name: Default::default(),
            sound_source: Default::default(),
        }
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.name = name;
        self
    }

    pub fn with_sound_source(mut self, source: Handle<SoundSource>) -> Self {
        self.sound_source = source;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let text = TextBuilder::new(WidgetBuilder::new())
            .with_text(make_item_name(&self.name, self.sound_source))
            .build(ctx);

        let node = SoundItem {
            tree: self.tree_builder.with_content(text).build_tree(ctx),
            text,
            sound_source: self.sound_source,
        };

        ctx.add_node(UiNode::new(node))
    }
}
