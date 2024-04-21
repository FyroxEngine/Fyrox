use crate::fyrox::graph::BaseSceneGraph;
use crate::fyrox::{
    core::{
        color::Color, pool::ErasedHandle, pool::Handle, reflect::prelude::*,
        type_traits::prelude::*, uuid_provider, visitor::prelude::*,
    },
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
        text::{TextBuilder, TextMessage},
        utils::make_simple_tooltip,
        widget::{Widget, WidgetBuilder, WidgetMessage},
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, Control, Thickness,
    },
    scene::node::Node,
};
use crate::{
    load_image,
    message::MessageSender,
    scene::selector::{HierarchyNode, NodeSelectorMessage, NodeSelectorWindowBuilder},
    world::graph::item::SceneItem,
    Message, UiMessage, UiNode, UserInterface, VerticalAlignment,
};
use std::{
    any::TypeId,
    fmt::Debug,
    ops::{Deref, DerefMut},
    sync::Mutex,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HandlePropertyEditorMessage {
    Value(ErasedHandle),
    Name(Option<String>),
    Hierarchy(HierarchyNode),
}

impl HandlePropertyEditorMessage {
    define_constructor!(HandlePropertyEditorMessage:Value => fn value(ErasedHandle), layout: false);
    define_constructor!(HandlePropertyEditorMessage:Name => fn name(Option<String>), layout: false);
    define_constructor!(HandlePropertyEditorMessage:Hierarchy => fn hierarchy(HierarchyNode), layout: false);
}

#[derive(Debug, Visit, Reflect, ComponentProvider)]
pub struct HandlePropertyEditor {
    widget: Widget,
    text: Handle<UiNode>,
    locate: Handle<UiNode>,
    select: Handle<UiNode>,
    make_unassigned: Handle<UiNode>,
    value: ErasedHandle,
    #[visit(skip)]
    #[reflect(hidden)]
    sender: MessageSender,
    selector: Handle<UiNode>,
    pick: Handle<UiNode>,
}

impl Clone for HandlePropertyEditor {
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

impl Deref for HandlePropertyEditor {
    type Target = Widget;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl DerefMut for HandlePropertyEditor {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

uuid_provider!(HandlePropertyEditor = "3ceca8c1-c365-4f03-a413-062f8f3cd685");

impl Control for HandlePropertyEditor {
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

        if let Some(msg) = message.data::<HandlePropertyEditorMessage>() {
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
                        request_name_sync(&self.sender, self.handle, self.value);
                    }
                    HandlePropertyEditorMessage::Name(value) => {
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
                                Color::ORANGE
                            } else {
                                fyrox::gui::COLOR_FOREGROUND
                            };
                            ui.send_message(WidgetMessage::foreground(
                                self.text,
                                MessageDirection::ToWidget,
                                Brush::Solid(color),
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
                                Brush::Solid(Color::RED),
                            ));
                        };
                    }
                    HandlePropertyEditorMessage::Hierarchy(hierarchy) => {
                        ui.send_message(NodeSelectorMessage::hierarchy(
                            self.selector,
                            MessageDirection::ToWidget,
                            hierarchy.clone(),
                        ));

                        ui.send_message(NodeSelectorMessage::selection(
                            self.selector,
                            MessageDirection::ToWidget,
                            vec![self.value],
                        ));
                    }
                }
            }
        } else if let Some(WidgetMessage::Drop(dropped)) = message.data() {
            if message.destination() == self.handle() {
                if let Some(item) = ui.node(*dropped).cast::<SceneItem>() {
                    ui.send_message(HandlePropertyEditorMessage::value(
                        self.handle(),
                        MessageDirection::ToWidget,
                        item.entity_handle,
                    ))
                }
            }
        } else if let Some(ButtonMessage::Click) = message.data() {
            if message.destination == self.locate {
                self.sender
                    .send(Message::LocateObject { handle: self.value });
            } else if message.destination == self.select {
                self.sender
                    .send(Message::SelectObject { handle: self.value });
            } else if message.destination == self.make_unassigned {
                ui.send_message(HandlePropertyEditorMessage::value(
                    self.handle,
                    MessageDirection::ToWidget,
                    ErasedHandle::default(),
                ));
            } else if message.destination == self.pick {
                let node_selector = NodeSelectorWindowBuilder::new(
                    WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(400.0))
                        .with_title(WindowTitle::text("Select a Node"))
                        .open(false),
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
                if let Some(first) = selection.first() {
                    ui.send_message(HandlePropertyEditorMessage::value(
                        self.handle,
                        MessageDirection::ToWidget,
                        *first,
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

struct HandlePropertyEditorBuilder {
    widget_builder: WidgetBuilder,
    value: ErasedHandle,
    sender: MessageSender,
}

fn make_icon(data: &[u8], color: Color, ctx: &mut BuildContext) -> Handle<UiNode> {
    ImageBuilder::new(
        WidgetBuilder::new()
            .with_width(16.0)
            .with_height(16.0)
            .with_margin(Thickness::uniform(1.0))
            .with_background(Brush::Solid(color)),
    )
    .with_opt_texture(load_image(data))
    .build(ctx)
}

impl HandlePropertyEditorBuilder {
    pub fn new(widget_builder: WidgetBuilder, sender: MessageSender) -> Self {
        Self {
            widget_builder,
            sender,
            value: Default::default(),
        }
    }

    pub fn with_value(mut self, value: ErasedHandle) -> Self {
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
                        include_bytes!("../../../resources/pick.png"),
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
                        include_bytes!("../../../resources/locate.png"),
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
                        include_bytes!("../../../resources/select_in_wv.png"),
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
                        include_bytes!("../../../resources/cross.png"),
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
                .build(),
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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum EntityKind {
    UiNode,
    SceneNode,
}

#[derive(Debug)]
pub struct NodeHandlePropertyEditorDefinition {
    sender: Mutex<MessageSender>,
    kind: EntityKind,
}

impl NodeHandlePropertyEditorDefinition {
    pub fn new(sender: MessageSender, kind: EntityKind) -> Self {
        Self {
            sender: Mutex::new(sender),
            kind,
        }
    }

    pub fn value(&self, property_info: &FieldInfo) -> Result<ErasedHandle, InspectorError> {
        match self.kind {
            EntityKind::UiNode => Ok((*property_info.cast_value::<Handle<UiNode>>()?).into()),
            EntityKind::SceneNode => Ok((*property_info.cast_value::<Handle<Node>>()?).into()),
        }
    }
}

impl PropertyEditorDefinition for NodeHandlePropertyEditorDefinition {
    fn value_type_id(&self) -> TypeId {
        match self.kind {
            EntityKind::UiNode => TypeId::of::<Handle<UiNode>>(),
            EntityKind::SceneNode => TypeId::of::<Handle<Node>>(),
        }
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = self.value(ctx.property_info)?;

        let sender = self.sender.lock().unwrap().clone();

        let editor = HandlePropertyEditorBuilder::new(WidgetBuilder::new(), sender.clone())
            .with_value(value)
            .build(ctx.build_context);

        request_name_sync(&sender, editor, value);

        Ok(PropertyEditorInstance::Simple { editor })
    }

    fn create_message(
        &self,
        ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError> {
        let value = self.value(ctx.property_info)?;

        Ok(Some(HandlePropertyEditorMessage::value(
            ctx.instance,
            MessageDirection::ToWidget,
            value,
        )))
    }

    fn translate_message(&self, ctx: PropertyEditorTranslationContext) -> Option<PropertyChanged> {
        if ctx.message.direction() == MessageDirection::FromWidget {
            if let Some(HandlePropertyEditorMessage::Value(value)) =
                ctx.message.data::<HandlePropertyEditorMessage>()
            {
                return Some(PropertyChanged {
                    owner_type_id: ctx.owner_type_id,
                    name: ctx.name.to_string(),
                    value: match self.kind {
                        EntityKind::UiNode => FieldKind::object(Handle::<UiNode>::from(*value)),
                        EntityKind::SceneNode => FieldKind::object(Handle::<Node>::from(*value)),
                    },
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
