use std::ops::{
    DerefMut,
    Deref,
};
use crate::{
    brush::Brush,
    border::BorderBuilder,
    popup::{PopupBuilder, Placement},
    message::{
        UiMessageData,
        WidgetMessage,
        PopupMessage,
        UiMessage,
        OsEvent,
        ButtonState
    },
    stack_panel::StackPanelBuilder,
    node::UINode,
    Control,
    UserInterface,
    Orientation,
    widget::{Widget, WidgetBuilder},
    core::{
        pool::Handle,
        math::vec2::Vec2,
        color::Color
    }
};

pub struct Menu<M: 'static, C: 'static + Control<M, C>> {
    widget: Widget<M, C>,
}

impl<M: 'static, C: 'static + Control<M, C>> Deref for Menu<M, C> {
    type Target = Widget<M, C>;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<M: 'static, C: 'static + Control<M, C>> DerefMut for Menu<M, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl<M: 'static, C: 'static + Control<M, C>> Clone for Menu<M, C> {
    fn clone(&self) -> Self {
        Self {
            widget: self.widget.raw_copy()
        }
    }
}

impl<M, C: 'static + Control<M, C>> Control<M, C> for Menu<M, C> {
    fn raw_copy(&self) -> UINode<M, C> {
        UINode::Menu(self.clone())
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface<M, C>, message: &mut UiMessage<M, C>) {
        self.widget.handle_routed_message(ui, message);

        if let UiMessageData::Widget(msg) = &message.data {
            if let WidgetMessage::MouseDown { .. } = msg {
                if ui.top_picking_restriction() != self.handle {
                    ui.push_picking_restriction(self.handle);
                }
            }
        }
    }

    fn handle_os_event(&mut self, self_handle: Handle<UINode<M, C>>, ui: &mut UserInterface<M, C>, event: &OsEvent) {
        if let OsEvent::MouseInput { state, .. } = event {
            if *state == ButtonState::Pressed && ui.top_picking_restriction() == self_handle {
                let pos = ui.cursor_position();
                if !self.widget.screen_bounds().contains(pos.x, pos.y) {
                    ui.pop_picking_restriction();
                }
            }
        }
    }
}

#[derive(Copy, Clone, PartialOrd, PartialEq, Hash)]
enum MenuItemPlacement {
    Bottom,
    Right
}

pub struct MenuItem<M: 'static, C: 'static + Control<M, C>> {
    widget: Widget<M, C>,
    items: Vec<Handle<UINode<M, C>>>,
    popup: Handle<UINode<M, C>>,
    active: bool,
    back: Handle<UINode<M, C>>,
    placement: MenuItemPlacement
}

impl<M: 'static, C: 'static + Control<M, C>> Deref for MenuItem<M, C> {
    type Target = Widget<M, C>;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<M: 'static, C: 'static + Control<M, C>> DerefMut for MenuItem<M, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl<M: 'static, C: 'static + Control<M, C>> Clone for MenuItem<M, C> {
    fn clone(&self) -> Self {
        Self {
            widget: self.widget.raw_copy(),
            items: self.items.clone(),
            popup: self.popup,
            active: false,
            back: self.back,
            placement: MenuItemPlacement::Right
        }
    }
}

impl<M, C: 'static + Control<M, C>> Control<M, C> for MenuItem<M, C> {
    fn raw_copy(&self) -> UINode<M, C> {
        UINode::MenuItem(self.clone())
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface<M, C>, message: &mut UiMessage<M, C>) {
        self.widget.handle_routed_message(ui, message);

        if let UiMessageData::Widget(msg) = &message.data {
            match msg {
                WidgetMessage::MouseLeave => {
                    ui.node_mut(self.back).set_background(Brush::Solid(Color::opaque(50,50,50)));
                }
                WidgetMessage::MouseEnter | WidgetMessage::MouseDown { .. } => {
                    ui.node_mut(self.back).set_background(Brush::Solid(Color::opaque(130,130,130)));
                    // Close other popups.

                    self.active = true;

                    if self.popup.is_some() {
                        let position = match self.placement {
                            MenuItemPlacement::Bottom => {
                                self.screen_position + Vec2::new(0.0, self.actual_size().y)
                            },
                            MenuItemPlacement::Right => {
                                self.screen_position + Vec2::new(self.actual_size().x, 0.0)
                            },
                        };

                        // Open popup.
                        ui.send_message(UiMessage {
                            handled: false,
                            data: UiMessageData::Popup(PopupMessage::Placement(Placement::Position(position))),
                            destination: self.popup,
                        });
                        ui.send_message(UiMessage {
                            handled: false,
                            data: UiMessageData::Popup(PopupMessage::Open),
                            destination: self.popup,
                        });
                    }
                }
                _ => {}
            }
        }
    }
}

pub struct MenuBuilder<M: 'static, C: 'static + Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
    items: Vec<Handle<UINode<M, C>>>,
}

impl<M, C: 'static + Control<M, C>> MenuBuilder<M, C> {
    pub fn new(widget_builder: WidgetBuilder<M, C>) -> Self {
        Self {
            widget_builder,
            items: Default::default(),
        }
    }

    pub fn with_items(mut self, items: Vec<Handle<UINode<M, C>>>) -> Self {
        self.items = items;
        self
    }

    pub fn build(self, ui: &mut UserInterface<M, C>) -> Handle<UINode<M, C>> {
        for &item in &self.items {
            if let UINode::MenuItem(item) = ui.node_mut(item) {
                item.placement = MenuItemPlacement::Bottom;
            }
        }

        let back = BorderBuilder::new(WidgetBuilder::new()
            .with_child(StackPanelBuilder::new(WidgetBuilder::new()
                .with_children(&self.items))
                .with_orientation(Orientation::Horizontal)
                .build(ui)))
            .build(ui);

        let menu = Menu {
            widget: self.widget_builder
                .with_child(back)
                .build(ui.sender())
        };

        let handle = ui.add_node(UINode::Menu(menu));

        ui.flush_messages();

        handle
    }
}

pub struct MenuItemBuilder<M: 'static, C: 'static + Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
    items: Vec<Handle<UINode<M, C>>>,
    content: Handle<UINode<M, C>>,
}

impl<M, C: 'static + Control<M, C>> MenuItemBuilder<M, C> {
    pub fn new(widget_builder: WidgetBuilder<M, C>) -> Self {
        Self {
            widget_builder,
            items: Default::default(),
            content: Default::default(),
        }
    }

    pub fn with_content(mut self, content: Handle<UINode<M, C>>) -> Self {
        self.content = content;
        self
    }

    pub fn with_items(mut self, items: Vec<Handle<UINode<M, C>>>) -> Self {
        self.items = items;
        self
    }

    pub fn build(self, ui: &mut UserInterface<M, C>) -> Handle<UINode<M, C>> {
        let back = BorderBuilder::new(WidgetBuilder::new()
            .with_child(self.content))
            .build(ui);

        let menu = MenuItem {
            widget: self.widget_builder
                .with_child(back)
                .build(ui.sender()),
            popup: if self.items.is_empty() {
                Handle::NONE
            } else {
                PopupBuilder::new(WidgetBuilder::new()
                    .with_min_size(Vec2::new(10.0, 10.0)))
                    .with_content(StackPanelBuilder::new(WidgetBuilder::new()
                        .with_children(&self.items))
                        .build(ui))
                    .build(ui)
            },
            items: self.items,
            active: false,
            back,
            placement: MenuItemPlacement::Right
        };

        let handle = ui.add_node(UINode::MenuItem(menu));

        ui.flush_messages();

        handle
    }
}