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

use crate::fyrox::{
    core::{
        color::Color, pool::Handle, reflect::prelude::*, type_traits::prelude::*, uuid::uuid,
        visitor::prelude::*,
    },
    graph::BaseSceneGraph,
    gui::{
        border::{BorderBuilder, BorderMessage},
        brush::Brush,
        button::{ButtonBuilder, ButtonMessage},
        define_constructor,
        grid::{Column, GridBuilder, Row},
        message::{MessageDirection, MouseButton, UiMessage},
        stack_panel::StackPanelBuilder,
        text::{TextBuilder, TextMessage},
        widget::{Widget, WidgetBuilder, WidgetMessage},
        BuildContext, Control, HorizontalAlignment, Thickness, UiNode, UserInterface,
        VerticalAlignment,
    },
};
use crate::plugins::absm::selectable::{Selectable, SelectableMessage};
use fyrox::core::pool::NodeVariant;

use fyrox::gui::style::resource::StyleResourceExt;
use fyrox::gui::style::{Style, StyledProperty};
use std::{
    fmt::{Debug, Formatter},
    ops::{Deref, DerefMut},
};

#[derive(Clone, Debug, Visit, Reflect)]
pub struct AbsmBaseNode {
    pub input_sockets: Vec<Handle<UiNode>>,
    pub output_socket: Handle<UiNode>,
}

#[derive(Visit, Reflect, ComponentProvider)]
#[reflect(derived_type = "UiNode")]
pub struct AbsmNode<T>
where
    T: Reflect,
{
    widget: Widget,
    background: Handle<UiNode>,
    #[component(include)]
    selectable: Selectable,
    pub name_value: String,
    pub model_handle: Handle<T>,
    #[component(include)]
    pub base: AbsmBaseNode,
    pub add_input: Handle<UiNode>,
    input_sockets_panel: Handle<UiNode>,
    normal_brush: StyledProperty<Brush>,
    selected_brush: StyledProperty<Brush>,
    name: Handle<UiNode>,
    edit: Handle<UiNode>,
}

impl NodeVariant<UiNode> for AbsmNode<fyrox::scene::node::Node> {}

impl<T: Reflect> Debug for AbsmNode<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "AbsmNode")
    }
}

impl<T> Clone for AbsmNode<T>
where
    T: Reflect,
{
    fn clone(&self) -> Self {
        Self {
            widget: self.widget.clone(),
            background: self.background,
            selectable: self.selectable.clone(),
            name_value: self.name_value.clone(),
            model_handle: self.model_handle,
            base: self.base.clone(),
            add_input: self.add_input,
            input_sockets_panel: self.input_sockets_panel,
            normal_brush: self.normal_brush.clone(),
            selected_brush: self.selected_brush.clone(),
            name: self.name,
            edit: self.edit,
        }
    }
}

impl<T> Deref for AbsmNode<T>
where
    T: Reflect,
{
    type Target = Widget;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<T> DerefMut for AbsmNode<T>
where
    T: Reflect,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl<T> AbsmNode<T>
where
    T: Reflect,
{
    fn update_colors(&self, ui: &UserInterface) {
        ui.send_message(WidgetMessage::background(
            self.background,
            MessageDirection::ToWidget,
            if self.selectable.selected {
                self.selected_brush.clone()
            } else {
                self.normal_brush.clone()
            },
        ));
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AbsmNodeMessage {
    Name(String),
    Enter,
    AddInput,
    InputSockets(Vec<Handle<UiNode>>),
    NormalBrush(StyledProperty<Brush>),
    SelectedBrush(StyledProperty<Brush>),
    SetActive(bool),
    Edit,
}

impl AbsmNodeMessage {
    define_constructor!(AbsmNodeMessage:Name => fn name(String), layout: false);
    define_constructor!(AbsmNodeMessage:Enter => fn enter(), layout: false);
    define_constructor!(AbsmNodeMessage:AddInput => fn add_input(), layout: false);
    define_constructor!(AbsmNodeMessage:InputSockets => fn input_sockets(Vec<Handle<UiNode>>), layout: false);
    define_constructor!(AbsmNodeMessage:NormalBrush => fn normal_color(StyledProperty<Brush>), layout: false);
    define_constructor!(AbsmNodeMessage:SelectedBrush => fn selected_color(StyledProperty<Brush>), layout: false);
    define_constructor!(AbsmNodeMessage:SetActive => fn set_active(bool), layout: false);
    define_constructor!(AbsmNodeMessage:Edit => fn edit(), layout: false);
}

impl<T: Reflect> TypeUuidProvider for AbsmNode<T> {
    fn type_uuid() -> Uuid {
        uuid!("15bc1a7e-a385-46e0-a65c-7e9c014b4a1d")
    }
}

impl<T> Control for AbsmNode<T>
where
    T: Reflect,
{
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);
        self.selectable
            .handle_routed_message(self.handle(), ui, message);

        if let Some(SelectableMessage::Select(selected)) = message.data() {
            if message.destination() == self.handle()
                && message.direction() == MessageDirection::FromWidget
            {
                self.update_colors(ui);
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
            } else if message.destination() == self.edit {
                ui.send_message(AbsmNodeMessage::edit(
                    self.handle(),
                    MessageDirection::FromWidget,
                ));
            }
        } else if let Some(msg) = message.data::<AbsmNodeMessage>() {
            if message.destination == self.handle()
                && message.direction() == MessageDirection::ToWidget
            {
                match msg {
                    AbsmNodeMessage::InputSockets(input_sockets) => {
                        if input_sockets != &self.base.input_sockets {
                            for &child in ui.node(self.input_sockets_panel).children() {
                                ui.send_message(WidgetMessage::remove(
                                    child,
                                    MessageDirection::ToWidget,
                                ));
                            }

                            for &socket in input_sockets {
                                ui.send_message(WidgetMessage::link(
                                    socket,
                                    MessageDirection::ToWidget,
                                    self.input_sockets_panel,
                                ));
                            }

                            self.base.input_sockets.clone_from(input_sockets);
                        }
                    }
                    AbsmNodeMessage::NormalBrush(color) => {
                        if &self.normal_brush != color {
                            self.normal_brush = color.clone();
                            self.update_colors(ui);
                        }
                    }
                    AbsmNodeMessage::SelectedBrush(color) => {
                        if &self.selected_brush != color {
                            self.selected_brush = color.clone();
                            self.update_colors(ui);
                        }
                    }
                    AbsmNodeMessage::Name(name) => {
                        if &self.name_value != name {
                            self.name_value.clone_from(name);

                            ui.send_message(TextMessage::text(
                                self.name,
                                MessageDirection::ToWidget,
                                format!("{} ({})", self.name_value, self.model_handle),
                            ));
                        }
                    }
                    AbsmNodeMessage::SetActive(active) => {
                        let (thickness, brush) = if *active {
                            (
                                Thickness::uniform(3.0),
                                Brush::Solid(Color::opaque(120, 80, 60)).into(),
                            )
                        } else {
                            (
                                Thickness::uniform(1.0),
                                ui.style.property(Style::BRUSH_LIGHT),
                            )
                        };

                        ui.send_message(BorderMessage::stroke_thickness(
                            self.background,
                            MessageDirection::ToWidget,
                            thickness.into(),
                        ));
                        ui.send_message(WidgetMessage::foreground(
                            self.background,
                            MessageDirection::ToWidget,
                            brush,
                        ));
                    }
                    _ => (),
                }
            }
        }
    }
}

pub struct AbsmNodeBuilder<T>
where
    T: Reflect,
{
    widget_builder: WidgetBuilder,
    name: String,
    model_handle: Handle<T>,
    input_sockets: Vec<Handle<UiNode>>,
    output_socket: Handle<UiNode>,
    can_add_sockets: bool,
    title: Option<String>,
    normal_brush: Option<StyledProperty<Brush>>,
    selected_brush: Option<StyledProperty<Brush>>,
    editable: bool,
}

impl<T> AbsmNodeBuilder<T>
where
    T: Reflect,
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
            normal_brush: None,
            selected_brush: None,
            editable: false,
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

    pub fn with_normal_brush(mut self, brush: StyledProperty<Brush>) -> Self {
        self.normal_brush = Some(brush);
        self
    }

    pub fn with_selected_brush(mut self, brush: StyledProperty<Brush>) -> Self {
        self.selected_brush = Some(brush);
        self
    }

    pub fn with_editable(mut self, editable: bool) -> Self {
        self.editable = editable;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let input_sockets_panel;
        let add_input;
        let name;
        let mut edit = Handle::NONE;
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
                    StackPanelBuilder::new(
                        WidgetBuilder::new()
                            .on_column(1)
                            .with_child({
                                name = TextBuilder::new(
                                    WidgetBuilder::new().with_width(150.0).with_height(75.0),
                                )
                                .with_vertical_text_alignment(VerticalAlignment::Center)
                                .with_horizontal_text_alignment(HorizontalAlignment::Center)
                                .with_text(format!("{} ({})", self.name, self.model_handle))
                                .build(ctx);
                                name
                            })
                            .with_child(if self.editable {
                                edit = ButtonBuilder::new(
                                    WidgetBuilder::new().with_margin(Thickness::uniform(1.0)),
                                )
                                .with_text("Edit")
                                .build(ctx);
                                edit
                            } else {
                                Handle::NONE
                            }),
                    )
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
                                    .with_background(ctx.style.property(Style::BRUSH_DARKER))
                                    .with_child(
                                        TextBuilder::new(
                                            WidgetBuilder::new()
                                                .with_vertical_alignment(VerticalAlignment::Center)
                                                .with_horizontal_alignment(
                                                    HorizontalAlignment::Center,
                                                )
                                                .with_margin(Thickness::uniform(2.0)),
                                        )
                                        .with_text(title)
                                        .build(ctx),
                                    ),
                            )
                            .with_pad_by_corner_radius(false)
                            .with_corner_radius(12.0f32.into())
                            .with_stroke_thickness(Thickness::zero().into())
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

        let normal_brush = self
            .normal_brush
            .unwrap_or_else(|| ctx.style.property(Style::BRUSH_LIGHTER_PRIMARY));

        let selected_brush = self
            .selected_brush
            .unwrap_or_else(|| ctx.style.property(Style::BRUSH_LIGHTER));

        let background = BorderBuilder::new(
            WidgetBuilder::new()
                .with_foreground(ctx.style.property(Style::BRUSH_LIGHT))
                .with_background(normal_brush.clone())
                .with_child(grid2),
        )
        .with_pad_by_corner_radius(false)
        .with_corner_radius(12.0f32.into())
        .build(ctx);

        let node = AbsmNode {
            widget: self.widget_builder.with_child(background).build(ctx),
            background,
            selectable: Default::default(),
            model_handle: self.model_handle,
            name_value: self.name,
            base: AbsmBaseNode {
                input_sockets: self.input_sockets,
                output_socket: self.output_socket,
            },
            add_input,
            input_sockets_panel,
            normal_brush,
            selected_brush,
            name,
            edit,
        };

        ctx.add_node(UiNode::new(node))
    }
}

#[cfg(test)]
mod test {
    use crate::plugins::absm::node::AbsmNodeBuilder;
    use fyrox::scene::node::Node;
    use fyrox::{gui::test::test_widget_deletion, gui::widget::WidgetBuilder};

    #[test]
    fn test_deletion() {
        test_widget_deletion(|ctx| AbsmNodeBuilder::<Node>::new(WidgetBuilder::new()).build(ctx));
    }
}
