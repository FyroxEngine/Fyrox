use crate::fyrox::{
    gui::{
        inspector::{
            editors::{
                PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorInstance,
                PropertyEditorMessageContext, PropertyEditorTranslationContext,
            },
            InspectorError, PropertyChanged,
        },
        message::{MessageDirection, UiMessage},
        text::{TextBuilder, TextMessage},
        widget::WidgetBuilder,
    },
    scene::tilemap::Tiles,
};
use std::any::TypeId;

#[derive(Debug)]
pub struct TilesPropertyEditorDefinition;

fn tiles_info(tiles: &Tiles) -> String {
    format!("Tile Count: {}", tiles.len())
}

impl PropertyEditorDefinition for TilesPropertyEditorDefinition {
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<Tiles>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let tiles = ctx.property_info.cast_value::<Tiles>()?;

        Ok(PropertyEditorInstance::Simple {
            editor: TextBuilder::new(WidgetBuilder::new())
                .with_text(tiles_info(tiles))
                .build(ctx.build_context),
        })
    }

    fn create_message(
        &self,
        ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError> {
        let tiles = ctx.property_info.cast_value::<Tiles>()?;

        Ok(Some(TextMessage::text(
            ctx.instance,
            MessageDirection::ToWidget,
            tiles_info(tiles),
        )))
    }

    fn translate_message(&self, _ctx: PropertyEditorTranslationContext) -> Option<PropertyChanged> {
        None
    }
}
