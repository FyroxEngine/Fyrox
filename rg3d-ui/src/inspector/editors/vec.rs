use crate::{
    core::{
        algebra::{Vector2, Vector3, Vector4},
        inspect::PropertyInfo,
        pool::Handle,
    },
    inspector::{
        editors::{Layout, PropertyEditorBuildContext, PropertyEditorDefinition},
        InspectorError,
    },
    message::{
        FieldKind, MessageDirection, PropertyChanged, UiMessage, UiMessageData, Vec2EditorMessage,
        Vec3EditorMessage, Vec4EditorMessage,
    },
    vec::{vec2::Vec2EditorBuilder, vec3::Vec3EditorBuilder, vec4::Vec4EditorBuilder},
    widget::WidgetBuilder,
    Thickness, UiNode,
};
use std::any::TypeId;

macro_rules! define_vector_editor {
    ($name:ident, $builder:ty, $message:tt, $message_variant:ident, $value:ty) => {
        #[derive(Debug)]
        pub struct $name;

        impl PropertyEditorDefinition for $name {
            fn value_type_id(&self) -> TypeId {
                TypeId::of::<$value>()
            }

            fn create_instance(
                &self,
                ctx: PropertyEditorBuildContext,
            ) -> Result<Handle<UiNode>, InspectorError> {
                let value = ctx.property_info.cast_value::<$value>()?;
                Ok(
                    <$builder>::new(WidgetBuilder::new().with_margin(Thickness::uniform(1.0)))
                        .with_value(*value)
                        .build(ctx.build_context),
                )
            }

            fn create_message(
                &self,
                instance: Handle<UiNode>,
                property_info: &PropertyInfo,
            ) -> Result<UiMessage, InspectorError> {
                let value = property_info.cast_value::<$value>()?;
                Ok($message::value(
                    instance,
                    MessageDirection::ToWidget,
                    *value,
                ))
            }

            fn translate_message(
                &self,
                name: &str,
                owner_type_id: TypeId,
                message: &UiMessage,
            ) -> Option<PropertyChanged> {
                if message.direction() == MessageDirection::FromWidget {
                    if let UiMessageData::$message_variant($message::Value(value)) = message.data()
                    {
                        return Some(PropertyChanged {
                            owner_type_id,
                            name: name.to_string(),
                            value: FieldKind::object(*value),
                        });
                    }
                }
                None
            }

            fn layout(&self) -> Layout {
                Layout::Horizontal
            }
        }
    };
}

define_vector_editor!(
    Vec4PropertyEditorDefinition,
    Vec4EditorBuilder,
    Vec4EditorMessage,
    Vec4Editor,
    Vector4<f32>
);

define_vector_editor!(
    Vec3PropertyEditorDefinition,
    Vec3EditorBuilder,
    Vec3EditorMessage,
    Vec3Editor,
    Vector3<f32>
);

define_vector_editor!(
    Vec2PropertyEditorDefinition,
    Vec2EditorBuilder,
    Vec2EditorMessage,
    Vec2Editor,
    Vector2<f32>
);
