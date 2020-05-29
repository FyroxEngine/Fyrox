use std::{
    ops::{
        DerefMut,
        Deref,
    },
    rc::Rc,
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
        ButtonState,
        MenuMessage,
        MenuItemMessage,
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
        color::Color,
    },
};

pub struct Menu<M: 'static, C: 'static + Control<M, C>> {
    widget: Widget<M, C>,
    active: bool,
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
            widget: self.widget.raw_copy(),
            active: self.active,
        }
    }
}

impl<M, C: 'static + Control<M, C>> Control<M, C> for Menu<M, C> {
    fn raw_copy(&self) -> UINode<M, C> {
        UINode::Menu(self.clone())
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface<M, C>, message: &mut UiMessage<M, C>) {
        self.widget.handle_routed_message(ui, message);

        match &message.data {
            UiMessageData::Menu(msg) => {
                match msg {
                    MenuMessage::Activate => {
                        if !self.active {
                            ui.push_picking_restriction(self.handle);
                            self.active = true;
                        }
                    }
                    MenuMessage::Deactivate => {
                        if self.active {
                            self.active = false;
                            ui.remove_picking_restriction(self.handle);

                            // Close descendant menu items.
                            let mut stack = self.children().to_vec();
                            while let Some(handle) = stack.pop() {
                                let node = ui.node(handle);
                                if let UINode::MenuItem(item) = node {
                                    ui.send_message(UiMessage {
                                        handled: false,
                                        data: UiMessageData::MenuItem(MenuItemMessage::Close),
                                        destination: handle,
                                    });
                                    // We have to search in popup content too because menu shows its content
                                    // in popup and content could be another menu item.
                                    stack.push(item.popup);
                                }
                                // Continue depth search.
                                stack.extend_from_slice(node.children());
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_os_event(&mut self, _self_handle: Handle<UINode<M, C>>, ui: &mut UserInterface<M, C>, event: &OsEvent) {
        // Handle menu items close by clicking outside of menu item. We using
        // raw event here because we need to know the fact that mouse was clicked
        // and we do not care which element was clicked so we'll get here in any
        // case.
        if let OsEvent::MouseInput { state, .. } = event {
            if *state == ButtonState::Pressed && self.active {
                // TODO: Make picking more accurate - right now it works only with rects.
                let pos = ui.cursor_position();
                if !self.widget.screen_bounds().contains(pos.x, pos.y) {
                    // Also check if we clicked inside some descendant menu item - in this
                    // case we don't need to close menu.
                    let mut any_picked = false;
                    let mut stack = self.children().to_vec();
                    'depth_search: while let Some(handle) = stack.pop() {
                        let node = ui.node(handle);
                        if let UINode::MenuItem(item) = node {
                            if ui.node(item.popup).screen_bounds().contains(pos.x, pos.y) {
                                // Once we found that we clicked inside some descendant menu item
                                // we can immediately stop search - we don't want to close menu
                                // items popups in this case and can safely skip all stuff below.
                                any_picked = true;
                                break 'depth_search;
                            }
                            // We have to search in popup content too because menu shows its content
                            // in popup and content could be another menu item.
                            stack.push(item.popup);
                        }
                        // Continue depth search.
                        stack.extend_from_slice(node.children());
                    }

                    if !any_picked {
                        ui.send_message(UiMessage {
                            handled: false,
                            data: UiMessageData::Menu(MenuMessage::Deactivate),
                            destination: self.handle,
                        });
                    }
                }
            }
        }
    }
}

#[derive(Copy, Clone, PartialOrd, PartialEq, Hash)]
enum MenuItemPlacement {
    Bottom,
    Right,
}

pub struct MenuItem<M: 'static, C: 'static + Control<M, C>> {
    widget: Widget<M, C>,
    items: Vec<Handle<UINode<M, C>>>,
    popup: Handle<UINode<M, C>>,
    back: Handle<UINode<M, C>>,
    placement: MenuItemPlacement,
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
            back: self.back,
            placement: MenuItemPlacement::Right,
        }
    }
}

// MenuItem uses popup to show its content, popup can be top-most only if it is
// direct child of root canvas of UI. This fact adds some complications to search
// of parent menu - we can't just traverse the tree because popup is not a child
// of menu item, instead we trying to fetch handle to parent menu item from popup's
// user data and continue up-search until we find menu.
fn find_menu<M, C: 'static + Control<M, C>>(from: Handle<UINode<M, C>>, ui: &UserInterface<M, C>) -> Handle<UINode<M, C>> {
    let mut handle = from;
    loop {
        let popup = ui.find_by_criteria_up(handle, |n| {
            if let UINode::Popup(_) = n { true } else { false }
        });
        if popup.is_none() {
            // Maybe we have Menu as parent for MenuItem.
            return ui.find_by_criteria_up(handle, |n| {
                if let UINode::Menu(_) = n { true } else { false }
            });
        } else {
            // Continue search from parent menu item of popup.
            if let UINode::Popup(popup) = ui.node(popup) {
                handle = *popup.user_data_ref::<Handle<UINode<M, C>>>();
            } else {
                unreachable!();
            }
        }
    }
}

impl<M, C: 'static + Control<M, C>> Control<M, C> for MenuItem<M, C> {
    fn raw_copy(&self) -> UINode<M, C> {
        UINode::MenuItem(self.clone())
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface<M, C>, message: &mut UiMessage<M, C>) {
        self.widget.handle_routed_message(ui, message);

        match &message.data {
            UiMessageData::Widget(msg) => {
                match msg {
                    WidgetMessage::MouseDown { .. } => {
                        let menu = find_menu(self.parent(), ui);
                        if menu.is_some() {

                            // Activate menu so it user will be able to open submenus by
                            // mouse hover.
                            ui.send_message(UiMessage {
                                handled: false,
                                data: UiMessageData::Menu(MenuMessage::Activate),
                                destination: menu,
                            });

                            ui.send_message(UiMessage {
                                handled: false,
                                data: UiMessageData::MenuItem(MenuItemMessage::Open),
                                destination: self.handle,
                            });
                        }
                    }
                    WidgetMessage::MouseLeave => {
                        ui.node_mut(self.back).set_background(Brush::Solid(Color::opaque(50, 50, 50)));
                    }
                    WidgetMessage::MouseEnter => {
                        ui.node_mut(self.back).set_background(Brush::Solid(Color::opaque(130, 130, 130)));

                        // While parent menu active it is possible to open submenus
                        // by simple mouse hover.
                        let menu = find_menu(self.parent(), ui);
                        if menu.is_some() {
                            if let UINode::Menu(menu) = ui.node(menu) {
                                if menu.active {
                                    ui.send_message(UiMessage {
                                        handled: false,
                                        data: UiMessageData::MenuItem(MenuItemMessage::Open),
                                        destination: self.handle,
                                    });
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            UiMessageData::MenuItem(msg) => {
                match msg {
                    MenuItemMessage::Open => {
                        if !self.items.is_empty() {
                            let position = match self.placement {
                                MenuItemPlacement::Bottom => {
                                    self.screen_position + Vec2::new(0.0, self.actual_size().y)
                                }
                                MenuItemPlacement::Right => {
                                    self.screen_position + Vec2::new(self.actual_size().x, 0.0)
                                }
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
                    MenuItemMessage::Close => {
                        ui.send_message(UiMessage {
                            handled: false,
                            data: UiMessageData::Popup(PopupMessage::Close),
                            destination: self.popup,
                        });
                    }
                }
            }
            _ => {}
        }
    }

    fn preview_message(&mut self, ui: &mut UserInterface<M, C>, message: &mut UiMessage<M, C>) {
        // We need to check if some new menu item opened and then close other not in
        // direct chain of menu items until to menu.
        if message.destination != self.handle {
            if let UiMessageData::MenuItem(msg) = &message.data {
                if let MenuItemMessage::Open = msg {
                    let mut found = false;
                    let mut handle = message.destination;
                    while handle.is_some() {
                        if handle == self.handle {
                            found = true;
                            break;
                        } else {
                            let node = ui.node(handle);
                            if let UINode::Popup(popup) = node {
                                // Once we found popup in chain, we must extract handle
                                // of parent menu item to continue search.
                                handle = *popup.user_data_ref::<Handle<UINode<M, C>>>();
                            } else {
                                handle = node.parent();
                            }
                        }
                    }

                    if !found {
                        ui.send_message(UiMessage {
                            handled: false,
                            data: UiMessageData::MenuItem(MenuItemMessage::Close),
                            destination: self.handle,
                        });
                    }
                }
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
                .build(ui.sender()),
            active: false,
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

        let popup = PopupBuilder::new(WidgetBuilder::new()
            .with_min_size(Vec2::new(10.0, 10.0)))
            .with_content(StackPanelBuilder::new(WidgetBuilder::new()
                .with_children(&self.items))
                .build(ui))
            // We'll control if popup is either open or closed manually.
            .stays_open(true)
            .build(ui);

        let menu = MenuItem {
            widget: self.widget_builder
                .with_child(back)
                .build(ui.sender()),
            popup,
            items: self.items,
            back,
            placement: MenuItemPlacement::Right,
        };

        let handle = ui.add_node(UINode::MenuItem(menu));

        ui.flush_messages();

        // "Link" popup with its parent menu item.
        if let UINode::Popup(popup) = ui.node_mut(popup) {
            popup.user_data = Some(Rc::new(handle));
        }

        handle
    }
}