use fyrox::{
    core::{algebra::Vector2, pool::Handle},
    gui::{
        define_constructor,
        message::{MessageDirection, UiMessage},
        UiNode,
    },
};

#[derive(Debug, Clone, PartialEq)]
pub enum SegmentMessage {
    SourcePosition(Vector2<f32>),
    DestPosition(Vector2<f32>),
}

impl SegmentMessage {
    define_constructor!(Self:SourcePosition => fn source_position(Vector2<f32>), layout: false);
    define_constructor!(Self:DestPosition => fn dest_position(Vector2<f32>), layout: false);
}

#[derive(Debug, Clone)]
pub struct Segment {
    pub source: Handle<UiNode>,
    pub source_pos: Vector2<f32>,
    pub dest: Handle<UiNode>,
    pub dest_pos: Vector2<f32>,
}

impl Segment {
    pub fn handle_routed_message(&mut self, self_handle: Handle<UiNode>, message: &mut UiMessage) {
        if let Some(msg) = message.data::<SegmentMessage>() {
            if message.destination() == self_handle
                && message.direction() == MessageDirection::ToWidget
            {
                match msg {
                    SegmentMessage::SourcePosition(pos) => {
                        self.source_pos = *pos;
                    }
                    SegmentMessage::DestPosition(pos) => {
                        self.dest_pos = *pos;
                    }
                }
            }
        }
    }
}
