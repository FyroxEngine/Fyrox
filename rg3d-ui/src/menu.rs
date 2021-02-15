use crate::core::algebra::Vector2;
use crate::{
    border::BorderBuilder,
    brush::Brush,
    core::{color::Color, pool::Handle},
    decorator::DecoratorBuilder,
    grid::{Column, GridBuilder, Row},
    message::{
        ButtonState, MenuItemMessage, MenuMessage, OsEvent, PopupMessage, UiMessage, UiMessageData,
        WidgetMessage,
    },
    message::{MessageData, MessageDirection},
    node::UINode,
    popup::{Placement, PopupBuilder},
    stack_panel::StackPanelBuilder,
    text::TextBuilder,
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, HorizontalAlignment, NodeHandleMapping, Orientation, RestrictionEntry,
    Thickness, UserInterface, VerticalAlignment, BRUSH_BRIGHT_BLUE, BRUSH_PRIMARY,
};
use std::{
    ops::{Deref, DerefMut},
    rc::Rc,
};

#[derive(Clone)]
pub struct Menu<M: MessageData, C: Control<M, C>> {
    widget: Widget<M, C>,
    active: bool,
}

crate::define_widget_deref!(Menu<M, C>);

impl<M: MessageData, C: Control<M, C>> Control<M, C> for Menu<M, C> {
    fn handle_routed_message(
        &mut self,
        ui: &mut UserInterface<M, C>,
        message: &mut UiMessage<M, C>,
    ) {
        self.widget.handle_routed_message(ui, message);

        if let UiMessageData::Menu(msg) = &message.data() {
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
                            if let UINode::MenuItem(item) = node {
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
        _self_handle: Handle<UINode<M, C>>,
        ui: &mut UserInterface<M, C>,
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
                        if let UINode::MenuItem(item) = node {
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
pub struct MenuItem<M: MessageData, C: Control<M, C>> {
    widget: Widget<M, C>,
    items: Vec<Handle<UINode<M, C>>>,
    popup: Handle<UINode<M, C>>,
    placement: MenuItemPlacement,
}

crate::define_widget_deref!(MenuItem<M, C>);

// MenuItem uses popup to show its content, popup can be top-most only if it is
// direct child of root canvas of UI. This fact adds some complications to search
// of parent menu - we can't just traverse the tree because popup is not a child
// of menu item, instead we trying to fetch handle to parent menu item from popup's
// user data and continue up-search until we find menu.
fn find_menu<M: MessageData, C: Control<M, C>>(
    from: Handle<UINode<M, C>>,
    ui: &UserInterface<M, C>,
) -> Handle<UINode<M, C>> {
    let mut handle = from;
    loop {
        let popup = ui.find_by_criteria_up(handle, |n| matches!(n, UINode::Popup(_)));
        if popup.is_none() {
            // Maybe we have Menu as parent for MenuItem.
            return ui.find_by_criteria_up(handle, |n| matches!(n, UINode::Menu(_)));
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

impl<M: MessageData, C: Control<M, C>> Control<M, C> for MenuItem<M, C> {
    fn resolve(&mut self, node_map: &NodeHandleMapping<M, C>) {
        node_map.resolve_slice(&mut self.items);
        node_map.resolve(&mut self.popup);
    }

    fn handle_routed_message(
        &mut self,
        ui: &mut UserInterface<M, C>,
        message: &mut UiMessage<M, C>,
    ) {
        self.widget.handle_routed_message(ui, message);

        match &message.data() {
            UiMessageData::Widget(msg) => {
                match msg {
                    WidgetMessage::MouseDown { .. } => {
                        let menu = find_menu(self.parent(), ui);
                        if menu.is_some() {
                            // Activate menu so it user will be able to open submenus by
                            // mouse hover.
                            ui.send_message(MenuMessage::activate(
                                menu,
                                MessageDirection::ToWidget,
                            ));

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
                                    ui.send_message(MenuMessage::deactivate(
                                        menu,
                                        MessageDirection::ToWidget,
                                    ));
                                }
                            }
                            message.set_handled(true);
                        }
                    }
                    WidgetMessage::MouseEnter => {
                        // While parent menu active it is possible to open submenus
                        // by simple mouse hover.
                        let menu = find_menu(self.parent(), ui);
                        if menu.is_some() {
                            if let UINode::Menu(menu) = ui.node(menu) {
                                if menu.active {
                                    ui.send_message(MenuItemMessage::open(
                                        self.handle(),
                                        MessageDirection::ToWidget,
                                    ));
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
                                    self.screen_position + Vector2::new(0.0, self.actual_size().y)
                                }
                                MenuItemPlacement::Right => {
                                    self.screen_position + Vector2::new(self.actual_size().x, 0.0)
                                }
                            };

                            // Open popup.
                            ui.send_message(PopupMessage::placement(
                                self.popup,
                                MessageDirection::ToWidget,
                                Placement::Position(position),
                            ));
                            ui.send_message(PopupMessage::open(
                                self.popup,
                                MessageDirection::ToWidget,
                            ));
                        }
                    }
                    MenuItemMessage::Close => {
                        ui.send_message(PopupMessage::close(
                            self.popup,
                            MessageDirection::ToWidget,
                        ));
                    }
                    MenuItemMessage::Click => {}
                }
            }
            _ => {}
        }
    }

    fn preview_message(&self, ui: &UserInterface<M, C>, message: &mut UiMessage<M, C>) {
        // We need to check if some new menu item opened and then close other not in
        // direct chain of menu items until to menu.
        if message.destination() != self.handle() {
            if let UiMessageData::MenuItem(MenuItemMessage::Open) = &message.data() {
                let mut found = false;
                let mut handle = message.destination();
                while handle.is_some() {
                    if handle == self.handle() {
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
                    ui.send_message(MenuItemMessage::close(
                        self.handle(),
                        MessageDirection::ToWidget,
                    ));
                }
            }
        }
    }
}

pub struct MenuBuilder<M: MessageData, C: Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
    items: Vec<Handle<UINode<M, C>>>,
}

impl<M: MessageData, C: Control<M, C>> MenuBuilder<M, C> {
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

    pub fn build(self, ctx: &mut BuildContext<M, C>) -> Handle<UINode<M, C>> {
        for &item in self.items.iter() {
            if let UINode::MenuItem(item) = &mut ctx[item] {
                item.placement = MenuItemPlacement::Bottom;
            }
        }

        let back = BorderBuilder::new(
            WidgetBuilder::new()
                .with_background(BRUSH_PRIMARY)
                .with_child(
                    StackPanelBuilder::new(WidgetBuilder::new().with_children(&self.items))
                        .with_orientation(Orientation::Horizontal)
                        .build(ctx),
                ),
        )
        .build(ctx);

        let menu = Menu {
            widget: self.widget_builder.with_child(back).build(),
            active: false,
        };

        ctx.add_node(UINode::Menu(menu))
    }
}

pub enum MenuItemContent<'a, 'b, M: MessageData, C: Control<M, C>> {
    /// Empty menu item.
    None,
    /// Quick-n-dirty way of building elements. It can cover most of use
    /// cases - it builds classic menu item:
    ///   _____________________
    ///  |    |      |        |
    ///  |icon| text |shortcut|
    ///  |____|______|________|
    Text {
        text: &'a str,
        shortcut: &'b str,
        icon: Handle<UINode<M, C>>,
    },
    /// Allows to put any node into menu item. It allows to customize menu
    /// item how needed - i.e. put image in it, or other user control.
    Node(Handle<UINode<M, C>>),
}

impl<'a, 'b, M: MessageData, C: Control<M, C>> MenuItemContent<'a, 'b, M, C> {
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

pub struct MenuItemBuilder<'a, 'b, M: MessageData, C: Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
    items: Vec<Handle<UINode<M, C>>>,
    content: MenuItemContent<'a, 'b, M, C>,
    back: Option<Handle<UINode<M, C>>>,
}

impl<'a, 'b, M: MessageData, C: Control<M, C>> MenuItemBuilder<'a, 'b, M, C> {
    pub fn new(widget_builder: WidgetBuilder<M, C>) -> Self {
        Self {
            widget_builder,
            items: Default::default(),
            content: MenuItemContent::None,
            back: None,
        }
    }

    pub fn with_content(mut self, content: MenuItemContent<'a, 'b, M, C>) -> Self {
        self.content = content;
        self
    }

    pub fn with_items(mut self, items: Vec<Handle<UINode<M, C>>>) -> Self {
        self.items = items;
        self
    }

    /// Allows you to specify the background content. Background node is only for decoration purpose,
    /// it can be any kind of node, by default it is Decorator.
    pub fn with_back(mut self, handle: Handle<UINode<M, C>>) -> Self {
        self.back = Some(handle);
        self
    }

    pub fn build(self, ctx: &mut BuildContext<M, C>) -> Handle<UINode<M, C>> {
        let content = match self.content {
            MenuItemContent::None => Handle::NONE,
            MenuItemContent::Text {
                text,
                shortcut,
                icon,
            } => GridBuilder::new(
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
            MenuItemContent::Node(node) => node,
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
                StackPanelBuilder::new(WidgetBuilder::new().with_children(&self.items)).build(ctx),
            )
            // We'll manually control if popup is either open or closed.
            .stays_open(true)
            .build(ctx);

        let menu = MenuItem {
            widget: self.widget_builder.with_child(back).build(),
            popup,
            items: self.items,
            placement: MenuItemPlacement::Right,
        };

        let handle = ctx.add_node(UINode::MenuItem(menu));

        // "Link" popup with its parent menu item.
        if let UINode::Popup(popup) = &mut ctx[popup] {
            popup.user_data = Some(Rc::new(handle));
        }

        handle
    }
}
