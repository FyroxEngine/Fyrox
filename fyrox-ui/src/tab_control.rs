// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! The Tab Control handles the visibility of several tabs, only showing a single tab that the user has selected via the
//! tab header buttons. See docs for [`TabControl`] widget for more info and usage examples.

#![warn(missing_docs)]

use crate::style::resource::StyleResourceExt;
use crate::style::{Style, StyledProperty};
use crate::{
    border::BorderBuilder,
    brush::Brush,
    button::{ButtonBuilder, ButtonMessage},
    core::{
        algebra::Vector2, color::Color, pool::Handle, reflect::prelude::*, type_traits::prelude::*,
        uuid_provider, visitor::prelude::*,
    },
    decorator::{DecoratorBuilder, DecoratorMessage},
    define_constructor,
    grid::{Column, GridBuilder, Row},
    message::{ButtonState, MessageDirection, MouseButton, UiMessage},
    utils::make_cross_primitive,
    vector_image::VectorImageBuilder,
    widget::{Widget, WidgetBuilder, WidgetMessage},
    wrap_panel::WrapPanelBuilder,
    BuildContext, Control, HorizontalAlignment, Orientation, Thickness, UiNode, UserInterface,
    VerticalAlignment,
};

use fyrox_core::variable::InheritableVariable;
use fyrox_graph::constructor::{ConstructorProvider, GraphNodeConstructor};
use fyrox_graph::BaseSceneGraph;
use std::{
    any::Any,
    cmp::Ordering,
    fmt::{Debug, Formatter},
    ops::{Deref, DerefMut},
    sync::Arc,
};

/// A set of messages for [`TabControl`] widget.
#[derive(Debug, Clone, PartialEq)]
pub enum TabControlMessage {
    /// Used to change the active tab of a [`TabControl`] widget (with [`MessageDirection::ToWidget`]) or to fetch if the active
    /// tab has changed (with [`MessageDirection::FromWidget`]).
    /// When the active tab changes, `ActiveTabUuid` will also be sent from the widget.
    /// When the active tab changes, `ActiveTabUuid` will also be sent from the widget.
    ActiveTab(Option<usize>),
    /// Used to change the active tab of a [`TabControl`] widget (with [`MessageDirection::ToWidget`]) or to fetch if the active
    /// tab has changed (with [`MessageDirection::FromWidget`]).
    /// When the active tab changes, `ActiveTab` will also be sent from the widget.
    ActiveTabUuid(Option<Uuid>),
    /// Emitted by a tab that needs to be closed (and removed). Does **not** remove the tab, its main usage is to catch the moment
    /// when the tab wants to be closed. To remove the tab use [`TabControlMessage::RemoveTab`] message.
    CloseTab(usize),
    /// Emitted by a tab that needs to be closed (and removed). Does **not** remove the tab, its main usage is to catch the moment
    /// when the tab wants to be closed. To remove the tab use [`TabControlMessage::RemoveTab`] message.
    CloseTabByUuid(Uuid),
    /// Used to remove a particular tab by its position in the tab list.
    RemoveTab(usize),
    /// Used to remove a particular tab by its UUID.
    RemoveTabByUuid(Uuid),
    /// Adds a new tab using its definition and activates the tab.
    AddTab {
        /// The UUID of the newly created tab.
        uuid: Uuid,
        /// The specifications for the tab.
        definition: TabDefinition,
    },
}

impl TabControlMessage {
    define_constructor!(
        /// Creates [`TabControlMessage::ActiveTab`] message.
        TabControlMessage:ActiveTab => fn active_tab(Option<usize>), layout: false
    );
    define_constructor!(
        /// Creates [`TabControlMessage::ActiveTabUuid`] message.
        TabControlMessage:ActiveTabUuid => fn active_tab_uuid(Option<Uuid>), layout: false
    );
    define_constructor!(
        /// Creates [`TabControlMessage::CloseTab`] message.
        TabControlMessage:CloseTab => fn close_tab(usize), layout: false
    );
    define_constructor!(
        /// Creates [`TabControlMessage::CloseTabByUuid`] message.
        TabControlMessage:CloseTabByUuid => fn close_tab_by_uuid(Uuid), layout: false
    );
    define_constructor!(
        /// Creates [`TabControlMessage::RemoveTab`] message.
        TabControlMessage:RemoveTab => fn remove_tab(usize), layout: false
    );
    define_constructor!(
        /// Creates [`TabControlMessage::RemoveTabByUuid`] message.
        TabControlMessage:RemoveTabByUuid => fn remove_tab_by_uuid(Uuid), layout: false
    );
    define_constructor!(
        /// Creates [`TabControlMessage::AddTab`] message.
        TabControlMessage:AddTab => fn add_tab_with_uuid(uuid: Uuid, definition: TabDefinition), layout: false
    );
    /// Creates [`TabControlMessage::AddTab`] message with a random UUID.
    pub fn add_tab(
        destination: Handle<UiNode>,
        direction: MessageDirection,
        definition: TabDefinition,
    ) -> UiMessage {
        UiMessage {
            handled: std::cell::Cell::new(false),
            data: Box::new(Self::AddTab {
                uuid: Uuid::new_v4(),
                definition,
            }),
            destination,
            direction,
            routing_strategy: Default::default(),
            perform_layout: std::cell::Cell::new(false),
            flags: 0,
        }
    }
}

/// User-defined data of a tab.
#[derive(Clone)]
pub struct TabUserData(pub Arc<dyn Any + Send + Sync>);

impl TabUserData {
    /// Creates new instance of the tab data.
    pub fn new<T>(data: T) -> Self
    where
        T: Any + Send + Sync,
    {
        Self(Arc::new(data))
    }
}

impl PartialEq for TabUserData {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(
            (&*self.0) as *const _ as *const (),
            (&*other.0) as *const _ as *const (),
        )
    }
}

impl Debug for TabUserData {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "User-defined data")
    }
}

/// Tab of the [`TabControl`] widget. It stores important tab data, that is widely used at runtime.
#[derive(Default, Clone, PartialEq, Visit, Reflect, Debug)]
pub struct Tab {
    /// Unique identifier of this tab.
    pub uuid: Uuid,
    /// A handle of the header button, that is used to switch tabs.
    pub header_button: Handle<UiNode>,
    /// Tab's content.
    pub content: Handle<UiNode>,
    /// A handle of a button, that is used to close the tab.
    pub close_button: Handle<UiNode>,
    /// A handle to a container widget, that holds the header.
    pub header_container: Handle<UiNode>,
    /// User-defined data.
    #[visit(skip)]
    #[reflect(hidden)]
    pub user_data: Option<TabUserData>,
    /// A handle of a node that is used to highlight tab's state.
    pub decorator: Handle<UiNode>,
    /// Content of the tab-switching (header) button.
    pub header_content: Handle<UiNode>,
}

/// The Tab Control handles the visibility of several tabs, only showing a single tab that the user has selected via the
/// tab header buttons. Each tab is defined via a Tab Definition struct which takes two widgets, one representing the tab
/// header and the other representing the tab's contents.
///
/// The following example makes a 2 tab, Tab Control containing some simple text widgets:
///
/// ```rust,no_run
/// # use fyrox_ui::{
/// #     BuildContext,
/// #     widget::WidgetBuilder,
/// #     text::TextBuilder,
/// #     tab_control::{TabControlBuilder, TabDefinition},
/// # };
/// fn create_tab_control(ctx: &mut BuildContext) {
///     TabControlBuilder::new(WidgetBuilder::new())
///         .with_tab(
///             TabDefinition{
///                 header: TextBuilder::new(WidgetBuilder::new())
///                             .with_text("First")
///                             .build(ctx),
///
///                 content: TextBuilder::new(WidgetBuilder::new())
///                             .with_text("First tab's contents!")
///                             .build(ctx),
///                 can_be_closed: true,
///                 user_data: None
///             }
///         )
///         .with_tab(
///             TabDefinition{
///                 header: TextBuilder::new(WidgetBuilder::new())
///                             .with_text("Second")
///                             .build(ctx),
///
///                 content: TextBuilder::new(WidgetBuilder::new())
///                             .with_text("Second tab's contents!")
///                             .build(ctx),
///                 can_be_closed: true,
///                 user_data: None
///             }
///         )
///         .build(ctx);
/// }
/// ```
///
/// As usual, we create the widget via the builder TabControlBuilder. Tabs are added via the [`TabControlBuilder::with_tab`]
/// function in the order you want them to appear, passing each call to the function a directly constructed [`TabDefinition`]
/// struct. Tab headers will appear from left to right at the top with tab contents shown directly below the tabs. As usual, if no
/// constraints are given to the base [`WidgetBuilder`] of the [`TabControlBuilder`], then the tab content area will resize to fit
/// whatever is in the current tab.
///
/// Each tab's content is made up of one widget, so to be useful you will want to use one of the container widgets to help
/// arrange additional widgets within the tab.
///
/// ## Tab Header Styling
///
/// Notice that you can put any widget into the tab header, so if you want images to denote each tab you can add an Image
/// widget to each header, and if you want an image *and* some text you can insert a stack panel with an image on top and
/// text below it.
///
/// You will also likely want to style whatever widgets you add. As can be seen when running the code example above, the
/// tab headers are scrunched when there are no margins provided to your text widgets. Simply add something like the below
/// code example and you will get a decent look:
///
/// ```rust,no_run
/// # use fyrox_ui::{
/// #     BuildContext,
/// #     widget::WidgetBuilder,
/// #     text::TextBuilder,
/// #     Thickness,
/// #     tab_control::{TabDefinition},
/// # };
/// # fn build(ctx: &mut BuildContext) {
/// # TabDefinition{
/// header: TextBuilder::new(
///             WidgetBuilder::new()
///                 .with_margin(Thickness::uniform(4.0))
///         )
///             .with_text("First")
///             .build(ctx),
/// # content: Default::default(),
/// # can_be_closed: true,
/// # user_data: None
/// # };
/// # }
///
/// ```
#[derive(Default, Clone, Visit, Reflect, Debug, ComponentProvider)]
#[reflect(derived_type = "UiNode")]
pub struct TabControl {
    /// Base widget of the tab control.
    pub widget: Widget,
    /// True if the user permitted to change the order of the tabs.
    pub is_tab_drag_allowed: bool,
    /// A set of tabs used by the tab control.
    pub tabs: Vec<Tab>,
    /// Active tab of the tab control.
    pub active_tab: Option<usize>,
    /// A handle of a widget, that holds content of every tab.
    pub content_container: Handle<UiNode>,
    /// A handle of a widget, that holds headers of every tab.
    pub headers_container: Handle<UiNode>,
    /// A brush, that will be used to highlight active tab.
    pub active_tab_brush: InheritableVariable<StyledProperty<Brush>>,
}

impl ConstructorProvider<UiNode, UserInterface> for TabControl {
    fn constructor() -> GraphNodeConstructor<UiNode, UserInterface> {
        GraphNodeConstructor::new::<Self>()
            .with_variant("Tab Control", |ui| {
                TabControlBuilder::new(WidgetBuilder::new().with_name("Tab Control"))
                    .build(&mut ui.build_ctx())
                    .into()
            })
            .with_group("Layout")
    }
}

crate::define_widget_deref!(TabControl);

uuid_provider!(TabControl = "d54cfac3-0afc-464b-838a-158b3a2253f5");

impl TabControl {
    fn do_drag(&mut self, position: Vector2<f32>, ui: &mut UserInterface) {
        let mut dragged_index = None;
        let mut target_index = None;
        for (tab_index, tab) in self.tabs.iter().enumerate() {
            let bounds = ui.node(tab.header_button).screen_bounds();
            let node_x = bounds.center().x;
            if bounds.contains(position) {
                if node_x < position.x {
                    target_index = Some(tab_index + 1);
                } else {
                    target_index = Some(tab_index);
                }
            }
            if ui.is_node_child_of(ui.captured_node, tab.header_button) {
                dragged_index = Some(tab_index);
            }
        }
        if let (Some(dragged_index), Some(mut target_index)) = (dragged_index, target_index) {
            if dragged_index < target_index {
                target_index -= 1;
            }
            if target_index != dragged_index {
                self.finalize_drag(dragged_index, target_index, ui);
            }
        }
    }
    fn finalize_drag(&mut self, from: usize, to: usize, ui: &mut UserInterface) {
        let uuid = self.active_tab.map(|i| self.tabs[i].uuid);
        let tab = self.tabs.remove(from);
        self.tabs.insert(to, tab);
        if let Some(uuid) = uuid {
            self.active_tab = self.tabs.iter().position(|t| t.uuid == uuid);
        }
        let new_tab_handles = self.tabs.iter().map(|t| t.header_container).collect();
        ui.send_message(WidgetMessage::replace_children(
            self.headers_container,
            MessageDirection::ToWidget,
            new_tab_handles,
        ));
    }
    /// Use a tab's UUID to look up the tab.
    pub fn get_tab_by_uuid(&self, uuid: Uuid) -> Option<&Tab> {
        self.tabs.iter().find(|t| t.uuid == uuid)
    }
    /// Send the necessary messages to activate the tab at the given index, or deactivate all tabs if no index is given.
    /// Do nothing if the given index does not refer to any existing tab.
    /// If the index was valid, send FromWidget messages to notify listeners of the change, using messages with the given flags.
    fn set_active_tab(&mut self, active_tab: Option<usize>, ui: &mut UserInterface, flags: u64) {
        if let Some(index) = active_tab {
            if self.tabs.len() <= index {
                return;
            }
        }
        // Send messages to update the state of each tab.
        for (existing_tab_index, tab) in self.tabs.iter().enumerate() {
            ui.send_message(WidgetMessage::visibility(
                tab.content,
                MessageDirection::ToWidget,
                active_tab == Some(existing_tab_index),
            ));
            ui.send_message(DecoratorMessage::select(
                tab.decorator,
                MessageDirection::ToWidget,
                active_tab == Some(existing_tab_index),
            ))
        }

        self.active_tab = active_tab;

        // Notify potential listeners that the active tab has changed.
        // First we notify by tab index.
        let mut msg =
            TabControlMessage::active_tab(self.handle, MessageDirection::FromWidget, active_tab);
        msg.flags = flags;
        ui.send_message(msg);
        // Next we notify by the tab's uuid, which does not change even as the tab moves.
        let tab_id = active_tab.and_then(|i| self.tabs.get(i)).map(|t| t.uuid);
        let mut msg =
            TabControlMessage::active_tab_uuid(self.handle, MessageDirection::FromWidget, tab_id);
        msg.flags = flags;
        ui.send_message(msg);
    }
    /// Send the messages necessary to remove the tab at the given index and update the currently active tab.
    /// This does not include sending FromWidget messages to notify listeners.
    /// If the given index does not refer to any tab, do nothing and return false.
    /// Otherwise, return true to indicate that some tab was removed.
    fn remove_tab(&mut self, index: usize, ui: &mut UserInterface) -> bool {
        let Some(tab) = self.tabs.get(index) else {
            return false;
        };
        ui.send_message(WidgetMessage::remove(
            tab.header_container,
            MessageDirection::ToWidget,
        ));
        ui.send_message(WidgetMessage::remove(
            tab.content,
            MessageDirection::ToWidget,
        ));

        self.tabs.remove(index);

        if let Some(active_tab) = &self.active_tab {
            match index.cmp(active_tab) {
                Ordering::Less => self.active_tab = Some(active_tab - 1), // Just the index needs to change, not the actual tab.
                Ordering::Equal => {
                    // The active tab was removed, so we need to change the active tab.
                    if self.tabs.is_empty() {
                        self.set_active_tab(None, ui, 0);
                    } else if *active_tab == 0 {
                        // The index has not changed, but this is actually a different tab,
                        // so we need to activate it.
                        self.set_active_tab(Some(0), ui, 0);
                    } else {
                        self.set_active_tab(Some(active_tab - 1), ui, 0);
                    }
                }
                Ordering::Greater => (), // Do nothing, since removed tab was to the right of active tab.
            }
        }

        true
    }
}

impl Control for TabControl {
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(ButtonMessage::Click) = message.data() {
            for (tab_index, tab) in self.tabs.iter().enumerate() {
                if message.destination() == tab.header_button && tab.header_button.is_some() {
                    ui.send_message(TabControlMessage::active_tab_uuid(
                        self.handle,
                        MessageDirection::ToWidget,
                        Some(tab.uuid),
                    ));
                    break;
                } else if message.destination() == tab.close_button {
                    // Send two messages, one containing the index, one containing the UUID,
                    // to allow listeners their choice of which system they prefer.
                    ui.send_message(TabControlMessage::close_tab(
                        self.handle,
                        MessageDirection::FromWidget,
                        tab_index,
                    ));
                    ui.send_message(TabControlMessage::close_tab_by_uuid(
                        self.handle,
                        MessageDirection::FromWidget,
                        tab.uuid,
                    ));
                }
            }
        } else if let Some(WidgetMessage::MouseDown { button, .. }) = message.data() {
            if *button == MouseButton::Middle {
                for (tab_index, tab) in self.tabs.iter().enumerate() {
                    if ui.is_node_child_of(message.destination(), tab.header_button) {
                        ui.send_message(TabControlMessage::close_tab(
                            self.handle,
                            MessageDirection::FromWidget,
                            tab_index,
                        ));
                        ui.send_message(TabControlMessage::close_tab_by_uuid(
                            self.handle,
                            MessageDirection::FromWidget,
                            tab.uuid,
                        ));
                    }
                }
            }
        } else if let Some(WidgetMessage::MouseMove { pos, state }) = message.data() {
            if state.left == ButtonState::Pressed
                && self.is_tab_drag_allowed
                && ui.is_node_child_of(ui.captured_node, self.headers_container)
            {
                self.do_drag(*pos, ui);
            }
        } else if let Some(msg) = message.data::<TabControlMessage>() {
            if message.destination() == self.handle()
                && message.direction() == MessageDirection::ToWidget
            {
                match msg {
                    TabControlMessage::ActiveTab(active_tab) => {
                        if self.active_tab != *active_tab {
                            self.set_active_tab(*active_tab, ui, message.flags);
                        }
                    }
                    TabControlMessage::ActiveTabUuid(uuid) => match uuid {
                        Some(uuid) => {
                            if let Some(active_tab) = self.tabs.iter().position(|t| t.uuid == *uuid)
                            {
                                if self.active_tab != Some(active_tab) {
                                    self.set_active_tab(Some(active_tab), ui, message.flags);
                                }
                            }
                        }
                        None if self.active_tab.is_some() => {
                            self.set_active_tab(None, ui, message.flags)
                        }
                        _ => (),
                    },
                    TabControlMessage::CloseTab(_) | TabControlMessage::CloseTabByUuid(_) => {
                        // Nothing to do.
                    }
                    TabControlMessage::RemoveTab(index) => {
                        // If a tab was removed, then resend the message.
                        // Users that remove tabs using the index-based message only get the index-based message in reponse,
                        // since presumably their application is not using UUIDs.
                        if self.remove_tab(*index, ui) {
                            ui.send_message(message.reverse());
                        }
                    }
                    TabControlMessage::RemoveTabByUuid(uuid) => {
                        // Find the tab that has the given uuid.
                        let index = self.tabs.iter().position(|t| t.uuid == *uuid);
                        // Users that remove tabs using the UUID-based message only get the UUID-based message in reponse,
                        // since presumably their application is not using tab indices.
                        if let Some(index) = index {
                            if self.remove_tab(index, ui) {
                                ui.send_message(message.reverse());
                            }
                        }
                    }
                    TabControlMessage::AddTab { uuid, definition } => {
                        if self.tabs.iter().any(|t| &t.uuid == uuid) {
                            ui.send_message(WidgetMessage::remove(
                                definition.header,
                                MessageDirection::ToWidget,
                            ));
                            ui.send_message(WidgetMessage::remove(
                                definition.content,
                                MessageDirection::ToWidget,
                            ));
                            return;
                        }
                        let header = Header::build(
                            definition,
                            false,
                            (*self.active_tab_brush).clone(),
                            &mut ui.build_ctx(),
                        );

                        ui.send_message(WidgetMessage::link(
                            header.button,
                            MessageDirection::ToWidget,
                            self.headers_container,
                        ));

                        ui.send_message(WidgetMessage::link(
                            definition.content,
                            MessageDirection::ToWidget,
                            self.content_container,
                        ));

                        ui.send_message(message.reverse());

                        self.tabs.push(Tab {
                            uuid: *uuid,
                            header_button: header.button,
                            content: definition.content,
                            close_button: header.close_button,
                            header_container: header.button,
                            user_data: definition.user_data.clone(),
                            decorator: header.decorator,
                            header_content: header.content,
                        });
                    }
                }
            }
        }
    }
}

/// Tab control builder is used to create [`TabControl`] widget instances and add them to the user interface.
pub struct TabControlBuilder {
    widget_builder: WidgetBuilder,
    is_tab_drag_allowed: bool,
    tabs: Vec<(Uuid, TabDefinition)>,
    active_tab_brush: Option<StyledProperty<Brush>>,
    initial_tab: usize,
}

/// Tab definition is used to describe content of each tab for the [`TabControlBuilder`] builder.
#[derive(Debug, Clone, PartialEq)]
pub struct TabDefinition {
    /// Content of the tab-switching (header) button.
    pub header: Handle<UiNode>,
    /// Content of the tab.
    pub content: Handle<UiNode>,
    /// A flag, that defines whether the tab can be closed or not.
    pub can_be_closed: bool,
    /// User-defined data.
    pub user_data: Option<TabUserData>,
}

struct Header {
    button: Handle<UiNode>,
    close_button: Handle<UiNode>,
    decorator: Handle<UiNode>,
    content: Handle<UiNode>,
}

impl Header {
    fn build(
        tab_definition: &TabDefinition,
        selected: bool,
        active_tab_brush: StyledProperty<Brush>,
        ctx: &mut BuildContext,
    ) -> Self {
        let close_button;
        let decorator;

        let button = ButtonBuilder::new(WidgetBuilder::new().on_row(0).on_column(0))
            .with_back({
                decorator = DecoratorBuilder::new(
                    BorderBuilder::new(WidgetBuilder::new())
                        .with_stroke_thickness(Thickness::uniform(0.0).into()),
                )
                .with_normal_brush(ctx.style.property(Style::BRUSH_DARK))
                .with_selected_brush(active_tab_brush)
                .with_pressed_brush(ctx.style.property(Style::BRUSH_LIGHTEST))
                .with_hover_brush(ctx.style.property(Style::BRUSH_LIGHT))
                .with_selected(selected)
                .build(ctx);
                decorator
            })
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .with_child(tab_definition.header)
                        .with_child({
                            close_button = if tab_definition.can_be_closed {
                                ButtonBuilder::new(
                                    WidgetBuilder::new()
                                        .with_margin(Thickness::right(1.0))
                                        .on_row(0)
                                        .on_column(1)
                                        .with_width(16.0)
                                        .with_height(16.0),
                                )
                                .with_back(
                                    DecoratorBuilder::new(
                                        BorderBuilder::new(WidgetBuilder::new())
                                            .with_corner_radius(5.0f32.into())
                                            .with_pad_by_corner_radius(false)
                                            .with_stroke_thickness(Thickness::uniform(0.0).into()),
                                    )
                                    .with_normal_brush(Brush::Solid(Color::TRANSPARENT).into())
                                    .with_hover_brush(ctx.style.property(Style::BRUSH_DARK))
                                    .build(ctx),
                                )
                                .with_content(
                                    VectorImageBuilder::new(
                                        WidgetBuilder::new()
                                            .with_horizontal_alignment(HorizontalAlignment::Center)
                                            .with_vertical_alignment(VerticalAlignment::Center)
                                            .with_width(8.0)
                                            .with_height(8.0)
                                            .with_foreground(
                                                ctx.style.property(Style::BRUSH_BRIGHTEST),
                                            ),
                                    )
                                    .with_primitives(make_cross_primitive(8.0, 2.0))
                                    .build(ctx),
                                )
                                .build(ctx)
                            } else {
                                Handle::NONE
                            };
                            close_button
                        }),
                )
                .add_row(Row::auto())
                .add_column(Column::stretch())
                .add_column(Column::auto())
                .build(ctx),
            )
            .build(ctx);

        Header {
            button,
            close_button,
            decorator,
            content: tab_definition.header,
        }
    }
}

impl TabControlBuilder {
    /// Creates new tab control builder.
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            tabs: Default::default(),
            is_tab_drag_allowed: false,
            active_tab_brush: None,
            initial_tab: 0,
            widget_builder,
        }
    }

    /// Controls the initially selected tab. The default is 0, the first tab on the left.
    pub fn with_initial_tab(mut self, tab_index: usize) -> Self {
        self.initial_tab = tab_index;
        self
    }

    /// Controls whether tabs may be dragged. The default is false.
    pub fn with_tab_drag(mut self, is_tab_drag_allowed: bool) -> Self {
        self.is_tab_drag_allowed = is_tab_drag_allowed;
        self
    }

    /// Adds a new tab to the builder.
    pub fn with_tab(mut self, tab: TabDefinition) -> Self {
        self.tabs.push((Uuid::new_v4(), tab));
        self
    }

    /// Adds a new tab to the builder, using the given UUID for the tab.
    pub fn with_tab_uuid(mut self, uuid: Uuid, tab: TabDefinition) -> Self {
        self.tabs.push((uuid, tab));
        self
    }

    /// Sets a desired brush for active tab.
    pub fn with_active_tab_brush(mut self, brush: StyledProperty<Brush>) -> Self {
        self.active_tab_brush = Some(brush);
        self
    }

    /// Finishes [`TabControl`] building and adds it to the user interface and returns its handle.
    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let tab_count = self.tabs.len();
        // Hide everything but initial tab content.
        for (i, (_, tab)) in self.tabs.iter().enumerate() {
            if let Some(content) = ctx.try_get_node_mut(tab.content) {
                content.set_visibility(i == self.initial_tab);
            }
        }

        let active_tab_brush = self
            .active_tab_brush
            .unwrap_or_else(|| ctx.style.property::<Brush>(Style::BRUSH_LIGHTEST));

        let tab_headers = self
            .tabs
            .iter()
            .enumerate()
            .map(|(i, (_, tab_definition))| {
                Header::build(
                    tab_definition,
                    i == self.initial_tab,
                    active_tab_brush.clone(),
                    ctx,
                )
            })
            .collect::<Vec<_>>();

        let headers_container = WrapPanelBuilder::new(
            WidgetBuilder::new()
                .with_children(tab_headers.iter().map(|h| h.button))
                .on_row(0),
        )
        .with_orientation(Orientation::Horizontal)
        .build(ctx);

        let content_container = GridBuilder::new(
            WidgetBuilder::new()
                .with_children(self.tabs.iter().map(|(_, t)| t.content))
                .on_row(1),
        )
        .add_row(Row::stretch())
        .add_column(Column::stretch())
        .build(ctx);

        let grid = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(headers_container)
                .with_child(content_container),
        )
        .add_column(Column::stretch())
        .add_row(Row::auto())
        .add_row(Row::stretch())
        .build(ctx);

        let border = BorderBuilder::new(
            WidgetBuilder::new()
                .with_background(ctx.style.property(Style::BRUSH_DARK))
                .with_child(grid),
        )
        .build(ctx);

        let tc = TabControl {
            widget: self.widget_builder.with_child(border).build(ctx),
            is_tab_drag_allowed: self.is_tab_drag_allowed,
            active_tab: if tab_count == 0 {
                None
            } else {
                Some(self.initial_tab)
            },
            tabs: tab_headers
                .into_iter()
                .zip(self.tabs)
                .map(|(header, (uuid, tab))| Tab {
                    uuid,
                    header_button: header.button,
                    content: tab.content,
                    close_button: header.close_button,
                    header_container: header.button,
                    user_data: tab.user_data,
                    decorator: header.decorator,
                    header_content: header.content,
                })
                .collect(),
            content_container,
            headers_container,
            active_tab_brush: active_tab_brush.into(),
        };

        ctx.add_node(UiNode::new(tc))
    }
}

#[cfg(test)]
mod test {
    use crate::tab_control::TabControlBuilder;
    use crate::{test::test_widget_deletion, widget::WidgetBuilder};

    #[test]
    fn test_deletion() {
        test_widget_deletion(|ctx| TabControlBuilder::new(WidgetBuilder::new()).build(ctx));
    }
}
