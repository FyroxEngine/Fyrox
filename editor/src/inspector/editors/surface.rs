use crate::fyrox::{
    core::{
        pool::Handle, reflect::prelude::*, type_traits::prelude::*, uuid_provider,
        visitor::prelude::*,
    },
    gui::{
        define_widget_deref,
        grid::Column,
        grid::{GridBuilder, Row},
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
    scene::mesh::surface::SurfaceSharedData,
};
use std::{
    any::TypeId,
    ops::{Deref, DerefMut},
};

#[derive(Clone, Visit, Reflect, Debug, ComponentProvider)]
#[allow(dead_code)]
pub struct SurfaceDataPropertyEditor {
    widget: Widget,
    data: SurfaceSharedData,
}

define_widget_deref!(SurfaceDataPropertyEditor);

uuid_provider!(SurfaceDataPropertyEditor = "8461a183-4fd4-4f74-a4f4-7fd8e84bf423");

impl Control for SurfaceDataPropertyEditor {
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message)
    }
}

fn surface_data_info(data: &SurfaceSharedData) -> String {
    let use_count = data.use_count();
    let guard = data.lock();
    format!(
        "Vertices: {}\nTriangles: {}\nUse Count: {}",
        guard.vertex_buffer.vertex_count(),
        guard.geometry_buffer.len(),
        use_count
    )
}

impl SurfaceDataPropertyEditor {
    pub fn build(ctx: &mut BuildContext, data: SurfaceSharedData) -> Handle<UiNode> {
        let editor = Self {
            widget: WidgetBuilder::new()
                .with_child(
                    GridBuilder::new(
                        WidgetBuilder::new().with_child(
                            TextBuilder::new(
                                WidgetBuilder::new()
                                    .on_row(0)
                                    .on_column(0)
                                    .with_margin(Thickness::uniform(1.0)),
                            )
                            .with_text(surface_data_info(&data))
                            .build(ctx),
                        ),
                    )
                    .add_column(Column::stretch())
                    .add_row(Row::auto())
                    .build(ctx),
                )
                .build(),
            data,
        };

        ctx.add_node(UiNode::new(editor))
    }
}

#[derive(Debug)]
pub struct SurfaceDataPropertyEditorDefinition;

impl PropertyEditorDefinition for SurfaceDataPropertyEditorDefinition {
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<SurfaceSharedData>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = ctx.property_info.cast_value::<SurfaceSharedData>()?;

        Ok(PropertyEditorInstance::Simple {
            editor: SurfaceDataPropertyEditor::build(ctx.build_context, value.clone()),
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
