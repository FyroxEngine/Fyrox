use crate::absm::{
    selectable::{Selectable, SelectableMessage},
    BORDER_COLOR, NORMAL_BACKGROUND, SELECTED_BACKGROUND,
};
use fyrox::{
    core::{color::Color, pool::Handle},
    gui::{
        border::BorderBuilder,
        brush::Brush,
        button::{ButtonBuilder, ButtonMessage},
        define_constructor,
        grid::{Column, GridBuilder, Row},
        message::{MessageDirection, MouseButton, UiMessage},
        stack_panel::StackPanelBuilder,
        text::TextBuilder,
        widget::{Widget, WidgetBuilder, WidgetMessage},
        BuildContext, Control, HorizontalAlignment, Thickness, UiNode, UserInterface,
        VerticalAlignment,
    },
};
use std::{
    any::{Any, TypeId},
    ops::{Deref, DerefMut},
};

// An "interface" marker that allows to check if the node is "some" ABSM node, without knowing
// actual data model handle type.
pub struct AbsmNodeMarker;

pub struct AbsmNode<T>
where
    T: 'static,
{
    widget: Widget,
    background: Handle<UiNode>,
    selectable: Selectable,
    pub input_sockets: Vec<Handle<UiNode>>,
    pub output_socket: Handle<UiNode>,
    pub name: String,
    pub model_handle: Handle<T>,
    marker: AbsmNodeMarker,
    pub add_input: Handle<UiNode>,
    input_sockets_panel: Handle<UiNode>,
}

impl<T> Clone for AbsmNode<T>
where
    T: 'static,
{
    fn clone(&self) -> Self {
        Self {
            widget: self.widget.clone(),
            background: self.background,
            selectable: self.selectable.clone(),
            input_sockets: self.input_sockets.clone(),
            output_socket: self.output_socket,
            name: self.name.clone(),
            model_handle: self.model_handle,
            marker: AbsmNodeMarker,
            add_input: self.add_input,
            input_sockets_panel: self.input_sockets_panel,
        }
    }
}

impl<T> Deref for AbsmNode<T>
where
    T: 'static,
{
    type Target = Widget;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<T> DerefMut for AbsmNode<T>
where
    T: 'static,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AbsmNodeMessage {
    Name(String),
    Enter,
    AddInput,
    InputSockets(Vec<Handle<UiNode>>),
}

impl AbsmNodeMessage {
    define_constructor!(AbsmNodeMessage:Name => fn name(String), layout: false);
    define_constructor!(AbsmNodeMessage:Enter => fn enter(), layout: false);
    define_constructor!(AbsmNodeMessage:AddInput => fn add_input(), layout: false);
    define_constructor!(AbsmNodeMessage:InputSockets => fn input_sockets(Vec<Handle<UiNode>>), layout: false);
}

impl<T> Control for AbsmNode<T>
where
    T: 'static,
{
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<Self>() {
            Some(self)
        } else if type_id == TypeId::of::<Selectable>() {
            Some(&self.selectable)
        } else if type_id == TypeId::of::<AbsmNodeMarker>() {
            Some(&self.marker)
        } else {
            None
        }
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);
        self.selectable
            .handle_routed_message(self.handle(), ui, message);

        if let Some(SelectableMessage::Select(selected)) = message.data() {
            if message.destination() == self.handle()
                && message.direction() == MessageDirection::FromWidget
            {
                ui.send_message(WidgetMessage::background(
                    self.background,
                    MessageDirection::ToWidget,
                    Brush::Solid(if *selected {
                        SELECTED_BACKGROUND
                    } else {
                        NORMAL_BACKGROUND
                    }),
                ));

                if *selected {
                    ui.send_message(WidgetMessage::topmost(
                        self.handle(),
                        MessageDirection::ToWidget,
                    ));
                }
            }
        } else if let Some(WidgetMessage::DoubleClick { button }) = message.data() {
            if !message.handled() && *button == MouseButton::Left {
                ui.send_message(AbsmNodeMessage::enter(
                    self.handle(),
                    MessageDirection::FromWidget,
                ));
            }
        } else if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.add_input {
                ui.send_message(AbsmNodeMessage::add_input(
                    self.handle(),
                    MessageDirection::FromWidget,
                ));
            }
        } else if let Some(AbsmNodeMessage::InputSockets(input_sockets)) = message.data() {
            if message.destination == self.handle()
                && message.direction() == MessageDirection::ToWidget
                && input_sockets != &self.input_sockets
            {
                for &child in ui.node(self.input_sockets_panel).children() {
                    ui.send_message(WidgetMessage::remove(child, MessageDirection::ToWidget));
                }

                for &socket in input_sockets {
                    ui.send_message(WidgetMessage::link(
                        socket,
                        MessageDirection::ToWidget,
                        self.input_sockets_panel,
                    ));
                }

                self.input_sockets = input_sockets.clone();
            }
        }
    }
}

pub struct AbsmNodeBuilder<T>
where
    T: 'static,
{
    widget_builder: WidgetBuilder,
    name: String,
    model_handle: Handle<T>,
    input_sockets: Vec<Handle<UiNode>>,
    output_socket: Handle<UiNode>,
    can_add_sockets: bool,
    title: Option<String>,
}

impl<T> AbsmNodeBuilder<T>
where
    T: 'static,
{
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            name: "New State".to_string(),
            model_handle: Default::default(),
            input_sockets: Default::default(),
            output_socket: Default::default(),
            can_add_sockets: false,
            title: None,
        }
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.name = name;
        self
    }

    pub fn with_model_handle(mut self, model: Handle<T>) -> Self {
        self.model_handle = model;
        self
    }

    pub fn with_input_sockets(mut self, sockets: Vec<Handle<UiNode>>) -> Self {
        self.input_sockets = sockets;
        self
    }

    pub fn with_output_socket(mut self, socket: Handle<UiNode>) -> Self {
        self.output_socket = socket;
        self
    }

    pub fn with_can_add_sockets(mut self, state: bool) -> Self {
        self.can_add_sockets = state;
        self
    }

    pub fn with_title(mut self, title: String) -> Self {
        self.title = Some(title);
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let input_sockets_panel;
        let add_input;
        let grid = GridBuilder::new(
            WidgetBuilder::new()
                .on_row(1)
                .on_column(0)
                .with_child(
                    GridBuilder::new(
                        WidgetBuilder::new()
                            .with_child({
                                input_sockets_panel = StackPanelBuilder::new(
                                    WidgetBuilder::new()
                                        .on_row(0)
                                        .on_column(0)
                                        .with_margin(Thickness::uniform(2.0))
                                        .with_vertical_alignment(VerticalAlignment::Center)
                                        .with_children(self.input_sockets.iter().cloned())
                                        .on_column(0),
                                )
                                .build(ctx);
                                input_sockets_panel
                            })
                            .with_child({
                                add_input = ButtonBuilder::new(
                                    WidgetBuilder::new()
                                        .with_margin(Thickness::uniform(1.0))
                                        .with_height(20.0)
                                        .with_visibility(self.can_add_sockets)
                                        .on_row(1)
                                        .on_column(0),
                                )
                                .with_text("+Input")
                                .build(ctx);
                                add_input
                            }),
                    )
                    .add_row(Row::stretch())
                    .add_row(Row::auto())
                    .add_column(Column::auto())
                    .build(ctx),
                )
                .with_child(
                    TextBuilder::new(
                        WidgetBuilder::new()
                            .with_width(150.0)
                            .with_height(75.0)
                            .on_column(1),
                    )
                    .with_vertical_text_alignment(VerticalAlignment::Center)
                    .with_horizontal_text_alignment(HorizontalAlignment::Center)
                    .with_text(&self.name)
                    .build(ctx),
                )
                .with_child(
                    StackPanelBuilder::new(
                        WidgetBuilder::new()
                            .with_margin(Thickness::uniform(2.0))
                            .with_vertical_alignment(VerticalAlignment::Center)
                            .with_child(self.output_socket)
                            .on_column(2),
                    )
                    .build(ctx),
                ),
        )
        .add_row(Row::stretch())
        .add_column(Column::auto())
        .add_column(Column::stretch())
        .add_column(Column::auto())
        .build(ctx);

        let grid2 = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(
                    self.title
                        .map(|title| {
                            BorderBuilder::new(
                                WidgetBuilder::new()
                                    .with_height(24.0)
                                    .with_background(Brush::Solid(Color::opaque(30, 30, 30)))
                                    .with_child(
                                        TextBuilder::new(
                                            WidgetBuilder::new()
                                                .with_vertical_alignment(VerticalAlignment::Center)
                                                .with_margin(Thickness::uniform(2.0)),
                                        )
                                        .with_text(title)
                                        .build(ctx),
                                    ),
                            )
                            .with_stroke_thickness(Thickness::zero())
                            .build(ctx)
                        })
                        .unwrap_or_default(),
                )
                .with_child(grid),
        )
        .add_row(Row::auto())
        .add_row(Row::stretch())
        .add_column(Column::stretch())
        .build(ctx);

        let background = BorderBuilder::new(
            WidgetBuilder::new()
                .with_foreground(Brush::Solid(BORDER_COLOR))
                .with_background(Brush::Solid(NORMAL_BACKGROUND))
                .with_child(grid2),
        )
        .build(ctx);

        let node = AbsmNode {
            widget: self.widget_builder.with_child(background).build(),
            background,
            selectable: Default::default(),
            model_handle: self.model_handle,
            name: self.name,
            input_sockets: self.input_sockets,
            output_socket: self.output_socket,
            marker: AbsmNodeMarker,
            add_input,
            input_sockets_panel,
        };

        ctx.add_node(UiNode::new(node))
    }
}
