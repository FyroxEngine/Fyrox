use crate::{
    core::{
        algebra::Vector2, math::Rect, pool::Handle, reflect::prelude::*, uuid_provider,
        visitor::prelude::*,
    },
    message::UiMessage,
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, UiNode, UserInterface,
};
use std::{
    any::{Any, TypeId},
    cell::Cell,
    ops::{Deref, DerefMut},
    sync::mpsc::Sender,
};

#[derive(Default, Clone, Visit, Reflect, Debug)]
pub struct Screen {
    /// Base widget of the screen.
    pub widget: Widget,
    #[visit(skip)]
    #[reflect(hidden)]
    pub last_screen_size: Cell<Vector2<f32>>,
}

crate::define_widget_deref!(Screen);

uuid_provider!(Screen = "3bc7649f-a1ba-49be-bc4e-e0624654e40c");

impl Control for Screen {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<Self>() {
            Some(self)
        } else {
            None
        }
    }

    fn measure_override(&self, ui: &UserInterface, _available_size: Vector2<f32>) -> Vector2<f32> {
        for &child in self.children.iter() {
            ui.measure_node(child, ui.screen_size());
        }

        ui.screen_size()
    }

    fn arrange_override(&self, ui: &UserInterface, _final_size: Vector2<f32>) -> Vector2<f32> {
        let final_rect = Rect::new(0.0, 0.0, ui.screen_size().x, ui.screen_size().y);

        for &child in self.children.iter() {
            ui.arrange_node(child, &final_rect);
        }

        ui.screen_size()
    }

    fn update(&mut self, _dt: f32, _sender: &Sender<UiMessage>, screen_size: Vector2<f32>) {
        if self.last_screen_size.get() != screen_size {
            self.invalidate_layout();
            self.last_screen_size.set(screen_size);
        }
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);
    }
}

pub struct ScreenBuilder {
    widget_builder: WidgetBuilder,
}

impl ScreenBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self { widget_builder }
    }

    pub fn build(self, ui: &mut BuildContext) -> Handle<UiNode> {
        let screen = Screen {
            widget: self.widget_builder.build(),
            last_screen_size: Cell::new(Default::default()),
        };
        ui.add_node(UiNode::new(screen))
    }
}
