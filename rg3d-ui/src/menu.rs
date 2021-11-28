use crate::{
    border::BorderBuilder,
    brush::Brush,
    core::{algebra::Vector2, color::Color, pool::Handle},
    decorator::DecoratorBuilder,
    define_constructor,
    grid::{Column, GridBuilder, Row},
    message::{ButtonState, MessageDirection, OsEvent, UiMessage},
    popup::{Placement, Popup, PopupBuilder, PopupMessage},
    stack_panel::StackPanelBuilder,
    text::TextBuilder,
    widget::{Widget, WidgetBuilder, WidgetMessage},
    BuildContext, Control, HorizontalAlignment, NodeHandleMapping, Orientation, RestrictionEntry,
    Thickness, UiNode, UserInterface, VerticalAlignment, BRUSH_BRIGHT_BLUE, BRUSH_PRIMARY,
};
use std::sync::mpsc::Sender;
use std::{
    any::{Any, TypeId},
    ops::{Deref, DerefMut},
    rc::Rc,
};

#[derive(Debug, Clone, PartialEq)]
pub enum MenuMessage {
    Activate,
    Deactivate,
}

impl MenuMessage {
    define_constructor!(MenuMessage:Activate => fn activate(), layout: false);
    define_constructor!(MenuMessage:Deactivate => fn deactivate(), layout: false);
}

#[derive(Debug, Clone, PartialEq)]
pub enum MenuItemMessage {
    Open,
    Close,
    Click,
}

impl MenuItemMessage {
    define_constructor!(MenuItemMessage:Open => fn open(), layout: false);
    define_constructor!(MenuItemMessage:Close => fn close(), layout: false);
    define_constructor!(MenuItemMessage:Click => fn click(), layout: false);
}

#[derive(Clone)]
pub struct Menu {
    widget: Widget,
    active: bool,
}

crate::define_widget_deref!(Menu);

impl Control for Menu {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<Self>() {
            Some(self)
        } else {
            None
        }
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(msg) = message.data::<MenuMessage>() {
            match msg {
                MenuMessage::Activate => {
                    if !self.active {
                        ui.push_picking_restriction(RestrictionEntry {
                            handle: self.handle(),
                            stop: false,
                        });
                        self.active = true;
                    }
                }
                MenuMessage::Deactivate => {
                    if self.active {
                        self.active = false;
                        ui.remove_picking_restriction(self.handle());

                        // Close descendant menu items.
                        let mut stack = self.children().to_vec();
                        while let Some(handle) = stack.pop() {
                            let node = ui.node(handle);
                            if let Some(item) = node.cast::<MenuItem>() {
                                ui.send_message(MenuItemMessage::close(
                                    handle,
                                    MessageDirection::ToWidget,
                                ));
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
    }

    fn handle_os_event(
        &mut self,
        _self_handle: Handle<UiNode>,
        ui: &mut UserInterface,
        event: &OsEvent,
    ) {
        // Handle menu items close by clicking outside of menu item. We using
        // raw event here because we need to know the fact that mouse was clicked
        // and we do not care which element was clicked so we'll get here in any
        // case.
        if let OsEvent::MouseInput { state, .. } = event {
            if *state == ButtonState::Pressed && self.active {
                // TODO: Make picking more accurate - right now it works only with rects.
                let pos = ui.cursor_position();
                if !self.widget.screen_bounds().contains(pos) {
                    // Also check if we clicked inside some descendant menu item - in this
                    // case we don't need to close menu.
                    let mut any_picked = false;
                    let mut stack = self.children().to_vec();
                    'depth_search: while let Some(handle) = stack.pop() {
                        let node = ui.node(handle);
                        if let Some(item) = node.cast::<MenuItem>() {
                            if ui.node(item.popup).screen_bounds().contains(pos) {
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
                        ui.send_message(MenuMessage::deactivate(
                            self.handle(),
                            MessageDirection::ToWidget,
                        ));
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

#[derive(Clone)]
pub struct MenuItem {
    widget: Widget,
    items: Vec<Handle<UiNode>>,
    popup: Handle<UiNode>,
    placement: MenuItemPlacement,
}

crate::define_widget_deref!(MenuItem);

// MenuItem uses popup to show its content, popup can be top-most only if it is
// direct child of root canvas of UI. This fact adds some complications to search
// of parent menu - we can't just traverse the tree because popup is not a child
// of menu item, instead we trying to fetch handle to parent menu item from popup's
// user data and continue up-search until we find menu.
fn find_menu(from: Handle<UiNode>, ui: &UserInterface) -> Handle<UiNode> {
    let mut handle = from;
    while handle.is_some() {
        if let Some((_, popup)) = ui.try_borrow_by_type_up::<Popup>(handle) {
            // Continue search from parent menu item of popup.
            handle = popup
                .user_data_ref::<Handle<UiNode>>()
                .cloned()
                .unwrap_or_default();
        } else {
            // Maybe we have Menu as parent for MenuItem.
            return ui.find_by_criteria_up(handle, |n| n.cast::<Menu>().is_some());
        }
    }
    Default::default()
}

fn close_menu_chain(from: Handle<UiNode>, ui: &UserInterface) {
    let mut handle = from;
    while handle.is_some() {
        if let Some((popup_handle, popup)) = ui.try_borrow_by_type_up::<Popup>(handle) {
            ui.send_message(PopupMessage::close(
                popup_handle,
                MessageDirection::ToWidget,
            ));

            // Continue search from parent menu item of popup.
            handle = popup
                .user_data_ref::<Handle<UiNode>>()
                .cloned()
                .unwrap_or_default();
        }
    }
}

impl Control for MenuItem {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<Self>() {
            Some(self)
        } else {
            None
        }
    }

    fn on_remove(&self, sender: &Sender<UiMessage>) {
        // Popup won't be deleted with the menu item, because it is not the child of the item.
        // So we have to remove it manually.
        sender
            .send(WidgetMessage::remove(
                self.popup,
                MessageDirection::ToWidget,
            ))
            .unwrap();
    }

    fn resolve(&mut self, node_map: &NodeHandleMapping) {
        node_map.resolve_slice(&mut self.items);
        node_map.resolve(&mut self.popup);
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(msg) = message.data::<WidgetMessage>() {
            match msg {
                WidgetMessage::MouseDown { .. } => {
                    let menu = find_menu(self.parent(), ui);
                    if menu.is_some() {
                        // Activate menu so it user will be able to open submenus by
                        // mouse hover.
                        ui.send_message(MenuMessage::activate(menu, MessageDirection::ToWidget));

                        ui.send_message(MenuItemMessage::open(
                            self.handle(),
                            MessageDirection::ToWidget,
                        ));
                    }
                }
                WidgetMessage::MouseUp { .. } => {
                    if !message.handled() {
                        ui.send_message(MenuItemMessage::click(
                            self.handle(),
                            MessageDirection::ToWidget,
                        ));
                        if self.items.is_empty() {
                            let menu = find_menu(self.parent(), ui);
                            if menu.is_some() {
                                // Deactivate menu if we have one.
                                ui.send_message(MenuMessage::deactivate(
                                    menu,
                                    MessageDirection::ToWidget,
                                ));
                            } else {
                                // Or close menu chain if menu item is in "orphaned" state.
                                close_menu_chain(self.parent(), ui);
                            }
                        }
                        message.set_handled(true);
                    }
                }
                WidgetMessage::MouseEnter => {
                    // While parent menu active it is possible to open submenus
                    // by simple mouse hover.
                    let menu = find_menu(self.parent(), ui);
                    let open = if menu.is_some() {
                        if let Some(menu) = ui.node(menu).cast::<Menu>() {
                            menu.active
                        } else {
                            false
                        }
                    } else {
                        true
                    };
                    if open {
                        ui.send_message(MenuItemMessage::open(
                            self.handle(),
                            MessageDirection::ToWidget,
                        ));
                    }
                }
                _ => {}
            }
        } else if let Some(msg) = message.data::<MenuItemMessage>() {
            match msg {
                MenuItemMessage::Open => {
                    if !self.items.is_empty() {
                        let placement = match self.placement {
                            MenuItemPlacement::Bottom => Placement::LeftBottom(self.handle),
                            MenuItemPlacement::Right => Placement::RightTop(self.handle),
                        };

                        // Open popup.
                        ui.send_message(PopupMessage::placement(
                            self.popup,
                            MessageDirection::ToWidget,
                            placement,
                        ));
                        ui.send_message(PopupMessage::open(self.popup, MessageDirection::ToWidget));
                    }
                }
                MenuItemMessage::Close => {
                    ui.send_message(PopupMessage::close(self.popup, MessageDirection::ToWidget));
                }
                MenuItemMessage::Click => {}
            }
        }
    }

    fn preview_message(&self, ui: &UserInterface, message: &mut UiMessage) {
        // We need to check if some new menu item opened and then close other not in
        // direct chain of menu items until to menu.
        if message.destination() != self.handle() {
            if let Some(MenuItemMessage::Open) = message.data::<MenuItemMessage>() {
                let mut found = false;
                let mut handle = message.destination();
                while handle.is_some() {
                    if handle == self.handle() {
                        found = true;
                        break;
                    } else {
                        let node = ui.node(handle);
                        if let Some(popup) = node.cast::<Popup>() {
                            // Once we found popup in chain, we must extract handle
                            // of parent menu item to continue search.
                            handle = popup
                                .user_data_ref::<Handle<UiNode>>()
                                .cloned()
                                .unwrap_or_default();
                        } else {
                            handle = node.parent();
                        }
                    }
                }

                if !found {
                    ui.send_message(MenuItemMessage::close(
                        self.handle(),
                        MessageDirection::ToWidget,
                    ));
                }
            }
        }
    }
}

pub struct MenuBuilder {
    widget_builder: WidgetBuilder,
    items: Vec<Handle<UiNode>>,
}

impl MenuBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            items: Default::default(),
        }
    }

    pub fn with_items(mut self, items: Vec<Handle<UiNode>>) -> Self {
        self.items = items;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        for &item in self.items.iter() {
            if let Some(item) = ctx[item].cast_mut::<MenuItem>() {
                item.placement = MenuItemPlacement::Bottom;
            }
        }

        let back = BorderBuilder::new(
            WidgetBuilder::new()
                .with_background(BRUSH_PRIMARY)
                .with_child(
                    StackPanelBuilder::new(
                        WidgetBuilder::new().with_children(self.items.iter().cloned()),
                    )
                    .with_orientation(Orientation::Horizontal)
                    .build(ctx),
                ),
        )
        .build(ctx);

        let menu = Menu {
            widget: self
                .widget_builder
                .with_handle_os_events(true)
                .with_child(back)
                .build(),
            active: false,
        };

        ctx.add_node(UiNode::new(menu))
    }
}

pub enum MenuItemContent<'a, 'b> {
    /// Quick-n-dirty way of building elements. It can cover most of use
    /// cases - it builds classic menu item:
    ///   _____________________
    ///  |    |      |        |
    ///  |icon| text |shortcut|
    ///  |____|______|________|
    Text {
        text: &'a str,
        shortcut: &'b str,
        icon: Handle<UiNode>,
    },
    /// Allows to put any node into menu item. It allows to customize menu
    /// item how needed - i.e. put image in it, or other user control.
    Node(Handle<UiNode>),
}

impl<'a, 'b> MenuItemContent<'a, 'b> {
    pub fn text_with_shortcut(text: &'a str, shortcut: &'b str) -> Self {
        MenuItemContent::Text {
            text,
            shortcut,
            icon: Default::default(),
        }
    }

    pub fn text(text: &'a str) -> Self {
        MenuItemContent::Text {
            text,
            shortcut: "",
            icon: Default::default(),
        }
    }
}

pub struct MenuItemBuilder<'a, 'b> {
    widget_builder: WidgetBuilder,
    items: Vec<Handle<UiNode>>,
    content: Option<MenuItemContent<'a, 'b>>,
    back: Option<Handle<UiNode>>,
}

impl<'a, 'b> MenuItemBuilder<'a, 'b> {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            items: Default::default(),
            content: None,
            back: None,
        }
    }

    pub fn with_content(mut self, content: MenuItemContent<'a, 'b>) -> Self {
        self.content = Some(content);
        self
    }

    pub fn with_items(mut self, items: Vec<Handle<UiNode>>) -> Self {
        self.items = items;
        self
    }

    /// Allows you to specify the background content. Background node is only for decoration purpose,
    /// it can be any kind of node, by default it is Decorator.
    pub fn with_back(mut self, handle: Handle<UiNode>) -> Self {
        self.back = Some(handle);
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let content = match self.content {
            None => Handle::NONE,
            Some(MenuItemContent::Text {
                text,
                shortcut,
                icon,
            }) => GridBuilder::new(
                WidgetBuilder::new()
                    .with_child(icon)
                    .with_child(
                        TextBuilder::new(
                            WidgetBuilder::new()
                                .with_vertical_alignment(VerticalAlignment::Center)
                                .with_margin(Thickness::uniform(1.0))
                                .on_column(1),
                        )
                        .with_text(text)
                        .build(ctx),
                    )
                    .with_child(
                        TextBuilder::new(
                            WidgetBuilder::new()
                                .with_vertical_alignment(VerticalAlignment::Center)
                                .with_horizontal_alignment(HorizontalAlignment::Right)
                                .with_margin(Thickness::uniform(1.0))
                                .on_column(2),
                        )
                        .with_text(shortcut)
                        .build(ctx),
                    ),
            )
            .add_row(Row::auto())
            .add_column(Column::auto())
            .add_column(Column::stretch())
            .add_column(Column::auto())
            .build(ctx),
            Some(MenuItemContent::Node(node)) => node,
        };

        let back = self.back.unwrap_or_else(|| {
            DecoratorBuilder::new(
                BorderBuilder::new(WidgetBuilder::new())
                    .with_stroke_thickness(Thickness::uniform(0.0)),
            )
            .with_hover_brush(BRUSH_BRIGHT_BLUE)
            .with_normal_brush(BRUSH_PRIMARY)
            .with_pressed_brush(Brush::Solid(Color::TRANSPARENT))
            .with_pressable(false)
            .build(ctx)
        });

        ctx.link(content, back);

        let popup = PopupBuilder::new(WidgetBuilder::new().with_min_size(Vector2::new(10.0, 10.0)))
            .with_content(
                StackPanelBuilder::new(
                    WidgetBuilder::new().with_children(self.items.iter().cloned()),
                )
                .build(ctx),
            )
            // We'll manually control if popup is either open or closed.
            .stays_open(true)
            .build(ctx);

        let menu = MenuItem {
            widget: self
                .widget_builder
                .with_preview_messages(true)
                .with_child(back)
                .build(),
            popup,
            items: self.items,
            placement: MenuItemPlacement::Right,
        };

        let handle = ctx.add_node(UiNode::new(menu));

        // "Link" popup with its parent menu item.
        if let Some(popup) = ctx[popup].cast_mut::<Popup>() {
            popup.user_data = Some(Rc::new(handle));
        }

        handle
    }
}
