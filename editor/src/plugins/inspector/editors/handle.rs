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

use crate::scene::selector::{AllowedType, SelectedHandle};
use crate::{
    fyrox::{
        core::{
            color::Color, pool::ErasedHandle, pool::Handle, reflect::prelude::*,
            type_traits::prelude::*, visitor::prelude::*,
        },
        graph::BaseSceneGraph,
        gui::{
            brush::Brush,
            button::{ButtonBuilder, ButtonMessage},
            define_constructor,
            draw::{CommandTexture, Draw, DrawingContext},
            grid::{Column, GridBuilder, Row},
            image::ImageBuilder,
            inspector::{
                editors::{
                    PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorInstance,
                    PropertyEditorMessageContext, PropertyEditorTranslationContext,
                },
                FieldKind, InspectorError, PropertyChanged,
            },
            message::MessageDirection,
            style::{resource::StyleResourceExt, Style},
            text::{TextBuilder, TextMessage},
            utils::make_simple_tooltip,
            widget::{Widget, WidgetBuilder, WidgetMessage},
            window::{WindowBuilder, WindowMessage, WindowTitle},
            BuildContext, Control, Thickness,
        },
    },
    load_image_internal,
    message::MessageSender,
    scene::selector::{HierarchyNode, NodeSelectorMessage, NodeSelectorWindowBuilder},
    world::graph::item::SceneItem,
    Message, UiMessage, UiNode, UserInterface, VerticalAlignment,
};
use fyrox::core::PhantomDataSendSync;
use std::{
    any::TypeId,
    fmt::{Debug, Formatter},
    ops::{Deref, DerefMut},
    sync::Mutex,
};

pub enum HandlePropertyEditorMessage<T: Reflect> {
    Value(Handle<T>),
}

impl<T: Reflect> Clone for HandlePropertyEditorMessage<T> {
    fn clone(&self) -> Self {
        match self {
            Self::Value(v) => Self::Value(*v),
        }
    }
}

impl<T: Reflect> Debug for HandlePropertyEditorMessage<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Value(v) => v.fmt(f),
        }
    }
}

impl<T: Reflect> PartialEq for HandlePropertyEditorMessage<T> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Value(left), Self::Value(right)) => left.eq(right),
        }
    }
}

impl<T: Reflect> HandlePropertyEditorMessage<T> {
    define_constructor!(HandlePropertyEditorMessage:Value => fn value(Handle<T>), layout: false);
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HandlePropertyEditorNameMessage(pub Option<String>);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HandlePropertyEditorHierarchyMessage(pub HierarchyNode);

#[derive(Visit, Reflect, TypeUuidProvider, ComponentProvider)]
#[type_uuid(id = "3ceca8c1-c365-4f03-a413-062f8f3cd685")]
#[reflect(derived_type = "UiNode")]
pub struct HandlePropertyEditor<T: Reflect> {
    widget: Widget,
    text: Handle<UiNode>,
    locate: Handle<UiNode>,
    select: Handle<UiNode>,
    make_unassigned: Handle<UiNode>,
    value: Handle<T>,
    #[visit(skip)]
    #[reflect(hidden)]
    sender: MessageSender,
    selector: Handle<UiNode>,
    pick: Handle<UiNode>,
}

impl<T: Reflect> Debug for HandlePropertyEditor<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "HandlePropertyEditor")
    }
}

impl<T: Reflect> Clone for HandlePropertyEditor<T> {
    fn clone(&self) -> Self {
        Self {
            widget: self.widget.clone(),
            text: self.text,
            value: self.value,
            sender: self.sender.clone(),
            selector: self.selector,
            locate: self.locate,
            select: self.select,
            make_unassigned: self.make_unassigned,
            pick: self.pick,
        }
    }
}

impl<T: Reflect> Deref for HandlePropertyEditor<T> {
    type Target = Widget;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<T: Reflect> DerefMut for HandlePropertyEditor<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl<T: Reflect> Control for HandlePropertyEditor<T> {
    fn draw(&self, drawing_context: &mut DrawingContext) {
        // Emit transparent geometry for the field to be able to catch mouse events without precise pointing at the
        // node name letters.
        drawing_context.push_rect_filled(&self.bounding_rect(), None);
        drawing_context.commit(
            self.clip_bounds(),
            Brush::Solid(Color::TRANSPARENT),
            CommandTexture::None,
            None,
        );
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(msg) = message.data::<HandlePropertyEditorNameMessage>() {
            let value = &msg.0;
            if message.destination() == self.handle()
                && message.direction() == MessageDirection::ToWidget
            {
                // Handle messages from the editor, it will respond to requests and provide
                // node names in efficient way.
                let value = if let Some(value) = value {
                    Some(value.as_str())
                } else if self.value.is_none() {
                    Some("Unassigned")
                } else {
                    None
                };

                if let Some(value) = value {
                    ui.send_message(TextMessage::text(
                        self.text,
                        MessageDirection::ToWidget,
                        format!("{} ({})", value, self.value),
                    ));

                    let color = if self.value.is_none() {
                        ui.style.property(Style::BRUSH_WARNING)
                    } else {
                        ui.style.property(Style::BRUSH_FOREGROUND)
                    };
                    ui.send_message(WidgetMessage::foreground(
                        self.text,
                        MessageDirection::ToWidget,
                        color,
                    ));
                } else {
                    ui.send_message(TextMessage::text(
                        self.text,
                        MessageDirection::ToWidget,
                        format!("<Invalid handle!> ({})", self.value),
                    ));

                    ui.send_message(WidgetMessage::foreground(
                        self.text,
                        MessageDirection::ToWidget,
                        ui.style.property(Style::BRUSH_ERROR),
                    ));
                };
            }
        }

        if let Some(msg) = message.data::<HandlePropertyEditorHierarchyMessage>() {
            let value = &msg.0;
            if message.destination() == self.handle()
                && message.direction() == MessageDirection::ToWidget
            {
                ui.send_message(NodeSelectorMessage::hierarchy(
                    self.selector,
                    MessageDirection::ToWidget,
                    value.clone(),
                ));

                ui.send_message(NodeSelectorMessage::selection(
                    self.selector,
                    MessageDirection::ToWidget,
                    vec![SelectedHandle {
                        handle: self.value.into(),
                        inner_type_id: TypeId::of::<T>(),
                        derived_type_ids: T::derived_types().to_vec(),
                    }],
                ));
            }
        }

        if let Some(msg) = message.data::<HandlePropertyEditorMessage<T>>() {
            if message.destination() == self.handle()
                && message.direction() == MessageDirection::ToWidget
            {
                match msg {
                    HandlePropertyEditorMessage::Value(handle) => {
                        if self.value != *handle {
                            self.value = *handle;
                            ui.send_message(message.reverse());
                        }

                        // Sync name in any case, because it may be changed.
                        request_name_sync(&self.sender, self.handle, self.value.into());
                    }
                }
            }
        } else if let Some(WidgetMessage::Drop(dropped)) = message.data() {
            if message.destination() == self.handle() {
                if let Some(item) = ui.node(*dropped).cast::<SceneItem>() {
                    ui.send_message(HandlePropertyEditorMessage::<T>::value(
                        self.handle(),
                        MessageDirection::ToWidget,
                        // TODO: Do type check here.
                        item.entity_handle.into(),
                    ))
                }
            }
        } else if let Some(ButtonMessage::Click) = message.data() {
            if message.destination == self.locate {
                self.sender.send(Message::LocateObject {
                    handle: self.value.into(),
                });
            } else if message.destination == self.select {
                self.sender.send(Message::SelectObject {
                    handle: self.value.into(),
                });
            } else if message.destination == self.make_unassigned {
                ui.send_message(HandlePropertyEditorMessage::value(
                    self.handle,
                    MessageDirection::ToWidget,
                    Handle::<T>::NONE,
                ));
            } else if message.destination == self.pick {
                let node_selector = NodeSelectorWindowBuilder::new(
                    WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(400.0))
                        .with_title(WindowTitle::text("Select a Node"))
                        .open(false),
                )
                .with_allowed_types(
                    [AllowedType {
                        id: TypeId::of::<T>(),
                        name: std::any::type_name::<T>().to_string(),
                    }]
                    .into_iter()
                    .collect(),
                )
                .build(&mut ui.build_ctx());

                ui.send_message(WindowMessage::open_modal(
                    node_selector,
                    MessageDirection::ToWidget,
                    true,
                    true,
                ));

                self.sender
                    .send(Message::ProvideSceneHierarchy { view: self.handle });

                self.selector = node_selector;
            }
        }
    }

    fn preview_message(&self, ui: &UserInterface, message: &mut UiMessage) {
        if let Some(NodeSelectorMessage::Selection(selection)) = message.data() {
            if message.destination() == self.selector
                && message.direction() == MessageDirection::FromWidget
            {
                if let Some(suitable) = selection.iter().find(|selected| {
                    selected.inner_type_id == TypeId::of::<T>()
                        || selected.derived_type_ids.contains(&TypeId::of::<T>())
                }) {
                    ui.send_message(HandlePropertyEditorMessage::<T>::value(
                        self.handle,
                        MessageDirection::ToWidget,
                        suitable.handle.into(),
                    ));
                }
            }
        } else if let Some(WindowMessage::Close) = message.data() {
            if message.destination() == self.selector {
                ui.send_message(WidgetMessage::remove(
                    self.selector,
                    MessageDirection::ToWidget,
                ));
            }
        }
    }
}

struct HandlePropertyEditorBuilder<T: Reflect> {
    widget_builder: WidgetBuilder,
    value: Handle<T>,
    sender: MessageSender,
}

fn make_icon(data: &[u8], color: Color, ctx: &mut BuildContext) -> Handle<UiNode> {
    ImageBuilder::new(
        WidgetBuilder::new()
            .with_width(16.0)
            .with_height(16.0)
            .with_margin(Thickness::uniform(1.0))
            .with_background(Brush::Solid(color).into()),
    )
    .with_opt_texture(load_image_internal(data))
    .build(ctx)
}

impl<T: Reflect> HandlePropertyEditorBuilder<T> {
    pub fn new(widget_builder: WidgetBuilder, sender: MessageSender) -> Self {
        Self {
            widget_builder,
            sender,
            value: Default::default(),
        }
    }

    pub fn with_value(mut self, value: Handle<T>) -> Self {
        self.value = value;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let text;
        let locate;
        let select;
        let make_unassigned;
        let pick;
        let grid = GridBuilder::new(
            WidgetBuilder::new()
                .with_child({
                    text = TextBuilder::new(WidgetBuilder::new().on_column(0))
                        .with_vertical_text_alignment(VerticalAlignment::Center)
                        .with_text(if self.value.is_none() {
                            "Unassigned".to_owned()
                        } else {
                            "Err: Desync!".to_owned()
                        })
                        .build(ctx);
                    text
                })
                .with_child({
                    pick = ButtonBuilder::new(
                        WidgetBuilder::new()
                            .with_tooltip(make_simple_tooltip(ctx, "Set..."))
                            .with_width(20.0)
                            .with_height(20.0)
                            .on_column(1),
                    )
                    .with_content(make_icon(
                        include_bytes!("../../../../resources/pick.png"),
                        Color::opaque(0, 180, 0),
                        ctx,
                    ))
                    .build(ctx);
                    pick
                })
                .with_child({
                    locate = ButtonBuilder::new(
                        WidgetBuilder::new()
                            .with_tooltip(make_simple_tooltip(ctx, "Locate Object"))
                            .with_width(20.0)
                            .with_height(20.0)
                            .on_column(2),
                    )
                    .with_content(make_icon(
                        include_bytes!("../../../../resources/locate.png"),
                        Color::opaque(180, 180, 180),
                        ctx,
                    ))
                    .build(ctx);
                    locate
                })
                .with_child({
                    select = ButtonBuilder::new(
                        WidgetBuilder::new()
                            .with_tooltip(make_simple_tooltip(ctx, "Select Object"))
                            .with_width(20.0)
                            .with_height(20.0)
                            .on_column(3),
                    )
                    .with_content(make_icon(
                        include_bytes!("../../../../resources/select_in_wv.png"),
                        Color::opaque(180, 180, 180),
                        ctx,
                    ))
                    .build(ctx);
                    select
                })
                .with_child({
                    make_unassigned = ButtonBuilder::new(
                        WidgetBuilder::new()
                            .with_tooltip(make_simple_tooltip(ctx, "Make Unassigned"))
                            .with_width(20.0)
                            .with_height(20.0)
                            .on_column(4),
                    )
                    .with_content(make_icon(
                        include_bytes!("../../../../resources/cross.png"),
                        Color::opaque(180, 0, 0),
                        ctx,
                    ))
                    .build(ctx);
                    make_unassigned
                }),
        )
        .add_row(Row::stretch())
        .add_column(Column::stretch())
        .add_column(Column::auto())
        .add_column(Column::auto())
        .add_column(Column::auto())
        .add_column(Column::auto())
        .build(ctx);

        let editor = HandlePropertyEditor {
            widget: self
                .widget_builder
                .with_tooltip(make_simple_tooltip(
                    ctx,
                    "Use <Alt+Mouse Drag> in World Viewer to assign the value here.",
                ))
                .with_preview_messages(true)
                .with_allow_drop(true)
                .with_child(grid)
                .build(ctx),
            text,
            value: self.value,
            sender: self.sender,
            selector: Default::default(),
            locate,
            select,
            make_unassigned,
            pick,
        };

        ctx.add_node(UiNode::new(editor))
    }
}

pub struct NodeHandlePropertyEditorDefinition<T: Reflect> {
    sender: Mutex<MessageSender>,
    #[allow(dead_code)]
    type_info: PhantomDataSendSync<T>,
}

impl<T: Reflect> Debug for NodeHandlePropertyEditorDefinition<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Node Handle")
    }
}

impl<T: Reflect> NodeHandlePropertyEditorDefinition<T> {
    pub fn new(sender: MessageSender) -> Self {
        Self {
            sender: Mutex::new(sender),
            type_info: PhantomDataSendSync::default(),
        }
    }
}

impl<T: Reflect> PropertyEditorDefinition for NodeHandlePropertyEditorDefinition<T> {
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<Handle<T>>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = ctx.property_info.cast_value::<Handle<T>>()?;

        let sender = self.sender.lock().unwrap().clone();

        let editor = HandlePropertyEditorBuilder::new(WidgetBuilder::new(), sender.clone())
            .with_value(*value)
            .build(ctx.build_context);

        request_name_sync(&sender, editor, ErasedHandle::from(*value));

        Ok(PropertyEditorInstance::Simple { editor })
    }

    fn create_message(
        &self,
        ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError> {
        let value = ctx.property_info.cast_value::<Handle<T>>()?;

        Ok(Some(HandlePropertyEditorMessage::value(
            ctx.instance,
            MessageDirection::ToWidget,
            *value,
        )))
    }

    fn translate_message(&self, ctx: PropertyEditorTranslationContext) -> Option<PropertyChanged> {
        if ctx.message.direction() == MessageDirection::FromWidget {
            if let Some(HandlePropertyEditorMessage::Value(value)) =
                ctx.message.data::<HandlePropertyEditorMessage<T>>()
            {
                return Some(PropertyChanged {
                    name: ctx.name.to_string(),
                    value: FieldKind::object(*value),
                });
            }
        }
        None
    }
}

fn request_name_sync(sender: &MessageSender, editor: Handle<UiNode>, handle: ErasedHandle) {
    // It is not possible to **effectively** provide information about node names here,
    // instead we ask the editor to provide such information in a deferred manner - by
    // sending a message.
    sender.send(Message::SyncNodeHandleName {
        view: editor,
        handle,
    });
}

#[cfg(test)]
mod test {
    use crate::plugins::inspector::editors::handle::HandlePropertyEditorBuilder;
    use fyrox::scene::node::Node;
    use fyrox::{gui::test::test_widget_deletion, gui::widget::WidgetBuilder};

    #[test]
    fn test_deletion() {
        test_widget_deletion(|ctx| {
            HandlePropertyEditorBuilder::<Node>::new(WidgetBuilder::new(), Default::default())
                .build(ctx)
        });
    }
}
