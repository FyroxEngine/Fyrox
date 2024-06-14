use crate::{
    fyrox::{
        core::{pool::Handle, reflect::prelude::*, type_traits::prelude::*, visitor::prelude::*},
        gui::{
            button::{ButtonBuilder, ButtonMessage},
            define_widget_deref,
            grid::{Column, GridBuilder, Row},
            inspector::{
                editors::{
                    PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorInstance,
                    PropertyEditorMessageContext, PropertyEditorTranslationContext,
                },
                InspectorError, PropertyChanged,
            },
            message::UiMessage,
            text::TextBuilder,
            widget::{Widget, WidgetBuilder},
            BuildContext, Control, Thickness, UiNode, UserInterface,
        },
        scene::mesh::surface::SurfaceResource,
    },
    message::MessageSender,
    Message,
};
use std::{
    any::TypeId,
    ops::{Deref, DerefMut},
};

#[derive(Clone, Visit, Reflect, Debug, ComponentProvider, TypeUuidProvider)]
#[type_uuid(id = "8461a183-4fd4-4f74-a4f4-7fd8e84bf423")]
#[allow(dead_code)]
pub struct SurfaceDataPropertyEditor {
    widget: Widget,
    view: Handle<UiNode>,
    data: SurfaceResource,
    #[visit(skip)]
    #[reflect(hidden)]
    sender: Option<MessageSender>,
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
        }
    }
}

fn surface_data_info(data: &SurfaceResource) -> String {
    let use_count = data.use_count();
    let guard = data.data_ref();
    format!(
        "Vertices: {}\nTriangles: {}\nUse Count: {}",
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

        let widget = WidgetBuilder::new()
            .with_child(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .with_child(
                            TextBuilder::new(
                                WidgetBuilder::new()
                                    .on_row(0)
                                    .on_column(0)
                                    .with_margin(Thickness::uniform(1.0)),
                            )
                            .with_text(surface_data_info(&data))
                            .build(ctx),
                        )
                        .with_child(view),
                )
                .add_column(Column::stretch())
                .add_column(Column::auto())
                .add_row(Row::auto())
                .build(ctx),
            )
            .build();

        let editor = Self {
            widget,
            data,
            view,
            sender: Some(sender),
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
            ),
        })
    }

    fn create_message(
        &self,
        _ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError> {
        Ok(None)
    }

    fn translate_message(&self, _ctx: PropertyEditorTranslationContext) -> Option<PropertyChanged> {
        None
    }
}
