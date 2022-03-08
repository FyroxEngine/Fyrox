use crate::{
    core::algebra::{Vector2, Vector3, Vector4},
    inspector::{
        editors::{
            PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorInstance,
            PropertyEditorMessageContext, PropertyEditorTranslationContext,
        },
        FieldKind, InspectorError, PropertyChanged,
    },
    message::{MessageDirection, UiMessage},
    numeric::NumericType,
    vec::{
        vec2::{Vec2EditorBuilder, Vec2EditorMessage},
        vec3::{Vec3EditorBuilder, Vec3EditorMessage},
        vec4::{Vec4EditorBuilder, Vec4EditorMessage},
    },
    widget::WidgetBuilder,
    Thickness,
};
use std::{any::TypeId, marker::PhantomData};

macro_rules! define_vector_editor {
    ($name:ident, $t:ident, $bounds:tt, $builder:ty, $message:tt, $value:ty) => {
        #[derive(Debug)]
        pub struct $name<$t: $bounds> {
            pub phantom: PhantomData<T>,
        }

        impl<$t: $bounds> Default for $name<$t> {
            fn default() -> Self {
                Self {
                    phantom: PhantomData,
                }
            }
        }

        impl<$t: $bounds> PropertyEditorDefinition for $name<$t> {
            fn value_type_id(&self) -> TypeId {
                TypeId::of::<$value>()
            }

            fn create_instance(
                &self,
                ctx: PropertyEditorBuildContext,
            ) -> Result<PropertyEditorInstance, InspectorError> {
                let value = ctx.property_info.cast_value::<$value>()?;
                Ok(PropertyEditorInstance::Simple {
                    editor: <$builder>::new(
                        WidgetBuilder::new().with_margin(Thickness::uniform(1.0)),
                    )
                    .with_value(*value)
                    .build(ctx.build_context),
                })
            }

            fn create_message(
                &self,
                ctx: PropertyEditorMessageContext,
            ) -> Result<Option<UiMessage>, InspectorError> {
                let value = ctx.property_info.cast_value::<$value>()?;
                Ok(Some(<$message<$t>>::value(
                    ctx.instance,
                    MessageDirection::ToWidget,
                    *value,
                )))
            }

            fn translate_message(
                &self,
                ctx: PropertyEditorTranslationContext,
            ) -> Option<PropertyChanged> {
                if ctx.message.direction() == MessageDirection::FromWidget {
                    if let Some($message::Value(value)) = ctx.message.data::<$message<$t>>() {
                        return Some(PropertyChanged {
                            owner_type_id: ctx.owner_type_id,
                            name: ctx.name.to_string(),
                            value: FieldKind::object(*value),
                        });
                    }
                }
                None
            }
        }
    };
}

define_vector_editor!(
    Vec4PropertyEditorDefinition,
    T,
    NumericType,
    Vec4EditorBuilder::<T>,
    Vec4EditorMessage,
    Vector4::<T>
);

define_vector_editor!(
    Vec3PropertyEditorDefinition,
    T,
    NumericType,
    Vec3EditorBuilder::<T>,
    Vec3EditorMessage,
    Vector3::<T>
);

define_vector_editor!(
    Vec2PropertyEditorDefinition,
    T,
    NumericType,
    Vec2EditorBuilder::<T>,
    Vec2EditorMessage,
    Vector2::<T>
);
