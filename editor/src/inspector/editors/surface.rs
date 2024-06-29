use crate::{
    asset::item::AssetItem,
    fyrox::{
        asset::manager::ResourceManager,
        core::{
            futures::executor::block_on, make_relative_path, pool::Handle, reflect::prelude::*,
            type_traits::prelude::*, visitor::prelude::*,
        },
        graph::BaseSceneGraph,
        gui::{
            button::{ButtonBuilder, ButtonMessage},
            define_constructor, define_widget_deref,
            grid::{Column, GridBuilder, Row},
            inspector::{
                editors::{
                    PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorInstance,
                    PropertyEditorMessageContext, PropertyEditorTranslationContext,
                },
                FieldKind, InspectorError, PropertyChanged,
            },
            message::{MessageDirection, UiMessage},
            text::{TextBuilder, TextMessage},
            widget::{Widget, WidgetBuilder, WidgetMessage},
            BuildContext, Control, Thickness, UiNode, UserInterface,
        },
        scene::mesh::surface::{SurfaceData, SurfaceResource},
    },
    inspector::EditorEnvironment,
    message::MessageSender,
    Message,
};
use std::{
    any::TypeId,
    ops::{Deref, DerefMut},
};

#[derive(Debug, PartialEq, Clone)]
pub enum SurfaceDataPropertyEditorMessage {
    Value(SurfaceResource),
}

impl SurfaceDataPropertyEditorMessage {
    define_constructor!(SurfaceDataPropertyEditorMessage:Value => fn value(SurfaceResource), layout: false);
}

#[derive(Clone, Visit, Reflect, Debug, ComponentProvider, TypeUuidProvider)]
#[type_uuid(id = "8461a183-4fd4-4f74-a4f4-7fd8e84bf423")]
#[allow(dead_code)]
pub struct SurfaceDataPropertyEditor {
    widget: Widget,
    view: Handle<UiNode>,
    data: SurfaceResource,
    text: Handle<UiNode>,
    #[visit(skip)]
    #[reflect(hidden)]
    sender: Option<MessageSender>,
    #[visit(skip)]
    #[reflect(hidden)]
    resource_manager: ResourceManager,
}

define_widget_deref!(SurfaceDataPropertyEditor);

impl Control for SurfaceDataPropertyEditor {
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(ButtonMessage::Click) = message.data() {
            if message.destination == self.view {
                if let Some(sender) = self.sender.as_ref() {
                    sender.send(Message::ViewSurfaceData(self.data.clone()));
                }
            }
        } else if let Some(WidgetMessage::Drop(dropped)) = message.data() {
            if message.destination() == self.handle() {
                if let Some(item) = ui.node(*dropped).cast::<AssetItem>() {
                    let path = if self
                        .resource_manager
                        .state()
                        .built_in_resources
                        .contains_key(&item.path)
                    {
                        Ok(item.path.clone())
                    } else {
                        make_relative_path(&item.path)
                    };

                    if let Ok(path) = path {
                        if let Ok(value) =
                            block_on(self.resource_manager.request::<SurfaceData>(path))
                        {
                            ui.send_message(SurfaceDataPropertyEditorMessage::value(
                                self.handle(),
                                MessageDirection::ToWidget,
                                value,
                            ));
                        }
                    }
                }
            }
        } else if let Some(SurfaceDataPropertyEditorMessage::Value(value)) = message.data() {
            if message.destination() == self.handle
                && message.direction() == MessageDirection::ToWidget
                && &self.data != value
            {
                self.data = value.clone();
                ui.send_message(message.reverse());

                ui.send_message(TextMessage::text(
                    self.text,
                    MessageDirection::ToWidget,
                    surface_data_info(value),
                ));
            }
        }
    }
}

fn surface_data_info(data: &SurfaceResource) -> String {
    let use_count = data.use_count();
    let kind = data.kind();
    let guard = data.data_ref();
    format!(
        "{}\nVertices: {}\nTriangles: {}\nUse Count: {}",
        kind,
        guard.vertex_buffer.vertex_count(),
        guard.geometry_buffer.len(),
        use_count
    )
}

impl SurfaceDataPropertyEditor {
    pub fn build(
        ctx: &mut BuildContext,
        data: SurfaceResource,
        sender: MessageSender,
        resource_manager: ResourceManager,
    ) -> Handle<UiNode> {
        let view = ButtonBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(1.0))
                .on_row(0)
                .on_column(1)
                .with_width(45.0)
                .with_height(22.0),
        )
        .with_text("View...")
        .build(ctx);

        let text = TextBuilder::new(
            WidgetBuilder::new()
                .on_row(0)
                .on_column(0)
                .with_margin(Thickness::uniform(1.0)),
        )
        .with_text(surface_data_info(&data))
        .build(ctx);

        let widget = WidgetBuilder::new()
            .with_child(
                GridBuilder::new(WidgetBuilder::new().with_child(text).with_child(view))
                    .add_column(Column::stretch())
                    .add_column(Column::auto())
                    .add_row(Row::auto())
                    .build(ctx),
            )
            .with_allow_drop(true)
            .build();

        let editor = Self {
            widget,
            data,
            view,
            sender: Some(sender),
            resource_manager,
            text,
        };

        ctx.add_node(UiNode::new(editor))
    }
}

#[derive(Debug)]
pub struct SurfaceDataPropertyEditorDefinition {
    pub sender: MessageSender,
}

impl PropertyEditorDefinition for SurfaceDataPropertyEditorDefinition {
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<SurfaceResource>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = ctx.property_info.cast_value::<SurfaceResource>()?;

        Ok(PropertyEditorInstance::Simple {
            editor: SurfaceDataPropertyEditor::build(
                ctx.build_context,
                value.clone(),
                self.sender.clone(),
                ctx.environment
                    .as_ref()
                    .unwrap()
                    .as_any()
                    .downcast_ref::<EditorEnvironment>()
                    .map(|e| e.resource_manager.clone())
                    .unwrap(),
            ),
        })
    }

    fn create_message(
        &self,
        ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError> {
        let value = ctx.property_info.cast_value::<SurfaceResource>()?;

        Ok(Some(SurfaceDataPropertyEditorMessage::value(
            ctx.instance,
            MessageDirection::ToWidget,
            value.clone(),
        )))
    }

    fn translate_message(&self, ctx: PropertyEditorTranslationContext) -> Option<PropertyChanged> {
        if ctx.message.direction() == MessageDirection::FromWidget {
            if let Some(SurfaceDataPropertyEditorMessage::Value(value)) = ctx.message.data() {
                return Some(PropertyChanged {
                    owner_type_id: ctx.owner_type_id,
                    name: ctx.name.to_string(),
                    value: FieldKind::object(value.clone()),
                });
            }
        }
        None
    }
}
