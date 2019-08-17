use crate::{
    math::vec2::Vec2,
    gui::{
        UserInterface,
        node::UINode,
    },
    utils::pool::Handle,
};
use glutin::{
    MouseButton,
    VirtualKeyCode,
};

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum RoutedEventHandlerType {
    MouseMove,
    MouseEnter,
    MouseLeave,
    MouseDown,
    MouseUp,
    MouseWheel,
    Count,
}

pub type RoutedEventHandler = dyn FnMut(&mut UserInterface, Handle<UINode>, &mut RoutedEvent);

pub type RoutedEventHandlerList = [Option<Box<RoutedEventHandler>>; RoutedEventHandlerType::Count as usize];

pub enum RoutedEventKind {
    MouseDown {
        pos: Vec2,
        button: MouseButton,
    },
    MouseUp {
        pos: Vec2,
        button: MouseButton,
    },
    MouseMove {
        pos: Vec2
    },
    Text {
        symbol: char
    },
    KeyDown {
        code: VirtualKeyCode
    },
    KeyUp {
        code: VirtualKeyCode
    },
    MouseWheel {
        pos: Vec2,
        amount: f32,
    },
    MouseLeave,
    MouseEnter,
}

pub struct RoutedEvent {
    pub kind: RoutedEventKind,
    pub handled: bool,
}

impl RoutedEvent {
    pub fn new(kind: RoutedEventKind) -> RoutedEvent {
        RoutedEvent {
            kind,
            handled: false,
        }
    }
}